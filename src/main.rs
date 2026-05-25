#[cfg(target_os = "windows")]
mod dx12 {
    use std::{fmt, mem::size_of};

    use windows::{
        Win32::Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_11_0,
            Direct3D12::{
                D3D12_FEATURE_ARCHITECTURE1, D3D12_FEATURE_D3D12_OPTIONS,
                D3D12_FEATURE_D3D12_OPTIONS1, D3D12_FEATURE_DATA_ARCHITECTURE1,
                D3D12_FEATURE_DATA_D3D12_OPTIONS, D3D12_FEATURE_DATA_D3D12_OPTIONS1,
                D3D12CreateDevice, ID3D12Device,
            },
            Dxgi::{CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, IDXGIAdapter4, IDXGIFactory4},
        },
        core::Result,
    };

    #[derive(Debug)]
    pub struct WarpDeviceCapabilities {
        pub adapter_name: String,
        pub resource_binding_tier: String,
        pub tiled_resources_tier: String,
        pub conservative_rasterization_tier: String,
        pub resource_heap_tier: String,
        pub wave_ops_supported: bool,
        pub int64_shader_ops_supported: bool,
        pub uma: bool,
        pub cache_coherent_uma: bool,
    }

    impl fmt::Display for WarpDeviceCapabilities {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            writeln!(f, "DX12 WARP device capabilities")?;
            writeln!(f, "Adapter: {}", self.adapter_name)?;
            writeln!(f, "Resource binding tier: {}", self.resource_binding_tier)?;
            writeln!(f, "Tiled resources tier: {}", self.tiled_resources_tier)?;
            writeln!(
                f,
                "Conservative rasterization tier: {}",
                self.conservative_rasterization_tier
            )?;
            writeln!(f, "Resource heap tier: {}", self.resource_heap_tier)?;
            writeln!(f, "Wave ops supported: {}", self.wave_ops_supported)?;
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
        unsafe {
            let factory: IDXGIFactory4 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0))?;
            let adapter: IDXGIAdapter4 = factory.EnumWarpAdapter()?;
            let descriptor = adapter.GetDesc3()?;
            let mut device: Option<ID3D12Device> = None;
            D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_11_0, &mut device)?;
            let device = device.expect("D3D12CreateDevice succeeded without returning a device");

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

            let mut architecture = D3D12_FEATURE_DATA_ARCHITECTURE1::default();
            architecture.NodeIndex = 0;
            device.CheckFeatureSupport(
                D3D12_FEATURE_ARCHITECTURE1,
                &mut architecture as *mut _ as _,
                size_of::<D3D12_FEATURE_DATA_ARCHITECTURE1>() as u32,
            )?;

            Ok(WarpDeviceCapabilities {
                adapter_name: utf16z_to_string(&descriptor.Description),
                resource_binding_tier: format!("{:?}", options.ResourceBindingTier),
                tiled_resources_tier: format!("{:?}", options.TiledResourcesTier),
                conservative_rasterization_tier: format!(
                    "{:?}",
                    options.ConservativeRasterizationTier
                ),
                resource_heap_tier: format!("{:?}", options.ResourceHeapTier),
                wave_ops_supported: options1.WaveOps.as_bool(),
                int64_shader_ops_supported: options1.Int64ShaderOps.as_bool(),
                uma: architecture.UMA.as_bool(),
                cache_coherent_uma: architecture.CacheCoherentUMA.as_bool(),
            })
        }
    }

    fn utf16z_to_string(value: &[u16]) -> String {
        let end = value.iter().position(|&character| character == 0).unwrap_or(value.len());
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

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn creates_dx12_warp_device() {
        let unsupported_platform = std::env::consts::OS;
        assert_ne!(unsupported_platform, "windows");
    }
}
