//! Everything the Dart detector is: the `Detector` impl and the info struct it
//! produces. No error type of its own — a malformed `pubspec.yaml` degrades to
//! a detected project with empty fields rather than failing, so nothing here
//! raises anything beyond the shared `DetectorError::Io`.

pub mod detector;
pub mod info;

pub use detector::DartDetector;
pub use info::DartInfo;
