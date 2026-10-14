//! Everything the Gradle detector is: the `Detector` impl and the info struct
//! it produces. No error type of its own — presence of a build script is the
//! whole signal, so nothing here raises anything beyond the shared
//! `DetectorError::Io`.

pub mod detector;
pub mod info;

pub use detector::GradleDetector;
pub use info::GradleInfo;
