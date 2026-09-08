//! Project-type detection.
//!
//! **Adding a detector is two lines in this file.** Create
//! `detectors/<kind>/` holding `detector.rs`, `info.rs` and `error.rs` (copy
//! the shape of `git/`), then declare the module below and list it in
//! [`default_detectors`]. Nothing else in the crate needs to change: the
//! tracker payload is generic, `DetectorError` carries no per-detector
//! variant, and the frontend renders unfamiliar kinds off their serde shape.

pub mod detector;
pub mod runner;

pub mod git;
pub mod unreal;

pub use detector::Detector;
pub use runner::{Detection, DetectorOutcome, DetectorRunner};

/// The detector set the app runs, in the order they are consulted.
///
/// The one list. `DetectorRunner::default` and the runner in Tauri's managed
/// state are both built from it, so registering here covers every path
/// detection runs through.
pub fn default_detectors() -> Vec<Box<dyn Detector>> {
    vec![Box::new(git::Gitector), Box::new(unreal::UnrealDetector)]
}
