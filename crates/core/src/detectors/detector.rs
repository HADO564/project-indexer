use std::path::Path;

use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

/// A pluggable project-type detector (git, Unity, Godot, Unreal, MATLAB, ...).
///
/// A detector answers one question: *does `path` look like a project of my
/// type, and if so, what can I tell you about it?* `Ok(Some(tracker))` means
/// yes, `Ok(None)` means "not mine" (a normal outcome, not a failure), and
/// `Err` is reserved for a genuine problem inspecting the path — an unreadable
/// directory, a corrupt repository.
///
/// Kept object-safe so detectors can be held as `Vec<Box<dyn Detector>>` and
/// run against a path uniformly, without the caller knowing which concrete
/// detectors are registered. That rules out a generic `impl AsRef<Path>`
/// parameter, so [`detect`](Self::detect) takes a concrete `&Path`.
///
/// `Send + Sync` are supertraits (rather than being tacked onto individual
/// `Box<dyn Detector>` usage sites) so `dyn Detector` itself carries them:
/// a `DetectorRunner` built from these can be dropped straight into Tauri's
/// managed app state (`App::manage`), which requires `Send + Sync + 'static`
/// for anything it holds, without extra bounds at each call site.
pub trait Detector: Send + Sync {
    /// Stable, lowercase identity for this detector, e.g. `"git"`. Used to
    /// tag detection outcomes and to target a single detector on re-detect.
    fn kind(&self) -> &'static str;

    /// The [`Tracker`] for `path` if this detector recognizes it as one of
    /// its projects, or `None` if it doesn't.
    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError>;
}
