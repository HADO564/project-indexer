//! Everything the .NET detector is: the `Detector` impl and the info struct it
//! produces. No error type of its own — presence of a project, a solution or
//! an SDK pin is the whole signal, so nothing here raises anything beyond the
//! shared `DetectorError::Io`.

pub mod detector;
pub mod info;

pub use detector::DotnetDetector;
pub use info::DotnetInfo;
