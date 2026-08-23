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
