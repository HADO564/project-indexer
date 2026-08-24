use std::io::ErrorKind;
use std::path::Path;

pub enum DirectoryStatus {
    Exists,
    DoesNotExist,
    NotADirectory,
    Inaccessible,
    PermissionDenied,
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

        let dir = std::env::temp_dir().join("project-indexer-tests-no-access-dir");
        std::fs::create_dir_all(&dir).expect("should create temp dir");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o000))
            .expect("should lock down permissions");

        let status = check_directory_status(dir.to_str().unwrap());

        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755))
            .expect("should restore permissions for cleanup");
        std::fs::remove_dir_all(&dir).expect("should clean up temp dir");

        assert!(matches!(status, DirectoryStatus::PermissionDenied));
    }
}
