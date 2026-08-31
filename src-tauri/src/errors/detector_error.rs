use thiserror::Error;

use crate::errors::GitError;
use crate::errors::UnrealError;

/// Errors a [`Detector`](crate::detectors::Detector) can raise while
/// inspecting a path.
///
/// Most detectors only touch the filesystem and can lean on the `Io` variant.
/// The first-party git and Unreal detectors carry richer, structured error
/// types, so those get a dedicated `#[from]` variant. A new detector with its
/// own error type doesn't have to edit this enum — it boxes into `Other`:
///
/// ```ignore
/// something().map_err(|e| DetectorError::Other(Box::new(e)))?;
/// ```
#[derive(Debug, Error)]
pub enum DetectorError {
    #[error("Failed to read path: {0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Git(#[from] GitError),

    #[error(transparent)]
    Unreal(#[from] UnrealError),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// Tauri serializes command errors over IPC as JSON, so `DetectorError` needs
/// `Serialize` to be usable as a command's `Err` type directly — same
/// approach as [`crate::errors::ProjectError`].
impl serde::Serialize for DetectorError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
