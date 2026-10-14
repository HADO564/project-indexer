//! Everything the Node detector is: the `Detector` impl and the info struct
//! it produces. No error type of its own — a malformed `package.json` degrades
//! to a detected project with empty fields rather than failing, so nothing
//! here raises anything beyond the shared `DetectorError::Io`.

pub mod detector;
pub mod info;

pub use detector::NodeDetector;
pub use info::NodeInfo;
