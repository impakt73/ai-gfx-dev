use std::{env, fs, io, path::PathBuf};

pub fn find_dxc_library(library_name: &str) -> io::Result<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(dxc_lib_dir) = env::var_os("DXC_LIB_DIR") {
        candidates.push(PathBuf::from(dxc_lib_dir).join(library_name));
    }

    if let Some(windows_sdk_dir) = env::var_os("WindowsSdkDir") {
        candidates.extend(windows_sdk_candidates(
            PathBuf::from(windows_sdk_dir),
            library_name,
        ));
    }

    if let Some(program_files_x86) = env::var_os("ProgramFiles(x86)") {
        candidates.extend(windows_sdk_candidates(
            PathBuf::from(program_files_x86)
                .join("Windows Kits")
                .join("10"),
            library_name,
        ));
    }

    candidates
        .push(PathBuf::from(r"C:\Windows\System32\Microsoft-Edge-WebView").join(library_name));

    candidates.into_iter().find(|path| path.is_file()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Unable to locate {library_name}. Set DXC_LIB_DIR to a directory containing the DXC binaries."
            ),
        )
    })
}

fn windows_sdk_candidates(base_dir: PathBuf, library_name: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let bin_dir = base_dir.join("bin");

    if let Ok(entries) = fs::read_dir(bin_dir) {
        let mut version_directories: Vec<_> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();

        version_directories.sort();
        version_directories.reverse();

        for version_directory in version_directories {
            candidates.push(version_directory.join("x64").join(library_name));
        }
    }

    candidates.push(
        base_dir
            .join("Redist")
            .join("D3D")
            .join("x64")
            .join(library_name),
    );
    candidates
}
