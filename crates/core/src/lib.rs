pub mod application;
pub mod detectors;
pub mod domain;
pub mod error;
pub mod icons;
pub mod infra;
pub mod platform;
pub mod ports;

#[cfg(test)]
mod tests;

pub use application::{GroupService, ProjectInspection, ProjectService, ScanService};
pub use detectors::git::{GitError, GitInfo};
pub use detectors::unreal::{UnrealError, UnrealInfo};
pub use detectors::{Detection, DetectorOutcome, DetectorRunner};
pub use domain::{Group, InstalledApp, Project, Tracker, UpdateGroup, UpdateProject};
pub use error::{DetectorError, IconError, LauncherError, ProjectError, RepositoryError};
pub use infra::{IconStore, SqliteRepository, StoredIcon, CURRENT_SCHEMA_VERSION};
pub use ports::{AppLauncher, GroupReader, GroupRepository, ProjectReader, ProjectRepository};
