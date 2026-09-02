use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub struct LauncherError(pub String);
