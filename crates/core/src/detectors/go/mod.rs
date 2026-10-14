//! Everything the Go detector is: the `Detector` impl and the info struct it
//! produces. No error type of its own — a `go.mod` with no readable `module`
//! directive degrades to a detected project with no module path rather than
//! failing, so nothing here raises anything beyond the shared
//! `DetectorError::Io`.

pub mod detector;
pub mod info;

pub use detector::GoDetector;
pub use info::GoInfo;
