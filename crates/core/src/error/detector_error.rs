use thiserror::Error;

/// Errors a detector can raise while inspecting a path.
///
/// Most detectors only touch the filesystem and can lean on the `Io` variant.
/// A detector with its own structured error type never edits this enum: it
/// writes `impl From<MyError> for DetectorError` in its own directory, which
/// boxes into `Other` and keeps `?` working. `git/error.rs` is the example.
///
/// ```ignore
/// something().map_err(|e| DetectorError::Other(Box::new(e)))?;
/// ```
#[derive(Debug, Error)]
pub enum DetectorError {
    #[error("Failed to read path: {0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// Tauri serializes command errors over IPC as JSON, so `DetectorError` needs
/// `Serialize` to be usable as a command's `Err` type directly — same
/// approach as [`crate::error::ProjectError`].
impl serde::Serialize for DetectorError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
