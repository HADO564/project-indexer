use std::path::Path;

use crate::errors::DetectorError;
use crate::models::tracker::Tracker;

/// A pluggable project-type detector (git, Unity, Godot, Unreal, MATLAB, ...).
///
/// Kept object-safe so detectors can be held as `Vec<Box<dyn Detector>>` and
/// run against a path uniformly, without the caller knowing which concrete
/// detectors are registered. That rules out a generic `impl AsRef<Path>`
/// parameter, so both methods take a concrete `&Path`.
///
/// `Send + Sync` are supertraits (rather than being tacked onto individual
/// `Box<dyn Detector>` usage sites) so `dyn Detector` itself carries them:
/// a `DetectorRunner` built from these can be dropped straight into Tauri's
/// managed app state (`App::manage`), which requires `Send + Sync + 'static`
/// for anything it holds, without extra bounds at each call site.
pub trait Detector: Send + Sync {
    /// Whether `path` looks like a project of this detector's type.
    fn detect(&self, path: &Path) -> Result<bool, DetectorError>;

    /// Gathers detail about `path` beyond the yes/no of [`detect`](Self::detect),
    /// returning the matching [`Tracker`] variant. Detectors that only need to
    /// report presence can skip overriding this.
    fn get_info(&self, _path: &Path) -> Result<Option<Tracker>, DetectorError> {
        Ok(None)
    }
}
