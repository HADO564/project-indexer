use std::path::Path;
use std::sync::Arc;

use crate::application::inspection::{results_from, DirectoryState, ProjectInspection};
use crate::detectors::DetectorRunner;
use crate::domain::naming::{disambiguate, suggest_project_name, taken_names_from};
use crate::domain::sorting::{filter_deleted, filter_favorites, sort_projects, SortOptions};
use crate::domain::{Project, Tracker, UpdateProject};
use crate::error::ProjectError;
use crate::platform::{check_directory_status, remove_directory, DirectoryStatus};
use crate::ports::{AppLauncher, GroupReader, ProjectRepository};

/// All the orchestration that used to live in the Tauri command handlers:
/// one method per current command, with the logic lifted unchanged. The
/// Tauri layer becomes a thin adapter over this, and a future CLI can drive
/// the exact same flows.
pub struct ProjectService {
    repo: Arc<dyn ProjectRepository>,
    launcher: Arc<dyn AppLauncher>,
    detectors: Arc<DetectorRunner>,
    groups: Arc<dyn GroupReader>,
}

/// Opaque by necessity — every field is a `dyn` port with no `Debug` bound, and
/// requiring one would constrain every future adapter for no gain. Exists so
/// types that hold a `ProjectService` can still `#[derive(Debug)]`.
impl std::fmt::Debug for ProjectService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectService").finish_non_exhaustive()
    }
}

// One service, split by the kind of operation rather than by type, so a
// change to (say) how a project is opened touches one file. Every child
// module adds an `impl ProjectService` block: Rust allows a type any number
// of inherent impls, and a child module can reach the private fields below
// because privacy extends to descendants.
mod bin;
mod crud;
mod detection;
mod directory;
mod launching;
mod queries;

pub use detection::{SweepFailure, SweepReport};

impl ProjectService {
    pub fn new(
        repo: Arc<dyn ProjectRepository>,
        launcher: Arc<dyn AppLauncher>,
        detectors: Arc<DetectorRunner>,
        groups: Arc<dyn GroupReader>,
    ) -> Self {
        Self {
            repo,
            launcher,
            detectors,
            groups,
        }
    }

    fn load(&self, id: &str) -> Result<Project, ProjectError> {
        self.repo
            .get(id)?
            .ok_or_else(|| ProjectError::NotFound(id.to_string()))
    }
}
