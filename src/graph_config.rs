use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

#[derive(Debug, Clone, Deserialize)]
pub struct ComputeGraphConfig {
    #[serde(default)]
    pub images: Vec<ImageResourceConfig>,
    #[serde(default)]
    pub buffers: Vec<BufferResourceConfig>,
    #[serde(rename = "compute_nodes", default)]
    pub compute_nodes: Vec<ComputeNodeConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ImageResourceConfig {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BufferResourceConfig {
    pub name: String,
    pub size_in_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ComputeNodeConfig {
    pub name: String,
    pub shader: String,
    pub dispatch: [u32; 3],
    #[serde(default)]
    pub bindings: Vec<ResourceBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ResourceBinding {
    pub resource: String,
    pub slot: u32,
    pub access: ResourceAccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFormat {
    Rgba8Unorm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceAccess {
    Read,
    Write,
    ReadWrite,
}

impl ResourceAccess {
    fn required_state(self) -> PlannedResourceState {
        match self {
            Self::Read => PlannedResourceState::ShaderRead,
            Self::Write | Self::ReadWrite => PlannedResourceState::UnorderedAccess,
        }
    }

    fn is_write(self) -> bool {
        matches!(self, Self::Write | Self::ReadWrite)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlannedResourceState {
    ShaderRead,
    UnorderedAccess,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlannedBarrier {
    Transition {
        resource: String,
        before: PlannedResourceState,
        after: PlannedResourceState,
    },
    Uav {
        resource: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceDefinition {
    Image(ImageResourceConfig),
    Buffer(BufferResourceConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedComputeNode {
    pub name: String,
    pub shader: String,
    pub dispatch: [u32; 3],
    pub bindings: Vec<ResourceBinding>,
    pub dependencies: Vec<String>,
    pub barriers_before: Vec<PlannedBarrier>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPlan {
    pub resources: BTreeMap<String, ResourceDefinition>,
    pub nodes: Vec<PlannedComputeNode>,
    pub initial_resource_states: BTreeMap<String, PlannedResourceState>,
    pub final_resource_states: BTreeMap<String, PlannedResourceState>,
}

impl ExecutionPlan {
    pub fn from_toml(source: &str) -> Result<Self, GraphConfigError> {
        ComputeGraphConfig::from_toml(source)?.build_execution_plan()
    }

    pub fn descriptor_count(&self) -> u32 {
        self.nodes
            .iter()
            .flat_map(|node| node.bindings.iter().map(|binding| binding.slot))
            .max()
            .map_or(0, |max_slot| max_slot + 1)
    }
}

impl ComputeGraphConfig {
    pub fn from_toml(source: &str) -> Result<Self, GraphConfigError> {
        let config = toml::from_str::<Self>(source).map_err(GraphConfigError::ParseToml)?;
        config.validate()?;
        Ok(config)
    }

    pub fn build_execution_plan(&self) -> Result<ExecutionPlan, GraphConfigError> {
        let resources = self.collect_resources()?;
        let mut trackers = resources
            .keys()
            .cloned()
            .map(|name| (name, ResourceTracker::default()))
            .collect::<BTreeMap<_, _>>();
        let mut initial_resource_states = BTreeMap::new();
        let mut final_resource_states = BTreeMap::new();
        let mut nodes = Vec::with_capacity(self.compute_nodes.len());

        for (node_index, node) in self.compute_nodes.iter().enumerate() {
            let mut dependency_indices = BTreeSet::new();
            let mut barriers_before = Vec::new();

            for binding in &node.bindings {
                let tracker = trackers.get_mut(&binding.resource).ok_or_else(|| {
                    GraphConfigError::UnknownResource {
                        node: node.name.clone(),
                        resource: binding.resource.clone(),
                    }
                })?;
                let required_state = binding.access.required_state();

                if binding.access.is_write() {
                    if let Some(last_writer) = tracker.last_writer {
                        dependency_indices.insert(last_writer);
                    }
                    dependency_indices.extend(tracker.active_readers.iter().copied());
                } else if let Some(last_writer) = tracker.last_writer {
                    dependency_indices.insert(last_writer);
                }

                match tracker.current_state {
                    Some(current_state) if current_state != required_state => {
                        barriers_before.push(PlannedBarrier::Transition {
                            resource: binding.resource.clone(),
                            before: current_state,
                            after: required_state,
                        });
                    }
                    Some(PlannedResourceState::UnorderedAccess)
                        if required_state == PlannedResourceState::UnorderedAccess =>
                    {
                        barriers_before.push(PlannedBarrier::Uav {
                            resource: binding.resource.clone(),
                        });
                    }
                    _ => {}
                }

                initial_resource_states
                    .entry(binding.resource.clone())
                    .or_insert(required_state);
                final_resource_states.insert(binding.resource.clone(), required_state);
                tracker.current_state = Some(required_state);

                if binding.access.is_write() {
                    tracker.active_readers.clear();
                    tracker.last_writer = Some(node_index);
                } else {
                    tracker.active_readers.insert(node_index);
                }
            }

            let dependencies = dependency_indices
                .into_iter()
                .map(|index| self.compute_nodes[index].name.clone())
                .collect();

            nodes.push(PlannedComputeNode {
                name: node.name.clone(),
                shader: node.shader.clone(),
                dispatch: node.dispatch,
                bindings: node.bindings.clone(),
                dependencies,
                barriers_before,
            });
        }

        Ok(ExecutionPlan {
            resources,
            nodes,
            initial_resource_states,
            final_resource_states,
        })
    }

    fn validate(&self) -> Result<(), GraphConfigError> {
        let _ = self.collect_resources()?;
        let mut node_names = BTreeSet::new();

        for node in &self.compute_nodes {
            if !node_names.insert(node.name.clone()) {
                return Err(GraphConfigError::DuplicateNode(node.name.clone()));
            }

            let mut bound_resources = BTreeSet::new();
            let mut bound_slots = BTreeSet::new();
            for binding in &node.bindings {
                if !bound_resources.insert(binding.resource.clone()) {
                    return Err(GraphConfigError::DuplicateNodeResourceBinding {
                        node: node.name.clone(),
                        resource: binding.resource.clone(),
                    });
                }
                if !bound_slots.insert(binding.slot) {
                    return Err(GraphConfigError::DuplicateNodeBindingSlot {
                        node: node.name.clone(),
                        slot: binding.slot,
                    });
                }
            }
        }

        Ok(())
    }

    fn collect_resources(&self) -> Result<BTreeMap<String, ResourceDefinition>, GraphConfigError> {
        let mut resources = BTreeMap::new();

        for image in &self.images {
            if resources
                .insert(image.name.clone(), ResourceDefinition::Image(image.clone()))
                .is_some()
            {
                return Err(GraphConfigError::DuplicateResource(image.name.clone()));
            }
        }

        for buffer in &self.buffers {
            if resources
                .insert(buffer.name.clone(), ResourceDefinition::Buffer(buffer.clone()))
                .is_some()
            {
                return Err(GraphConfigError::DuplicateResource(buffer.name.clone()));
            }
        }

        Ok(resources)
    }
}

#[derive(Debug, Default)]
struct ResourceTracker {
    current_state: Option<PlannedResourceState>,
    active_readers: BTreeSet<usize>,
    last_writer: Option<usize>,
}

#[derive(Debug)]
pub enum GraphConfigError {
    ParseToml(toml::de::Error),
    DuplicateResource(String),
    DuplicateNode(String),
    DuplicateNodeResourceBinding { node: String, resource: String },
    DuplicateNodeBindingSlot { node: String, slot: u32 },
    UnknownResource { node: String, resource: String },
}

impl fmt::Display for GraphConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseToml(error) => write!(f, "failed to parse compute graph TOML: {error}"),
            Self::DuplicateResource(resource) => {
                write!(f, "resource '{resource}' is defined more than once")
            }
            Self::DuplicateNode(node) => write!(f, "compute node '{node}' is defined more than once"),
            Self::DuplicateNodeResourceBinding { node, resource } => write!(
                f,
                "compute node '{node}' binds resource '{resource}' more than once"
            ),
            Self::DuplicateNodeBindingSlot { node, slot } => {
                write!(f, "compute node '{node}' binds descriptor slot {slot} more than once")
            }
            Self::UnknownResource { node, resource } => {
                write!(f, "compute node '{node}' references unknown resource '{resource}'")
            }
        }
    }
}

impl Error for GraphConfigError {}

#[cfg(test)]
mod tests {
    use super::{
        ExecutionPlan, PlannedBarrier, PlannedResourceState, ResourceAccess, ResourceDefinition,
    };

    #[test]
    fn infers_dependencies_and_barriers_from_resource_usage() {
        let plan = ExecutionPlan::from_toml(
            r#"
                [[images]]
                name = "checkerboard"
                width = 8
                height = 8
                format = "rgba8_unorm"

                [[buffers]]
                name = "scratch"
                size_in_bytes = 64

                [[compute_nodes]]
                name = "writer"
                shader = "checkerboard_compute"
                dispatch = [1, 1, 1]

                [[compute_nodes.bindings]]
                resource = "checkerboard"
                slot = 0
                access = "write"

                [[compute_nodes]]
                name = "reader"
                shader = "simple_compute"
                dispatch = [1, 1, 1]

                [[compute_nodes.bindings]]
                resource = "checkerboard"
                slot = 0
                access = "read"
            "#,
        )
        .expect("graph plan should parse");

        assert_eq!(plan.descriptor_count(), 1);
        assert!(matches!(
            plan.resources.get("checkerboard"),
            Some(ResourceDefinition::Image(_))
        ));
        assert!(matches!(
            plan.resources.get("scratch"),
            Some(ResourceDefinition::Buffer(_))
        ));
        assert_eq!(plan.nodes[0].dependencies, Vec::<String>::new());
        assert_eq!(plan.nodes[0].bindings[0].access, ResourceAccess::Write);
        assert_eq!(plan.nodes[1].dependencies, vec!["writer".to_string()]);
        assert_eq!(
            plan.nodes[1].barriers_before,
            vec![PlannedBarrier::Transition {
                resource: "checkerboard".to_string(),
                before: PlannedResourceState::UnorderedAccess,
                after: PlannedResourceState::ShaderRead,
            }]
        );
    }
}
