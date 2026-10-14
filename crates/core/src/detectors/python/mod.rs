//! Everything the Python detector is: the `Detector` impl and the info struct
//! it produces. No error type of its own — a malformed `pyproject.toml`
//! degrades to a detected project with no name rather than failing, so nothing
//! here raises anything beyond the shared `DetectorError::Io`.

pub mod detector;
pub mod info;

pub use detector::PythonDetector;
pub use info::PythonInfo;
