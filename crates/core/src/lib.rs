pub mod application;
pub mod detectors;
pub mod domain;
pub mod error;
pub mod icons;
pub mod infra;
pub mod platform;
pub mod ports;

pub use application::{GroupService, ProjectInspection, ProjectService, ScanService};
pub use detectors::{Detection, DetectorOutcome, DetectorRunner};
pub use domain::{
    GitInfo, Group, InstalledApp, Project, Tracker, UnrealInfo, UpdateGroup, UpdateProject,
};
pub use error::{
    DetectorError, GitError, IconError, LauncherError, ProjectError, RepositoryError, UnrealError,
};
pub use infra::{IconStore, SqliteRepository, StoredIcon, CURRENT_SCHEMA_VERSION};
pub use ports::{AppLauncher, GroupReader, GroupRepository, ProjectReader, ProjectRepository};
