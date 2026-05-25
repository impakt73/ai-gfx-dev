#[path = "src/dxc_locator.rs"]
mod dxc_locator;

use hassle_rs::Dxc;
use std::{env, fs, path::PathBuf};

const COMPUTE_SHADER_SOURCE_PATH: &str = "shaders/simple_compute.hlsl";
const BUILD_TIME_COMPUTE_SHADER_NAME: &str = "build_time_compute_shader.dxil";

#[cfg(target_os = "windows")]
#[link(name = "Advapi32")]
unsafe extern "system" {}

fn main() {
    println!("cargo:rerun-if-changed={COMPUTE_SHADER_SOURCE_PATH}");
    println!("cargo:rerun-if-changed=third_party/dxc/bin/x64/dxcompiler.dll");
    println!("cargo:rerun-if-changed=third_party/dxc/bin/x64/dxil.dll");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("cargo did not provide OUT_DIR"));
    let output_path = out_dir.join(BUILD_TIME_COMPUTE_SHADER_NAME);

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        fs::write(output_path, []).expect("failed to write non-Windows placeholder shader");
        return;
    }

    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("cargo did not provide CARGO_MANIFEST_DIR"),
    );
    let shader_path = manifest_dir.join(COMPUTE_SHADER_SOURCE_PATH);
    let shader_source = fs::read_to_string(&shader_path)
        .expect("failed to read compute shader source for build-time compilation");
    let compiled_shader = compile_compute_shader(&shader_source, COMPUTE_SHADER_SOURCE_PATH)
        .expect("failed to compile build-time compute shader");

    fs::write(output_path, compiled_shader)
        .expect("failed to write build-time compiled compute shader");
}

fn compile_compute_shader(shader_source: &str, source_name: &str) -> Result<Vec<u8>, String> {
    let dxc_path =
        dxc_locator::find_dxc_library("dxcompiler.dll").map_err(|error| error.to_string())?;
    let dxc = Dxc::new(Some(dxc_path)).map_err(|error| error.to_string())?;
    let compiler = dxc.create_compiler().map_err(|error| error.to_string())?;
    let library = dxc.create_library().map_err(|error| error.to_string())?;
    let source_blob = library
        .create_blob_with_encoding_from_str(shader_source)
        .map_err(|error| error.to_string())?;

    match compiler.compile(&source_blob, source_name, "main", "cs_6_0", &[], None, &[]) {
        Ok(result) => {
            let shader_blob = result.get_result().map_err(|error| error.to_string())?;
            Ok(shader_blob.to_vec())
        }
        Err((result, error)) => {
            let error_message = result
                .get_error_buffer()
                .ok()
                .and_then(|error_blob| library.get_blob_as_string(&error_blob.into()).ok())
                .filter(|message| !message.trim().is_empty())
                .unwrap_or_else(|| format!("{error:?}"));
            Err(error_message)
        }
    }
}
