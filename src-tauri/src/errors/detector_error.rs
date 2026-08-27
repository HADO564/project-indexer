use thiserror::Error;

use crate::errors::GitError;
use crate::errors::UnrealError;

/// Errors a [`crate::detectors::detector::Detector`] can raise while
/// inspecting a path. Shared across all detector implementations (git,
/// Unity, Godot, Unreal, MATLAB, ...) — a tool-specific error type gets its
/// own variant here (as [`GitError`] does) rather than `Detector` growing a
/// per-tool error type.
#[derive(Debug, Error)]
pub enum DetectorError {
    #[error("Failed to read path: {0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Git(#[from] GitError),

    #[error(transparent)]
    Unreal(#[from] UnrealError),
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
