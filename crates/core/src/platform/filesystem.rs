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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_existing_directory() {
        let dir = std::env::temp_dir();
        assert!(matches!(
            check_directory_status(dir.to_str().unwrap()),
            DirectoryStatus::Exists
        ));
    }

    #[test]
    fn flags_missing_directory() {
        let missing = std::env::temp_dir().join("project-indexer-tests-missing-dir-xyz");
        assert!(matches!(
            check_directory_status(missing.to_str().unwrap()),
            DirectoryStatus::DoesNotExist
        ));
    }

    #[test]
    fn flags_a_file_as_not_a_directory() {
        let file = std::env::temp_dir().join("project-indexer-tests-status-file.txt");
        std::fs::write(&file, b"placeholder").expect("should write temp file");

        let status = check_directory_status(file.to_str().unwrap());

        std::fs::remove_file(&file).expect("should clean up temp file");
        assert!(matches!(status, DirectoryStatus::NotADirectory));
    }

    // Permission bits on Windows don't gate `fs::metadata` the way Unix mode
    // bits do, so this is only reliably testable on Unix.
    #[cfg(unix)]
    #[test]
    fn flags_permission_denied() {
        use std::os::unix::fs::PermissionsExt;

        // Statting a path resolves it through its parent, so the mode that
        // decides the outcome is the parent's execute bit, not the target's own
        // — a mode-000 directory is still perfectly stattable from outside.
        // The path under test therefore has to sit *inside* the locked
        // directory, so that resolving it requires traversing one we've closed.
        let outer = std::env::temp_dir().join("project-indexer-tests-no-access-dir");
        let inner = outer.join("child");
        std::fs::create_dir_all(&inner).expect("should create temp dirs");
        std::fs::set_permissions(&outer, std::fs::Permissions::from_mode(0o000))
            .expect("should lock down permissions");

        let status = check_directory_status(inner.to_str().unwrap());

        std::fs::set_permissions(&outer, std::fs::Permissions::from_mode(0o755))
            .expect("should restore permissions for cleanup");
        std::fs::remove_dir_all(&outer).expect("should clean up temp dir");

        assert!(matches!(status, DirectoryStatus::PermissionDenied));
    }
}
