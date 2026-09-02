pub mod domain;
pub mod error;
pub mod platform;

pub use domain::{GitInfo, InstalledApp, Project, Tracker, UnrealInfo, UpdateProject};
pub use error::{
    DetectorError, GitError, LauncherError, ProjectError, RepositoryError, UnrealError,
};
