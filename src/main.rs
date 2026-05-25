#[cfg(target_os = "windows")]
mod dx12 {
    use std::{fmt, mem::size_of};

    use windows::{
        Win32::Graphics::{
            Direct3D::{
                D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1,
                D3D_FEATURE_LEVEL_12_0, D3D_FEATURE_LEVEL_12_1, D3D_FEATURE_LEVEL_12_2,
            },
            Direct3D12::{
                D3D_HIGHEST_SHADER_MODEL, D3D_SHADER_MODEL, D3D_SHADER_MODEL_5_1,
                D3D_SHADER_MODEL_6_0, D3D_SHADER_MODEL_6_1, D3D_SHADER_MODEL_6_2,
                D3D_SHADER_MODEL_6_3, D3D_SHADER_MODEL_6_4, D3D_SHADER_MODEL_6_5,
                D3D_SHADER_MODEL_6_6, D3D_SHADER_MODEL_6_7, D3D_SHADER_MODEL_6_8,
                D3D_SHADER_MODEL_6_9, D3D12_FEATURE, D3D12_FEATURE_ARCHITECTURE1,
                D3D12_FEATURE_D3D12_OPTIONS, D3D12_FEATURE_D3D12_OPTIONS1,
                D3D12_FEATURE_D3D12_OPTIONS5, D3D12_FEATURE_D3D12_OPTIONS7,
                D3D12_FEATURE_DATA_ARCHITECTURE1, D3D12_FEATURE_DATA_D3D12_OPTIONS,
                D3D12_FEATURE_DATA_D3D12_OPTIONS1, D3D12_FEATURE_DATA_D3D12_OPTIONS5,
                D3D12_FEATURE_DATA_D3D12_OPTIONS7, D3D12_FEATURE_DATA_SHADER_MODEL,
                D3D12_FEATURE_SHADER_MODEL, D3D12_MESH_SHADER_TIER, D3D12_MESH_SHADER_TIER_1,
                D3D12_MESH_SHADER_TIER_NOT_SUPPORTED, D3D12_RAYTRACING_TIER,
                D3D12_RAYTRACING_TIER_1_0, D3D12_RAYTRACING_TIER_1_1,
                D3D12_RAYTRACING_TIER_NOT_SUPPORTED, D3D12CreateDevice, ID3D12Device,
            },
            Dxgi::{CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, IDXGIAdapter4, IDXGIFactory4},
        },
        core::Result,
    };

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

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn creates_dx12_warp_device() {
        let unsupported_platform = std::env::consts::OS;
        assert_ne!(unsupported_platform, "windows");
    }
}
