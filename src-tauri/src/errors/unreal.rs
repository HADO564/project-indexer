use thiserror::Error;

#[derive(Error, Debug)]
pub enum UnrealError {
    #[error("Failed to parse .uproject file: {0}")]
    ParseUproject(#[source] serde_json::Error),
}
