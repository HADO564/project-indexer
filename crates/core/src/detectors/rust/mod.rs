//! Everything the Rust detector is: the `Detector` impl and the info struct it
//! produces. No error type of its own — a `Cargo.toml` with no readable
//! `[package] name` (a virtual-manifest workspace root, say) degrades to a
//! detected project with no name rather than failing, so nothing here raises
//! anything beyond the shared `DetectorError::Io`.

pub mod detector;
pub mod info;

pub use detector::RustDetector;
pub use info::RustInfo;
