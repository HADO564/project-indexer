//! Everything the Docker detector is: the `Detector` impl and the info struct
//! it produces. No error type of its own — presence of a Dockerfile or a
//! compose file is the whole signal, so nothing here raises anything beyond
//! the shared `DetectorError::Io`.

pub mod detector;
pub mod info;

pub use detector::DockerDetector;
pub use info::DockerInfo;
