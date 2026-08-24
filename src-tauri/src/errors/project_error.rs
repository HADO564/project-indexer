use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("Project name cannot be empty")]
    InvalidName,

    #[error("Project directory does not exist: {0}")]
    InvalidDirectory(String),

    #[error("A project with this name already exists: {0}")]
    DuplicateName(String),

    #[error("A project with this directory already exists: {0}")]
    DuplicateDirectory(String),

    #[error("Project with id '{0}' not found")]
    NotFound(String),

    #[error("Failed to open project directory: {0}")]
    OpenFailed(String),

    #[error("Project store error: {0}")]
    Store(String),

    #[error("Project directory is not accessible: {0}")]
    DirectoryInaccessible(String),

    #[error("Project directory has been deleted or moved: {0}")]
    DirectoryDeletedOrMoved(String),

    #[error("Project is not in the bin, so it can't be permanently deleted: {0}")]
    ProjectNotInBin(String),

    #[error("The app associated with this project has been removed or cannot be found: {0}")]
    OpenWithAppMissing(String),
}

/// Tauri serializes command errors as their `Display` string, so the JS
/// side keeps seeing the same plain-string error it always has.
impl serde::Serialize for ProjectError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
