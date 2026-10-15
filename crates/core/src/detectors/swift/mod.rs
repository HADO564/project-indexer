//! Everything the Swift detector is: the `Detector` impl and the info struct
//! it produces. No error type of its own — a `Package.swift` whose `name`
//! can't be read degrades to a detected project with no name rather than
//! failing, so nothing here raises anything beyond the shared
//! `DetectorError::Io`.

pub mod detector;
pub mod info;

pub use detector::SwiftDetector;
pub use info::SwiftInfo;
