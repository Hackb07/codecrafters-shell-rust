use std::env;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub fn find_executable(command: &str) -> Option<PathBuf> {
    let path_env = env::var("PATH").unwrap_or_default();
    for dir in env::split_paths(&path_env) {
        let full_path = dir.join(command);
        if full_path.is_file() && is_executable(&full_path) {
            return Some(full_path);
        }
    }
    None
}

pub fn open_file(path: &str, append: bool) -> File {
    OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(path)
        .unwrap()
}

pub fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    if let Ok(metadata) = fs::metadata(path) {
        let mode = metadata.permissions().mode();
        return mode & 0o111 != 0;
    }

    path.is_file()
}
