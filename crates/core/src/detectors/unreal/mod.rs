//! Everything the unreal detector is: the `Detector` impl, the info
//! struct it produces, and its error type. Self-contained on purpose —
//! adding a detector should mean adding a directory, not editing five.

pub mod detector;
pub mod error;
pub mod info;

pub use detector::UnrealDetector;
pub use error::UnrealError;
pub use info::UnrealInfo;
