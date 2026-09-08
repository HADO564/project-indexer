use thiserror::Error;

#[derive(Error, Debug)]
pub enum UnrealError {
    #[error("Failed to parse .uproject file: {0}")]
    ParseUproject(#[source] serde_json::Error),
}

/// Lets `?` lift a [`UnrealError`] into the shared [`DetectorError`] without
/// that enum carrying a variant per detector. Living here is what keeps
/// a new detector's blast radius inside its own directory.
impl From<UnrealError> for crate::error::DetectorError {
    fn from(e: UnrealError) -> Self {
        crate::error::DetectorError::Other(Box::new(e))
    }
}
