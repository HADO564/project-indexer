use crate::detectors::detector::Detector;
use crate::detectors::git::Gitector;
use crate::detectors::unreal::UnrealDetector;

/// The detector set the app runs by default, in the order they're consulted.
///
/// This is the single place a new detector is registered — add its
/// `Box::new(...)` here and nothing else in `detectors/` needs to change.
/// [`DetectorRunner::default`](crate::detectors::DetectorRunner::default) and
/// the runner held in Tauri's managed state are both built from this list, so
/// registering once covers every path detection runs through.
pub fn default_detectors() -> Vec<Box<dyn Detector>> {
    vec![Box::new(Gitector), Box::new(UnrealDetector)]
}
