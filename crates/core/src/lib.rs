pub mod detectors;
pub mod domain;
pub mod error;
pub mod infra;
pub mod platform;
pub mod ports;

pub use detectors::{Detection, DetectorOutcome, DetectorRunner};
pub use domain::{GitInfo, InstalledApp, Project, Tracker, UnrealInfo, UpdateProject};
pub use error::{
    DetectorError, GitError, LauncherError, ProjectError, RepositoryError, UnrealError,
};
pub use infra::{SqliteRepository, CURRENT_SCHEMA_VERSION};
pub use ports::{AppLauncher, ProjectReader, ProjectRepository};
