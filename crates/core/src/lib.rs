pub mod detectors;
pub mod domain;
pub mod error;
pub mod platform;

pub use detectors::{Detection, DetectorOutcome, DetectorRunner};
pub use domain::{GitInfo, InstalledApp, Project, Tracker, UnrealInfo, UpdateProject};
pub use error::{
    DetectorError, GitError, LauncherError, ProjectError, RepositoryError, UnrealError,
};
