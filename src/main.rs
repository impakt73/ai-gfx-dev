mod dxc_locator;
mod graph_config;

#[cfg(target_os = "windows")]
#[link(name = "Advapi32")]
unsafe extern "system" {}

#[cfg(target_os = "windows")]
mod dx12 {
    use crate::{
        dxc_locator::find_dxc_library,
        graph_config::{
            ExecutionPlan, ImageFormat, PlannedBarrier, PlannedResourceState, ResourceDefinition,
        },
    };
    use hassle_rs::Dxc;
    use std::{
        collections::BTreeMap,
        fmt,
        mem::{ManuallyDrop, size_of},
        ptr, slice,
    };
    use windows::{
        Win32::{
            Foundation::{CloseHandle, E_FAIL},
            Graphics::{
                Direct3D::{
                    D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1,
                    D3D_FEATURE_LEVEL_12_0, D3D_FEATURE_LEVEL_12_1, D3D_FEATURE_LEVEL_12_2,
                    ID3DBlob,
                },
                Direct3D12::{
                    D3D_HIGHEST_SHADER_MODEL, D3D_ROOT_SIGNATURE_VERSION_1, D3D_SHADER_MODEL,
                    D3D_SHADER_MODEL_5_1, D3D_SHADER_MODEL_6_0, D3D_SHADER_MODEL_6_1,
                    D3D_SHADER_MODEL_6_2, D3D_SHADER_MODEL_6_3, D3D_SHADER_MODEL_6_4,
                    D3D_SHADER_MODEL_6_5, D3D_SHADER_MODEL_6_6, D3D_SHADER_MODEL_6_7,
                    D3D_SHADER_MODEL_6_8, D3D_SHADER_MODEL_6_9, D3D12_BUFFER_SRV,
                    D3D12_BUFFER_SRV_FLAG_NONE, D3D12_BUFFER_UAV, D3D12_BUFFER_UAV_FLAG_NONE,
                    D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC,
                    D3D12_COMPUTE_PIPELINE_STATE_DESC, D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
                    D3D12_DESCRIPTOR_HEAP_DESC, D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
                    D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV, D3D12_FEATURE,
                    D3D12_FEATURE_ARCHITECTURE1, D3D12_FEATURE_D3D12_OPTIONS,
                    D3D12_FEATURE_D3D12_OPTIONS1, D3D12_FEATURE_D3D12_OPTIONS5,
                    D3D12_FEATURE_D3D12_OPTIONS7, D3D12_FEATURE_DATA_ARCHITECTURE1,
                    D3D12_FEATURE_DATA_D3D12_OPTIONS, D3D12_FEATURE_DATA_D3D12_OPTIONS1,
                    D3D12_FEATURE_DATA_D3D12_OPTIONS5, D3D12_FEATURE_DATA_D3D12_OPTIONS7,
                    D3D12_FEATURE_DATA_SHADER_MODEL, D3D12_FEATURE_SHADER_MODEL,
                    D3D12_FENCE_FLAG_NONE, D3D12_HEAP_FLAG_NONE, D3D12_HEAP_PROPERTIES,
                    D3D12_HEAP_TYPE, D3D12_HEAP_TYPE_DEFAULT, D3D12_HEAP_TYPE_READBACK,
                    D3D12_MESH_SHADER_TIER, D3D12_MESH_SHADER_TIER_1,
                    D3D12_MESH_SHADER_TIER_NOT_SUPPORTED, D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
                    D3D12_RAYTRACING_TIER, D3D12_RAYTRACING_TIER_1_0, D3D12_RAYTRACING_TIER_1_1,
                    D3D12_RAYTRACING_TIER_NOT_SUPPORTED, D3D12_RESOURCE_BARRIER,
                    D3D12_RESOURCE_BARRIER_0, D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                    D3D12_RESOURCE_BARRIER_FLAG_NONE, D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                    D3D12_RESOURCE_BARRIER_TYPE_UAV, D3D12_RESOURCE_DESC,
                    D3D12_RESOURCE_DIMENSION_BUFFER, D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                    D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_COPY_DEST,
                    D3D12_RESOURCE_STATE_COPY_SOURCE,
                    D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE,
                    D3D12_RESOURCE_STATE_UNORDERED_ACCESS, D3D12_RESOURCE_STATES,
                    D3D12_RESOURCE_TRANSITION_BARRIER, D3D12_RESOURCE_UAV_BARRIER,
                    D3D12_ROOT_SIGNATURE_DESC,
                    D3D12_ROOT_SIGNATURE_FLAG_CBV_SRV_UAV_HEAP_DIRECTLY_INDEXED,
                    D3D12_ROOT_SIGNATURE_FLAG_SAMPLER_HEAP_DIRECTLY_INDEXED, D3D12_SHADER_BYTECODE,
                    D3D12_SHADER_RESOURCE_VIEW_DESC, D3D12_SRV_DIMENSION_BUFFER,
                    D3D12_SRV_DIMENSION_TEXTURE2D, D3D12_TEX2D_SRV, D3D12_TEX2D_UAV,
                    D3D12_TEXTURE_COPY_LOCATION, D3D12_TEXTURE_COPY_LOCATION_0,
                    D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                    D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX, D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                    D3D12_TEXTURE_LAYOUT_UNKNOWN, D3D12_UAV_DIMENSION_BUFFER,
                    D3D12_UAV_DIMENSION_TEXTURE2D, D3D12_UNORDERED_ACCESS_VIEW_DESC,
                    D3D12CreateDevice, D3D12SerializeRootSignature, ID3D12CommandAllocator,
                    ID3D12CommandList, ID3D12CommandQueue, ID3D12DescriptorHeap, ID3D12Device,
                    ID3D12Fence, ID3D12GraphicsCommandList, ID3D12PipelineState, ID3D12Resource,
                    ID3D12RootSignature,
                },
                Dxgi::{
                    Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC},
                    CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, IDXGIAdapter4, IDXGIFactory4,
                },
            },
            System::Threading::{CreateEventA, INFINITE, WaitForSingleObject},
        },
        core::{Error, Interface, Result},
    };

    const COMPUTE_SHADER_PROFILE: &str = "cs_6_6";
    const COMPUTE_SHADER_SOURCE_NAME: &str = "shaders/simple_compute.hlsl";
    const COMPUTE_SHADER_SOURCE: &str = include_str!("../shaders/simple_compute.hlsl");
    const BUILD_TIME_COMPUTE_SHADER_DXIL: &[u8] =
        include_bytes!(concat!(env!("OUT_DIR"), "/build_time_compute_shader.dxil"));
    const CHECKERBOARD_COMPUTE_SHADER_SOURCE_NAME: &str = "shaders/checkerboard_compute.hlsl";
    const CHECKERBOARD_COMPUTE_SHADER_SOURCE: &str =
        include_str!("../shaders/checkerboard_compute.hlsl");
    const BUILD_TIME_CHECKERBOARD_COMPUTE_SHADER_DXIL: &[u8] = include_bytes!(concat!(
        env!("OUT_DIR"),
        "/build_time_checkerboard_compute_shader.dxil"
    ));
    const CHECKERBOARD_PIXEL_SIZE: usize = 4;
    const STRUCTURED_BUFFER_STRIDE: u32 = 4;

    struct GpuResource {
        resource: ID3D12Resource,
        definition: ResourceDefinition,
    }

    struct ExecutedComputeGraph {
        device: ID3D12Device,
        plan: ExecutionPlan,
        resources: BTreeMap<String, GpuResource>,
    }

    struct WarpDevice {
        adapter_name: String,
        feature_level: D3D_FEATURE_LEVEL,
        device: ID3D12Device,
    }

    #[derive(Debug)]
    pub struct WarpDeviceCapabilities {
        pub adapter_name: String,
        pub feature_level: String,
        pub resource_binding_tier: String,
        pub tiled_resources_tier: String,
        pub conservative_rasterization_tier: String,
        pub resource_heap_tier: String,
        pub raytracing_tier: String,
        pub mesh_shader_tier: String,
        pub highest_shader_model: String,
        pub wave_ops_supported: bool,
        pub wave_lane_count_min: u32,
        pub wave_lane_count_max: u32,
        pub total_lane_count: u32,
        pub int64_shader_ops_supported: bool,
        pub uma: bool,
        pub cache_coherent_uma: bool,
    }

    impl fmt::Display for WarpDeviceCapabilities {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            writeln!(f, "DX12 WARP device capabilities")?;
            writeln!(f, "Adapter: {}", self.adapter_name)?;
            writeln!(f, "Feature level: {}", self.feature_level)?;
            writeln!(f, "Resource binding tier: {}", self.resource_binding_tier)?;
            writeln!(f, "Tiled resources tier: {}", self.tiled_resources_tier)?;
            writeln!(
                f,
                "Conservative rasterization tier: {}",
                self.conservative_rasterization_tier
            )?;
            writeln!(f, "Resource heap tier: {}", self.resource_heap_tier)?;
            writeln!(f, "Ray tracing tier: {}", self.raytracing_tier)?;
            writeln!(f, "Mesh shader tier: {}", self.mesh_shader_tier)?;
            writeln!(f, "Highest shader model: {}", self.highest_shader_model)?;
            writeln!(f, "Wave ops supported: {}", self.wave_ops_supported)?;
            writeln!(f, "Wave lane count min: {}", self.wave_lane_count_min)?;
            writeln!(f, "Wave lane count max: {}", self.wave_lane_count_max)?;
            writeln!(f, "Total wave lane count: {}", self.total_lane_count)?;
            writeln!(
                f,
                "Int64 shader ops supported: {}",
                self.int64_shader_ops_supported
            )?;
            writeln!(f, "UMA: {}", self.uma)?;
            write!(f, "Cache coherent UMA: {}", self.cache_coherent_uma)
        }
    }

    pub fn create_warp_device_capabilities() -> Result<WarpDeviceCapabilities> {
        let warp_device = create_warp_device()?;
        let device = warp_device.device;

        unsafe {
            let mut options = D3D12_FEATURE_DATA_D3D12_OPTIONS::default();
            device.CheckFeatureSupport(
                D3D12_FEATURE_D3D12_OPTIONS,
                &mut options as *mut _ as _,
                size_of::<D3D12_FEATURE_DATA_D3D12_OPTIONS>() as u32,
            )?;

            let mut options1 = D3D12_FEATURE_DATA_D3D12_OPTIONS1::default();
            device.CheckFeatureSupport(
                D3D12_FEATURE_D3D12_OPTIONS1,
                &mut options1 as *mut _ as _,
                size_of::<D3D12_FEATURE_DATA_D3D12_OPTIONS1>() as u32,
            )?;

            let options5 = try_check_feature_support::<D3D12_FEATURE_DATA_D3D12_OPTIONS5>(
                &device,
                D3D12_FEATURE_D3D12_OPTIONS5,
            )
            .unwrap_or_default();

            let options7 = try_check_feature_support::<D3D12_FEATURE_DATA_D3D12_OPTIONS7>(
                &device,
                D3D12_FEATURE_D3D12_OPTIONS7,
            )
            .unwrap_or_default();

            let mut architecture = D3D12_FEATURE_DATA_ARCHITECTURE1::default();
            architecture.NodeIndex = 0;
            device.CheckFeatureSupport(
                D3D12_FEATURE_ARCHITECTURE1,
                &mut architecture as *mut _ as _,
                size_of::<D3D12_FEATURE_DATA_ARCHITECTURE1>() as u32,
            )?;

            let highest_shader_model =
                try_highest_shader_model(&device).unwrap_or(D3D_SHADER_MODEL_5_1);

            Ok(WarpDeviceCapabilities {
                adapter_name: warp_device.adapter_name,
                feature_level: format_feature_level(warp_device.feature_level).to_string(),
                resource_binding_tier: format!("{:?}", options.ResourceBindingTier),
                tiled_resources_tier: format!("{:?}", options.TiledResourcesTier),
                conservative_rasterization_tier: format!(
                    "{:?}",
                    options.ConservativeRasterizationTier
                ),
                resource_heap_tier: format!("{:?}", options.ResourceHeapTier),
                raytracing_tier: format_raytracing_tier(options5.RaytracingTier).to_string(),
                mesh_shader_tier: format_mesh_shader_tier(options7.MeshShaderTier).to_string(),
                highest_shader_model: format_shader_model(highest_shader_model).to_string(),
                wave_ops_supported: options1.WaveOps.as_bool(),
                wave_lane_count_min: options1.WaveLaneCountMin,
                wave_lane_count_max: options1.WaveLaneCountMax,
                total_lane_count: options1.TotalLaneCount,
                int64_shader_ops_supported: options1.Int64ShaderOps.as_bool(),
                uma: architecture.UMA.as_bool(),
                cache_coherent_uma: architecture.CacheCoherentUMA.as_bool(),
            })
        }
    }

    pub fn build_time_compute_shader() -> &'static [u8] {
        BUILD_TIME_COMPUTE_SHADER_DXIL
    }

    pub fn build_time_checkerboard_compute_shader() -> &'static [u8] {
        BUILD_TIME_CHECKERBOARD_COMPUTE_SHADER_DXIL
    }

    pub fn plan_compute_graph(config_toml: &str) -> Result<ExecutionPlan> {
        ExecutionPlan::from_toml(config_toml).map_err(|error| Error::new(E_FAIL, error.to_string()))
    }

    pub fn compile_runtime_compute_shader() -> Result<Vec<u8>> {
        compile_compute_shader(COMPUTE_SHADER_SOURCE, COMPUTE_SHADER_SOURCE_NAME)
    }

    pub fn compile_runtime_checkerboard_compute_shader() -> Result<Vec<u8>> {
        compile_compute_shader(
            CHECKERBOARD_COMPUTE_SHADER_SOURCE,
            CHECKERBOARD_COMPUTE_SHADER_SOURCE_NAME,
        )
    }

    pub fn create_compute_pipeline_state(compiled_shader: &[u8]) -> Result<ID3D12PipelineState> {
        let device = create_warp_device()?.device;
        create_compute_pipeline_state_for_device(&device, compiled_shader)
    }

    pub fn dispatch_configured_compute_graph(
        config_toml: &str,
        output_image: &str,
    ) -> Result<(ExecutionPlan, Vec<u8>)> {
        let plan = plan_compute_graph(config_toml)?;
        let executed_graph = execute_compute_graph(plan)?;
        let output_resource = executed_graph.resources.get(output_image).ok_or_else(|| {
            Error::new(
                E_FAIL,
                format!("configured compute graph does not define image resource '{output_image}'"),
            )
        })?;
        let texture = match &output_resource.definition {
            ResourceDefinition::Image(_) => &output_resource.resource,
            ResourceDefinition::Buffer(_) => {
                return Err(Error::new(
                    E_FAIL,
                    format!(
                        "resource '{output_image}' is a buffer and cannot be read back as an image"
                    ),
                ));
            }
        };
        let image_desc = unsafe { texture.GetDesc() };
        let mut placed_footprint = D3D12_PLACED_SUBRESOURCE_FOOTPRINT::default();
        let mut num_rows = 0;
        let mut row_size_in_bytes = 0;
        let mut total_bytes = 0;
        let image_dimensions = match &output_resource.definition {
            ResourceDefinition::Image(image) => (image.width as usize, image.height as usize),
            ResourceDefinition::Buffer(_) => unreachable!(),
        };

        let device = executed_graph.device.clone();
        unsafe {
            device.GetCopyableFootprints(
                &image_desc,
                0,
                1,
                0,
                Some(&mut placed_footprint),
                Some(&mut num_rows),
                Some(&mut row_size_in_bytes),
                Some(&mut total_bytes),
            );
        }

        let readback_buffer = create_readback_buffer(&device, total_bytes)?;
        let command_queue = create_command_queue(&device)?;
        let command_allocator = create_command_allocator(&device)?;
        let command_list = create_command_list(&device, &command_allocator)?;
        let fence = create_fence(&device)?;

        let final_state = *executed_graph
            .plan
            .final_resource_states
            .get(output_image)
            .ok_or_else(|| {
                Error::new(
                    E_FAIL,
                    format!("configured compute graph never uses output image '{output_image}'"),
                )
            })?;

        unsafe {
            let mut barriers = [planned_transition_resource_barrier(
                texture,
                final_state,
                D3D12_RESOURCE_STATE_COPY_SOURCE,
            )];
            submit_resource_barriers(&command_list, &mut barriers);

            let mut destination = placed_texture_copy_location(&readback_buffer, placed_footprint);
            let mut source = subresource_texture_copy_location(texture, 0);
            command_list.CopyTextureRegion(&destination, 0, 0, 0, &source, None);
            ManuallyDrop::drop(&mut destination.pResource);
            ManuallyDrop::drop(&mut source.pResource);

            command_list.Close()?;

            let command_lists = [Some(command_list.cast::<ID3D12CommandList>()?)];
            command_queue.ExecuteCommandLists(&command_lists);
            command_queue.Signal(&fence, 1)?;
        }

        wait_for_fence(&fence, 1)?;
        let pixels = readback_texture(
            &readback_buffer,
            &placed_footprint,
            image_dimensions.0,
            image_dimensions.1,
        )?;
        Ok((executed_graph.plan, pixels))
    }

    fn execute_compute_graph(plan: ExecutionPlan) -> Result<ExecutedComputeGraph> {
        let device = create_warp_device()?.device;
        let root_signature = create_dynamic_resource_root_signature(&device)?;
        let descriptor_heap =
            create_shader_visible_uav_heap(&device, plan.descriptor_count().max(1))?;
        let resources = create_graph_resources(&device, &plan)?;
        let pipeline_states = create_compute_pipeline_states(&device, &plan)?;
        let command_queue = create_command_queue(&device)?;
        let command_allocator = create_command_allocator(&device)?;
        let command_list = create_command_list(&device, &command_allocator)?;
        let fence = create_fence(&device)?;
        let descriptor_heaps = [Some(descriptor_heap.clone())];

        unsafe {
            command_list.SetDescriptorHeaps(&descriptor_heaps);
            command_list.SetComputeRootSignature(&root_signature);

            for node in &plan.nodes {
                update_compute_node_descriptors(&device, &descriptor_heap, &resources, node)?;

                let mut barriers = node
                    .barriers_before
                    .iter()
                    .map(|barrier| planned_barrier_to_dx12(barrier, &resources))
                    .collect::<Result<Vec<_>>>()?;
                submit_resource_barriers(&command_list, &mut barriers);

                let pipeline_state = pipeline_states.get(&node.shader).ok_or_else(|| {
                    Error::new(
                        E_FAIL,
                        format!(
                            "missing pipeline state for compute shader '{}'",
                            node.shader
                        ),
                    )
                })?;
                command_list.SetPipelineState(pipeline_state);
                command_list.Dispatch(node.dispatch[0], node.dispatch[1], node.dispatch[2]);
            }

            command_list.Close()?;

            let command_lists = [Some(command_list.cast::<ID3D12CommandList>()?)];
            command_queue.ExecuteCommandLists(&command_lists);
            command_queue.Signal(&fence, 1)?;
        }

        wait_for_fence(&fence, 1)?;
        Ok(ExecutedComputeGraph {
            device,
            plan,
            resources,
        })
    }

    fn create_compute_pipeline_states(
        device: &ID3D12Device,
        plan: &ExecutionPlan,
    ) -> Result<BTreeMap<String, ID3D12PipelineState>> {
        let mut pipeline_states = BTreeMap::new();

        for node in &plan.nodes {
            if pipeline_states.contains_key(&node.shader) {
                continue;
            }

            let shader = embedded_compute_shader(&node.shader).ok_or_else(|| {
                Error::new(
                    E_FAIL,
                    format!("unknown embedded compute shader '{}'", node.shader),
                )
            })?;
            let pipeline_state = create_compute_pipeline_state_for_device(device, shader)?;
            pipeline_states.insert(node.shader.clone(), pipeline_state);
        }

        Ok(pipeline_states)
    }

    fn create_graph_resources(
        device: &ID3D12Device,
        plan: &ExecutionPlan,
    ) -> Result<BTreeMap<String, GpuResource>> {
        plan.resources
            .iter()
            .map(|(name, definition)| {
                let initial_state = plan
                    .initial_resource_states
                    .get(name)
                    .copied()
                    .unwrap_or(PlannedResourceState::UnorderedAccess);
                let resource = create_graph_resource(device, definition, initial_state)?;
                Ok((
                    name.clone(),
                    GpuResource {
                        resource,
                        definition: definition.clone(),
                    },
                ))
            })
            .collect()
    }

    fn create_graph_resource(
        device: &ID3D12Device,
        definition: &ResourceDefinition,
        initial_state: PlannedResourceState,
    ) -> Result<ID3D12Resource> {
        match definition {
            ResourceDefinition::Image(image) => {
                let texture_desc = D3D12_RESOURCE_DESC {
                    Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                    Width: image.width as u64,
                    Height: image.height,
                    DepthOrArraySize: 1,
                    MipLevels: 1,
                    Format: dxgi_format_for_image(image.format),
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
                    Flags: D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS,
                    ..Default::default()
                };

                let mut texture = None;
                unsafe {
                    device.CreateCommittedResource(
                        &default_heap_properties(D3D12_HEAP_TYPE_DEFAULT),
                        D3D12_HEAP_FLAG_NONE,
                        &texture_desc,
                        planned_state_to_dx12(initial_state),
                        None,
                        &mut texture,
                    )?;
                }
                Ok(texture.expect("CreateCommittedResource returned no texture"))
            }
            ResourceDefinition::Buffer(buffer) => {
                if buffer.size_in_bytes == 0 {
                    return Err(Error::new(
                        E_FAIL,
                        format!("buffer '{}' must be larger than zero bytes", buffer.name),
                    ));
                }

                let buffer_desc = D3D12_RESOURCE_DESC {
                    Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
                    Width: buffer.size_in_bytes,
                    Height: 1,
                    DepthOrArraySize: 1,
                    MipLevels: 1,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                    Flags: D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS,
                    ..Default::default()
                };

                let mut resource = None;
                unsafe {
                    device.CreateCommittedResource(
                        &default_heap_properties(D3D12_HEAP_TYPE_DEFAULT),
                        D3D12_HEAP_FLAG_NONE,
                        &buffer_desc,
                        planned_state_to_dx12(initial_state),
                        None,
                        &mut resource,
                    )?;
                }
                Ok(resource.expect("CreateCommittedResource returned no buffer"))
            }
        }
    }

    fn update_compute_node_descriptors(
        device: &ID3D12Device,
        descriptor_heap: &ID3D12DescriptorHeap,
        resources: &BTreeMap<String, GpuResource>,
        node: &crate::graph_config::PlannedComputeNode,
    ) -> Result<()> {
        let increment = unsafe {
            device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV)
        } as usize;
        let heap_start = unsafe { descriptor_heap.GetCPUDescriptorHandleForHeapStart() };

        for binding in &node.bindings {
            let handle = descriptor_handle_at(heap_start, binding.slot as usize, increment);
            let resource = resources.get(&binding.resource).ok_or_else(|| {
                Error::new(
                    E_FAIL,
                    format!("missing GPU resource '{}'", binding.resource),
                )
            })?;

            match &resource.definition {
                ResourceDefinition::Image(image) => match binding.access {
                    crate::graph_config::ResourceAccess::Read => unsafe {
                        let view_desc = D3D12_SHADER_RESOURCE_VIEW_DESC {
                            Format: dxgi_format_for_image(image.format),
                            ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2D,
                            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
                            Anonymous: Default::default(),
                        };
                        let mut view_desc = view_desc;
                        view_desc.Anonymous.Texture2D = D3D12_TEX2D_SRV {
                            MostDetailedMip: 0,
                            MipLevels: 1,
                            PlaneSlice: 0,
                            ResourceMinLODClamp: 0.0,
                        };
                        device.CreateShaderResourceView(
                            Some(&resource.resource),
                            Some(&view_desc),
                            handle,
                        );
                    },
                    crate::graph_config::ResourceAccess::Write
                    | crate::graph_config::ResourceAccess::ReadWrite => unsafe {
                        let view_desc = D3D12_UNORDERED_ACCESS_VIEW_DESC {
                            Format: dxgi_format_for_image(image.format),
                            ViewDimension: D3D12_UAV_DIMENSION_TEXTURE2D,
                            Anonymous: Default::default(),
                        };
                        let mut view_desc = view_desc;
                        view_desc.Anonymous.Texture2D = D3D12_TEX2D_UAV {
                            MipSlice: 0,
                            PlaneSlice: 0,
                        };
                        device.CreateUnorderedAccessView(
                            Some(&resource.resource),
                            None,
                            Some(&view_desc),
                            handle,
                        );
                    },
                },
                ResourceDefinition::Buffer(buffer) => {
                    let num_elements =
                        checked_buffer_element_count(&buffer.name, buffer.size_in_bytes)?;
                    match binding.access {
                        crate::graph_config::ResourceAccess::Read => unsafe {
                            let view_desc = D3D12_SHADER_RESOURCE_VIEW_DESC {
                                Format: DXGI_FORMAT_UNKNOWN,
                                ViewDimension: D3D12_SRV_DIMENSION_BUFFER,
                                Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
                                Anonymous: Default::default(),
                            };
                            let mut view_desc = view_desc;
                            view_desc.Anonymous.Buffer = D3D12_BUFFER_SRV {
                                FirstElement: 0,
                                NumElements: num_elements,
                                StructureByteStride: STRUCTURED_BUFFER_STRIDE,
                                Flags: D3D12_BUFFER_SRV_FLAG_NONE,
                            };
                            device.CreateShaderResourceView(
                                Some(&resource.resource),
                                Some(&view_desc),
                                handle,
                            );
                        },
                        crate::graph_config::ResourceAccess::Write
                        | crate::graph_config::ResourceAccess::ReadWrite => unsafe {
                            let view_desc = D3D12_UNORDERED_ACCESS_VIEW_DESC {
                                Format: DXGI_FORMAT_UNKNOWN,
                                ViewDimension: D3D12_UAV_DIMENSION_BUFFER,
                                Anonymous: Default::default(),
                            };
                            let mut view_desc = view_desc;
                            view_desc.Anonymous.Buffer = D3D12_BUFFER_UAV {
                                FirstElement: 0,
                                NumElements: num_elements,
                                StructureByteStride: STRUCTURED_BUFFER_STRIDE,
                                CounterOffsetInBytes: 0,
                                Flags: D3D12_BUFFER_UAV_FLAG_NONE,
                            };
                            device.CreateUnorderedAccessView(
                                Some(&resource.resource),
                                None,
                                Some(&view_desc),
                                handle,
                            );
                        },
                    }
                }
            }
        }

        Ok(())
    }

    fn planned_barrier_to_dx12(
        barrier: &PlannedBarrier,
        resources: &BTreeMap<String, GpuResource>,
    ) -> Result<D3D12_RESOURCE_BARRIER> {
        let resource = &resources
            .get(match barrier {
                PlannedBarrier::Transition { resource, .. } | PlannedBarrier::Uav { resource } => {
                    resource
                }
            })
            .ok_or_else(|| Error::new(E_FAIL, "planned barrier references missing resource"))?
            .resource;

        Ok(match barrier {
            PlannedBarrier::Transition { before, after, .. } => transition_resource_barrier(
                resource,
                planned_state_to_dx12(*before),
                planned_state_to_dx12(*after),
            ),
            PlannedBarrier::Uav { .. } => uav_resource_barrier(resource),
        })
    }

    fn embedded_compute_shader(name: &str) -> Option<&'static [u8]> {
        match name {
            "simple_compute" | "noop" => Some(build_time_compute_shader()),
            "checkerboard_compute" => Some(build_time_checkerboard_compute_shader()),
            _ => None,
        }
    }

    fn planned_state_to_dx12(state: PlannedResourceState) -> D3D12_RESOURCE_STATES {
        match state {
            PlannedResourceState::ShaderRead => D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE,
            PlannedResourceState::UnorderedAccess => D3D12_RESOURCE_STATE_UNORDERED_ACCESS,
        }
    }

    fn dxgi_format_for_image(
        format: ImageFormat,
    ) -> windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT {
        match format {
            ImageFormat::Rgba8Unorm => DXGI_FORMAT_R8G8B8A8_UNORM,
        }
    }

    fn checked_buffer_element_count(name: &str, size_in_bytes: u64) -> Result<u32> {
        if size_in_bytes % STRUCTURED_BUFFER_STRIDE as u64 != 0 {
            return Err(Error::new(
                E_FAIL,
                format!(
                    "buffer '{name}' size {size_in_bytes} is not divisible by {STRUCTURED_BUFFER_STRIDE}"
                ),
            ));
        }

        let element_count = size_in_bytes / STRUCTURED_BUFFER_STRIDE as u64;
        u32::try_from(element_count).map_err(|_| {
            Error::new(
                E_FAIL,
                format!("buffer '{name}' is too large for a structured buffer view"),
            )
        })
    }

    fn create_warp_device() -> Result<WarpDevice> {
        unsafe {
            let factory: IDXGIFactory4 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0))?;
            let adapter: IDXGIAdapter4 = factory.EnumWarpAdapter()?;
            let descriptor = adapter.GetDesc3()?;
            let feature_level = highest_supported_feature_level(&adapter)?;

            let mut device: Option<ID3D12Device> = None;
            D3D12CreateDevice(&adapter, feature_level, &mut device)?;

            Ok(WarpDevice {
                adapter_name: utf16z_to_string(&descriptor.Description),
                feature_level,
                device: device.expect("D3D12CreateDevice succeeded without returning a device"),
            })
        }
    }

    fn compile_compute_shader(shader_source: &str, source_name: &str) -> Result<Vec<u8>> {
        let dxc_path = map_external_result(find_dxc_library("dxcompiler.dll"))?;
        let dxc = map_external_result(Dxc::new(Some(dxc_path)))?;
        let compiler = map_external_result(dxc.create_compiler())?;
        let library = map_external_result(dxc.create_library())?;
        let source_blob =
            map_external_result(library.create_blob_with_encoding_from_str(shader_source))?;

        match compiler.compile(
            &source_blob,
            source_name,
            "main",
            COMPUTE_SHADER_PROFILE,
            &[],
            None,
            &[],
        ) {
            Ok(result) => {
                let shader_blob = map_external_result(result.get_result())?;
                Ok(shader_blob.to_vec())
            }
            Err((result, error)) => {
                let error_message = result
                    .get_error_buffer()
                    .ok()
                    .and_then(|error_blob| library.get_blob_as_string(&error_blob.into()).ok())
                    .filter(|message| !message.trim().is_empty())
                    .unwrap_or_else(|| format!("{error:?}"));
                Err(Error::new(E_FAIL, error_message))
            }
        }
    }

    fn create_compute_pipeline_state_for_device(
        device: &ID3D12Device,
        compiled_shader: &[u8],
    ) -> Result<ID3D12PipelineState> {
        let root_signature = create_dynamic_resource_root_signature(device)?;
        let mut pipeline_state_desc = D3D12_COMPUTE_PIPELINE_STATE_DESC {
            pRootSignature: ManuallyDrop::new(Some(root_signature)),
            CS: shader_bytecode(compiled_shader),
            ..Default::default()
        };

        let pipeline_state = unsafe { device.CreateComputePipelineState(&pipeline_state_desc) };
        unsafe {
            ManuallyDrop::drop(&mut pipeline_state_desc.pRootSignature);
        }
        pipeline_state
    }

    fn create_dynamic_resource_root_signature(
        device: &ID3D12Device,
    ) -> Result<ID3D12RootSignature> {
        let root_signature_desc = D3D12_ROOT_SIGNATURE_DESC {
            Flags: D3D12_ROOT_SIGNATURE_FLAG_CBV_SRV_UAV_HEAP_DIRECTLY_INDEXED
                | D3D12_ROOT_SIGNATURE_FLAG_SAMPLER_HEAP_DIRECTLY_INDEXED,
            ..Default::default()
        };
        let mut serialized_root_signature = None;

        unsafe {
            D3D12SerializeRootSignature(
                &root_signature_desc,
                D3D_ROOT_SIGNATURE_VERSION_1,
                &mut serialized_root_signature,
                None,
            )?;
        }

        let serialized_root_signature =
            serialized_root_signature.expect("root signature serialization returned no blob");
        let root_signature_bytes = blob_bytes(&serialized_root_signature);
        unsafe { device.CreateRootSignature(0, root_signature_bytes) }
    }

    fn create_shader_visible_uav_heap(
        device: &ID3D12Device,
        num_descriptors: u32,
    ) -> Result<ID3D12DescriptorHeap> {
        let heap_desc = D3D12_DESCRIPTOR_HEAP_DESC {
            Type: D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
            NumDescriptors: num_descriptors,
            Flags: D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
            ..Default::default()
        };
        unsafe { device.CreateDescriptorHeap(&heap_desc) }
    }

    fn descriptor_handle_at(
        start: windows::Win32::Graphics::Direct3D12::D3D12_CPU_DESCRIPTOR_HANDLE,
        index: usize,
        increment: usize,
    ) -> windows::Win32::Graphics::Direct3D12::D3D12_CPU_DESCRIPTOR_HANDLE {
        windows::Win32::Graphics::Direct3D12::D3D12_CPU_DESCRIPTOR_HANDLE {
            ptr: start.ptr + index * increment,
        }
    }

    fn create_readback_buffer(device: &ID3D12Device, size_in_bytes: u64) -> Result<ID3D12Resource> {
        let buffer_desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Width: size_in_bytes,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            ..Default::default()
        };

        let mut readback_buffer = None;
        unsafe {
            device.CreateCommittedResource(
                &default_heap_properties(D3D12_HEAP_TYPE_READBACK),
                D3D12_HEAP_FLAG_NONE,
                &buffer_desc,
                D3D12_RESOURCE_STATE_COPY_DEST,
                None,
                &mut readback_buffer,
            )?;
        }
        Ok(readback_buffer.expect("CreateCommittedResource returned no readback buffer"))
    }

    fn create_command_queue(device: &ID3D12Device) -> Result<ID3D12CommandQueue> {
        let queue_desc = D3D12_COMMAND_QUEUE_DESC {
            Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
            ..Default::default()
        };
        unsafe { device.CreateCommandQueue(&queue_desc) }
    }

    fn create_command_allocator(device: &ID3D12Device) -> Result<ID3D12CommandAllocator> {
        unsafe { device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT) }
    }

    fn create_command_list(
        device: &ID3D12Device,
        command_allocator: &ID3D12CommandAllocator,
    ) -> Result<ID3D12GraphicsCommandList> {
        unsafe {
            device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, command_allocator, None)
        }
    }

    fn create_fence(device: &ID3D12Device) -> Result<ID3D12Fence> {
        unsafe { device.CreateFence(0, D3D12_FENCE_FLAG_NONE) }
    }

    fn wait_for_fence(fence: &ID3D12Fence, value: u64) -> Result<()> {
        if unsafe { fence.GetCompletedValue() } < value {
            let event = unsafe { CreateEventA(None, false, false, None)? };
            unsafe { fence.SetEventOnCompletion(value, event)? };
            unsafe { WaitForSingleObject(event, INFINITE) };
            unsafe { CloseHandle(event)? };
        }
        Ok(())
    }

    fn readback_texture(
        readback_buffer: &ID3D12Resource,
        placed_footprint: &D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
        width: usize,
        height: usize,
    ) -> Result<Vec<u8>> {
        let row_pitch = placed_footprint.Footprint.RowPitch as usize;
        let bytes_per_row = width * CHECKERBOARD_PIXEL_SIZE;
        let mut mapped_data = ptr::null_mut();

        unsafe {
            readback_buffer.Map(0, None, Some(&mut mapped_data))?;
        }

        let mapped_data = mapped_data.cast::<u8>();
        let mut pixels = vec![0; bytes_per_row * height];

        for row in 0..height {
            let source_offset = row * row_pitch;
            let destination_offset = row * bytes_per_row;
            let source =
                unsafe { slice::from_raw_parts(mapped_data.add(source_offset), bytes_per_row) };
            pixels[destination_offset..destination_offset + bytes_per_row].copy_from_slice(source);
        }

        unsafe {
            readback_buffer.Unmap(0, None);
        }

        Ok(pixels)
    }

    fn default_heap_properties(heap_type: D3D12_HEAP_TYPE) -> D3D12_HEAP_PROPERTIES {
        D3D12_HEAP_PROPERTIES {
            Type: heap_type,
            ..Default::default()
        }
    }

    fn transition_resource_barrier(
        resource: &ID3D12Resource,
        state_before: D3D12_RESOURCE_STATES,
        state_after: D3D12_RESOURCE_STATES,
    ) -> D3D12_RESOURCE_BARRIER {
        D3D12_RESOURCE_BARRIER {
            Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
            Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
            Anonymous: D3D12_RESOURCE_BARRIER_0 {
                Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                    pResource: ManuallyDrop::new(Some(resource.clone())),
                    StateBefore: state_before,
                    StateAfter: state_after,
                    Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                }),
            },
        }
    }

    fn planned_transition_resource_barrier(
        resource: &ID3D12Resource,
        state_before: PlannedResourceState,
        state_after: D3D12_RESOURCE_STATES,
    ) -> D3D12_RESOURCE_BARRIER {
        transition_resource_barrier(resource, planned_state_to_dx12(state_before), state_after)
    }

    fn uav_resource_barrier(resource: &ID3D12Resource) -> D3D12_RESOURCE_BARRIER {
        D3D12_RESOURCE_BARRIER {
            Type: D3D12_RESOURCE_BARRIER_TYPE_UAV,
            Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
            Anonymous: D3D12_RESOURCE_BARRIER_0 {
                UAV: ManuallyDrop::new(D3D12_RESOURCE_UAV_BARRIER {
                    pResource: ManuallyDrop::new(Some(resource.clone())),
                }),
            },
        }
    }

    fn submit_resource_barriers(
        command_list: &ID3D12GraphicsCommandList,
        barriers: &mut [D3D12_RESOURCE_BARRIER],
    ) {
        if barriers.is_empty() {
            return;
        }

        unsafe {
            command_list.ResourceBarrier(barriers);
        }
    }

    fn placed_texture_copy_location(
        resource: &ID3D12Resource,
        placed_footprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
    ) -> D3D12_TEXTURE_COPY_LOCATION {
        D3D12_TEXTURE_COPY_LOCATION {
            pResource: ManuallyDrop::new(Some(resource.clone())),
            Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                PlacedFootprint: placed_footprint,
            },
        }
    }

    fn subresource_texture_copy_location(
        resource: &ID3D12Resource,
        subresource: u32,
    ) -> D3D12_TEXTURE_COPY_LOCATION {
        D3D12_TEXTURE_COPY_LOCATION {
            pResource: ManuallyDrop::new(Some(resource.clone())),
            Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                SubresourceIndex: subresource,
            },
        }
    }

    fn blob_bytes(blob: &ID3DBlob) -> &[u8] {
        unsafe { slice::from_raw_parts(blob.GetBufferPointer().cast(), blob.GetBufferSize()) }
    }

    fn shader_bytecode(compiled_shader: &[u8]) -> D3D12_SHADER_BYTECODE {
        D3D12_SHADER_BYTECODE {
            pShaderBytecode: compiled_shader.as_ptr().cast(),
            BytecodeLength: compiled_shader.len(),
        }
    }

    fn map_external_result<T, E: fmt::Display>(result: std::result::Result<T, E>) -> Result<T> {
        result.map_err(|error| Error::new(E_FAIL, format!("{error}")))
    }

    fn highest_supported_feature_level(adapter: &IDXGIAdapter4) -> Result<D3D_FEATURE_LEVEL> {
        unsafe {
            let mut last_error = None;

            for feature_level in [
                D3D_FEATURE_LEVEL_12_2,
                D3D_FEATURE_LEVEL_12_1,
                D3D_FEATURE_LEVEL_12_0,
                D3D_FEATURE_LEVEL_11_1,
                D3D_FEATURE_LEVEL_11_0,
            ] {
                let mut device: Option<ID3D12Device> = None;

                match D3D12CreateDevice(adapter, feature_level, &mut device) {
                    Ok(()) => return Ok(feature_level),
                    Err(error) => last_error = Some(error),
                }
            }

            Err(last_error.expect("feature level probing did not produce a result"))
        }
    }

    fn try_check_feature_support<T: Default>(
        device: &ID3D12Device,
        feature: D3D12_FEATURE,
    ) -> Option<T> {
        unsafe {
            let mut data = T::default();
            device
                .CheckFeatureSupport(feature, &mut data as *mut _ as _, size_of::<T>() as u32)
                .ok()?;
            Some(data)
        }
    }

    fn try_highest_shader_model(device: &ID3D12Device) -> Option<D3D_SHADER_MODEL> {
        for requested_shader_model in [
            D3D_HIGHEST_SHADER_MODEL,
            D3D_SHADER_MODEL_6_8,
            D3D_SHADER_MODEL_6_7,
            D3D_SHADER_MODEL_6_6,
            D3D_SHADER_MODEL_6_5,
            D3D_SHADER_MODEL_6_4,
            D3D_SHADER_MODEL_6_3,
            D3D_SHADER_MODEL_6_2,
            D3D_SHADER_MODEL_6_1,
            D3D_SHADER_MODEL_6_0,
            D3D_SHADER_MODEL_5_1,
        ] {
            let mut shader_model = D3D12_FEATURE_DATA_SHADER_MODEL {
                HighestShaderModel: requested_shader_model,
            };

            unsafe {
                if device
                    .CheckFeatureSupport(
                        D3D12_FEATURE_SHADER_MODEL,
                        &mut shader_model as *mut _ as _,
                        size_of::<D3D12_FEATURE_DATA_SHADER_MODEL>() as u32,
                    )
                    .is_ok()
                {
                    return Some(shader_model.HighestShaderModel);
                }
            }
        }

        None
    }

    fn format_feature_level(feature_level: D3D_FEATURE_LEVEL) -> &'static str {
        match feature_level {
            value if value == D3D_FEATURE_LEVEL_12_2 => "12.2",
            value if value == D3D_FEATURE_LEVEL_12_1 => "12.1",
            value if value == D3D_FEATURE_LEVEL_12_0 => "12.0",
            value if value == D3D_FEATURE_LEVEL_11_1 => "11.1",
            value if value == D3D_FEATURE_LEVEL_11_0 => "11.0",
            _ => "Unknown",
        }
    }

    fn format_raytracing_tier(tier: D3D12_RAYTRACING_TIER) -> &'static str {
        match tier {
            value if value == D3D12_RAYTRACING_TIER_1_1 => "Tier 1.1",
            value if value == D3D12_RAYTRACING_TIER_1_0 => "Tier 1.0",
            value if value == D3D12_RAYTRACING_TIER_NOT_SUPPORTED => "Not supported",
            _ => "Unknown",
        }
    }

    fn format_mesh_shader_tier(tier: D3D12_MESH_SHADER_TIER) -> &'static str {
        match tier {
            value if value == D3D12_MESH_SHADER_TIER_1 => "Tier 1",
            value if value == D3D12_MESH_SHADER_TIER_NOT_SUPPORTED => "Not supported",
            _ => "Unknown",
        }
    }

    fn format_shader_model(shader_model: D3D_SHADER_MODEL) -> &'static str {
        match shader_model {
            value if value == D3D_SHADER_MODEL_6_9 => "6.9",
            value if value == D3D_SHADER_MODEL_6_8 => "6.8",
            value if value == D3D_SHADER_MODEL_6_7 => "6.7",
            value if value == D3D_SHADER_MODEL_6_6 => "6.6",
            value if value == D3D_SHADER_MODEL_6_5 => "6.5",
            value if value == D3D_SHADER_MODEL_6_4 => "6.4",
            value if value == D3D_SHADER_MODEL_6_3 => "6.3",
            value if value == D3D_SHADER_MODEL_6_2 => "6.2",
            value if value == D3D_SHADER_MODEL_6_1 => "6.1",
            value if value == D3D_SHADER_MODEL_6_0 => "6.0",
            value if value == D3D_SHADER_MODEL_5_1 => "5.1",
            _ => "Unknown",
        }
    }

    fn utf16z_to_string(value: &[u16]) -> String {
        let end = value
            .iter()
            .position(|&character| character == 0)
            .unwrap_or(value.len());
        String::from_utf16_lossy(&value[..end])
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    {
        match dx12::create_warp_device_capabilities() {
            Ok(capabilities) => println!("{capabilities}"),
            Err(error) => {
                eprintln!("Failed to create a DX12 WARP device: {error}");
                std::process::exit(1);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("This sample requires Windows to create a DX12 WARP device.");
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "windows")]
    #[test]
    fn creates_dx12_warp_device() -> windows::core::Result<()> {
        let capabilities = super::dx12::create_warp_device_capabilities()?;

        assert!(!capabilities.adapter_name.is_empty());
        assert!(!capabilities.feature_level.is_empty());
        assert!(!capabilities.highest_shader_model.is_empty());

        if capabilities.wave_ops_supported {
            assert!(capabilities.wave_lane_count_min > 0);
            assert!(capabilities.wave_lane_count_max >= capabilities.wave_lane_count_min);
            assert!(capabilities.total_lane_count >= capabilities.wave_lane_count_max);
        }

        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn compiles_compute_shader_at_build_time() {
        let compiled_shader = super::dx12::build_time_compute_shader();
        assert!(!compiled_shader.is_empty());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn compiles_compute_shader_at_runtime() -> windows::core::Result<()> {
        let compiled_shader = super::dx12::compile_runtime_compute_shader()?;
        assert!(!compiled_shader.is_empty());
        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn creates_compute_pso_from_build_time_shader() -> windows::core::Result<()> {
        let _pipeline_state =
            super::dx12::create_compute_pipeline_state(super::dx12::build_time_compute_shader())?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn creates_compute_pso_from_runtime_shader() -> windows::core::Result<()> {
        let compiled_shader = super::dx12::compile_runtime_compute_shader()?;
        let _pipeline_state = super::dx12::create_compute_pipeline_state(&compiled_shader)?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn dispatches_checkerboard_compute_shader_from_toml_config() -> windows::core::Result<()> {
        let config_toml = r#"
            [[images]]
            name = "checkerboard_output"
            width = 8
            height = 8
            format = "rgba8_unorm"

            [[buffers]]
            name = "scratch_buffer"
            size_in_bytes = 64

            [[compute_nodes]]
            name = "generate_checkerboard"
            shader = "checkerboard_compute"
            dispatch = [1, 1, 1]

            [[compute_nodes.bindings]]
            resource = "checkerboard_output"
            slot = 0
            access = "write"

            [[compute_nodes]]
            name = "preserve_output"
            shader = "noop"
            dispatch = [1, 1, 1]

            [[compute_nodes.bindings]]
            resource = "checkerboard_output"
            slot = 0
            access = "write"
        "#;
        let (plan, pixels) =
            super::dx12::dispatch_configured_compute_graph(config_toml, "checkerboard_output")?;

        assert_eq!(plan.nodes[0].dependencies, Vec::<String>::new());
        assert_eq!(
            plan.nodes[1].dependencies,
            vec!["generate_checkerboard".to_string()]
        );
        assert_eq!(
            plan.nodes[1].barriers_before,
            vec![crate::graph_config::PlannedBarrier::Uav {
                resource: "checkerboard_output".to_string(),
            }]
        );

        for y in 0..8 {
            for x in 0..8 {
                let offset = ((y * 8 + x) * 4) as usize;
                let expected = if (x + y) % 2 == 0 {
                    [255, 255, 255, 255]
                } else {
                    [0, 0, 0, 255]
                };
                assert_eq!(&pixels[offset..offset + 4], &expected);
            }
        }

        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn compiles_checkerboard_compute_shader_at_runtime() -> windows::core::Result<()> {
        let compiled_shader = super::dx12::compile_runtime_checkerboard_compute_shader()?;
        assert!(!compiled_shader.is_empty());
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn creates_dx12_warp_device() {
        let unsupported_platform = std::env::consts::OS;
        assert_ne!(unsupported_platform, "windows");
    }
}
