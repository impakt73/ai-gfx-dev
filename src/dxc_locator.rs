use std::{io, path::PathBuf};

pub fn find_dxc_library(library_name: &str) -> io::Result<PathBuf> {
    let vendored_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("third_party")
        .join("dxc")
        .join("bin")
        .join("x64")
        .join(library_name);

    if vendored_path.is_file() {
        Ok(vendored_path)
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Unable to locate vendored DXC library at {}",
                vendored_path.display()
            ),
        ))
    }
}
