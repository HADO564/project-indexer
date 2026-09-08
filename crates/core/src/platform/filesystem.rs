use std::io::ErrorKind;
use std::path::Path;

pub enum DirectoryStatus {
    Exists,
    DoesNotExist,
    NotADirectory,
    Inaccessible,
    PermissionDenied,
}

/// Recursively deletes a directory from disk, treating an already-missing
/// directory as success (deleting is idempotent: the goal state — the
/// directory being gone — is already reached). `std::fs::remove_dir_all`
/// works identically on Windows and Linux, so no OS-specific handling is
/// needed here.
pub fn remove_directory(path: &str) -> Result<(), String> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Failed to delete directory: {}", e)),
    }
}

pub fn check_directory_status(path: &str) -> DirectoryStatus {
    let path = Path::new(path);
    match std::fs::metadata(path) {
        Ok(metadata) => {
            if metadata.is_dir() {
                DirectoryStatus::Exists
            } else {
                DirectoryStatus::NotADirectory
            }
        }
        Err(e) => match e.kind() {
            ErrorKind::NotFound => DirectoryStatus::DoesNotExist,
            ErrorKind::PermissionDenied => DirectoryStatus::PermissionDenied,
            _ => DirectoryStatus::Inaccessible,
        },
    }
}
