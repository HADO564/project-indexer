//! Everything the Unity detector is: the `Detector` impl and the info struct
//! it produces. No error type of its own — a `ProjectVersion.txt` with no
//! readable editor version degrades to a detected project with no version
//! rather than failing, so nothing here raises anything beyond the shared
//! `DetectorError::Io`.

pub mod detector;
pub mod info;

pub use detector::UnityDetector;
pub use info::UnityInfo;
