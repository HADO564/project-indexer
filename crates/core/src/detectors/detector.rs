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
/// **Detectors stay stateless, and ideally zero-sized.** `Gitector` and
/// `UnrealDetector` are both unit structs, so the whole registered set costs
/// two fat pointers — 16 bytes each — and `Box::new` never reaches the
/// allocator for a zero-sized type. That is what makes "just register another
/// one" free: twenty detectors are 320 bytes at rest, and one that does not
/// match allocates nothing, because the expensive work sits behind the cheap
/// marker check that returns `Ok(None)`.
///
/// A detector holding state — a compiled regex, a cached schema, a memo table
/// — breaks that, and breaks it once per registered detector. Anything worth
/// caching belongs in the caller, or behind the fast-vs-deep detection split,
/// not in the detector itself. Statelessness is also what will make a detector
/// safe to instantiate per call when third-party ones arrive as wasm modules.
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
