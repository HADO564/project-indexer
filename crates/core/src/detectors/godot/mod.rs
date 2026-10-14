//! Everything the Godot detector is: the `Detector` impl and the info struct
//! it produces. No error type of its own — a `project.godot` with no readable
//! name degrades to a detected project with no name rather than failing, so
//! nothing here raises anything beyond the shared `DetectorError::Io`.

pub mod detector;
pub mod info;

pub use detector::GodotDetector;
pub use info::GodotInfo;
