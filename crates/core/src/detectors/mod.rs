//! Project-type detection.
//!
//! **Adding a detector is two lines in this file.** Create
//! `detectors/<kind>/` holding `detector.rs` and `info.rs` (copy the shape of
//! `git/`), plus `error.rs` only when the detector propagates a parse failure
//! the way `unreal/` does — the marker-file detectors here (`node`, `go`, ...)
//! instead degrade a malformed marker to "detected, fields empty", so they
//! need no error type. Then declare the module below and list it in
//! [`default_detectors`]. Little else in the crate changes: the tracker
//! payload is a per-detector info struct, `DetectorError` carries no
//! per-detector variant, and the frontend renders unfamiliar kinds off their
//! serde shape. The `Tracker` enum and the CLI's tracker columns do name every
//! kind, on purpose, so the compiler points at them when a kind is added.

pub mod detector;
pub mod runner;

pub mod cmake;
pub mod dart;
pub mod docker;
pub mod dotnet;
pub mod elixir;
pub mod git;
pub mod go;
pub mod godot;
pub mod gradle;
pub mod maven;
pub mod node;
pub mod php;
pub mod python;
pub mod ruby;
pub mod rust;
pub mod swift;
pub mod unity;
pub mod unreal;

pub use detector::Detector;
pub use runner::{Detection, DetectorOutcome, DetectorRunner};

/// The detector set the app runs, in the order they are consulted.
///
/// The one list. `DetectorRunner::default` and the runner in Tauri's managed
/// state are both built from it, so registering here covers every path
/// detection runs through.
pub fn default_detectors() -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(git::Gitector),
        Box::new(unreal::UnrealDetector),
        Box::new(node::NodeDetector),
        Box::new(python::PythonDetector),
        Box::new(go::GoDetector),
        Box::new(godot::GodotDetector),
        Box::new(unity::UnityDetector),
        Box::new(rust::RustDetector),
        Box::new(cmake::CmakeDetector),
        Box::new(maven::MavenDetector),
        Box::new(gradle::GradleDetector),
        Box::new(docker::DockerDetector),
        Box::new(php::PhpDetector),
        Box::new(ruby::RubyDetector),
        Box::new(dart::DartDetector),
        Box::new(swift::SwiftDetector),
        Box::new(elixir::ElixirDetector),
        Box::new(dotnet::DotnetDetector),
    ]
}
