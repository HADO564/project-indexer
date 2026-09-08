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

    /// Best-effort: a project is still worth tracking even if we can't tell
    /// what kind of project it is. Detection is resilient — whatever
    /// detectors succeeded still count — and `refresh_trackers` lets the
    /// caller retry the rest explicitly, where a failure is worth surfacing.
    pub fn create(
        &self,
        name: String,
        directory: String,
        description: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<Project, ProjectError> {
        let existing_active: Vec<Project> = self
            .repo
            .list()?
            .into_iter()
            .filter(|p| !p.is_deleted)
            .collect();
        Project::check_for_duplicate_name_or_dir(&name, &directory, &existing_active)?;

        let mut project = Project::new(name, directory, description, tags)?;
        let detection = self.detectors.detect_project(Path::new(&project.directory));
        project.trackers = detection.trackers();
        for error in detection.errors() {
            eprintln!("Detector error for '{}': {}", project.directory, error);
        }
        self.repo.save(&project)?;
        Ok(project)
    }

    pub fn update(&self, id: &str, update: UpdateProject) -> Result<Project, ProjectError> {
        let mut project = self.load(id)?;
        // Checked here rather than in `Project::update` because it needs the
        // group store — a foreign key the DB would otherwise reject only
        // after the rest of the update was already applied in memory.
        // `Some(None)` (clearing the group) needs no check.
        if let Some(Some(group_id)) = &update.group_id {
            if self.groups.get_group(group_id)?.is_none() {
                return Err(ProjectError::GroupNotFound(group_id.clone()));
            }
        }
        project.update(update)?;
        self.repo.save(&project)?;
        Ok(project)
    }

    pub fn get(&self, id: &str) -> Result<Project, ProjectError> {
        self.load(id)
    }

    /// Non-deleted projects for the main list view, ordered per `options`.
    pub fn list(&self, options: SortOptions) -> Result<Vec<Project>, ProjectError> {
        let mut projects: Vec<Project> = self
            .repo
            .list()?
            .into_iter()
            .filter(|p| !p.is_deleted)
            .collect();
        sort_projects(&mut projects, options);
        Ok(projects)
    }

    /// Soft-deleted projects for the bin view, ordered per `options`.
    pub fn list_deleted(&self, options: SortOptions) -> Result<Vec<Project>, ProjectError> {
        Ok(filter_deleted(&self.repo.list()?, options))
    }

    /// Favorited, non-deleted projects, ordered per `options`.
    pub fn list_favorites(&self, options: SortOptions) -> Result<Vec<Project>, ProjectError> {
        let active: Vec<Project> = self
            .repo
            .list()?
            .into_iter()
            .filter(|p| !p.is_deleted)
            .collect();
        Ok(filter_favorites(&active, options))
    }

    /// IDs of live projects whose directory is no longer on disk — deleted or
    /// replaced by a file. An *inaccessible* directory (offline network
    /// drive, permissions hiccup) is deliberately not flagged.
    pub fn list_missing_directories(&self) -> Result<Vec<String>, ProjectError> {
        Ok(self
            .repo
            .list()?
            .into_iter()
            .filter(|p| !p.is_deleted)
            .filter(|p| {
                matches!(
                    check_directory_status(&p.directory),
                    DirectoryStatus::DoesNotExist | DirectoryStatus::NotADirectory
                )
            })
            .map(|p| p.id)
            .collect())
    }

    /// All-or-nothing re-detection: any detector failure is returned to the
    /// caller and the stored trackers are left untouched.
    pub fn refresh_trackers(&self, id: &str) -> Result<Project, ProjectError> {
        let mut project = self.load(id)?;
        Project::check_directory_health(&project.directory)?;
        project.trackers = self
            .detectors
            .detect_project(Path::new(&project.directory))
            .into_result()
            .map_err(|e| ProjectError::Detection(e.to_string()))?;
        self.repo.save(&project)?;
        Ok(project)
    }

    /// Runs detection against a directory that isn't a project yet — nothing
    /// is read from or written to the store. Advisory, so best-effort.
    pub fn preview_detection(&self, directory: &str) -> Vec<Tracker> {
        let detection = self.detectors.detect_project(Path::new(directory));
        for error in detection.errors() {
            eprintln!("Detector error previewing '{directory}': {error}");
        }
        detection.trackers()
    }

    /// Loads a project and runs detection against its directory **without
    /// persisting**. A missing/inaccessible directory is reported via
    /// `directory_status` (with empty `results`), not as an error.
    pub fn inspect(&self, id: &str, only: Option<&str>) -> Result<ProjectInspection, ProjectError> {
        let project = self.load(id)?;
        let (directory_status, results) = match Project::check_directory_health(&project.directory)
        {
            Ok(()) => {
                let detection = self.detectors.inspect(Path::new(&project.directory), only);
                (
                    DirectoryState {
                        ok: true,
                        message: None,
                    },
                    results_from(detection),
                )
            }
            Err(error) => (
                DirectoryState {
                    ok: false,
                    message: Some(error.to_string()),
                },
                Vec::new(),
            ),
        };
        Ok(ProjectInspection {
            project,
            directory_status,
            results,
        })
    }

    /// Permanently purges a project's metadata. Only allowed on an already
    /// soft-deleted project (from the bin).
    pub fn delete(&self, id: &str) -> Result<(), ProjectError> {
        let project = self.load(id)?;
        if !project.is_deleted {
            return Err(ProjectError::ProjectNotInBin(id.to_string()));
        }
        self.repo.delete(id)?;
        Ok(())
    }

    /// Restores a soft-deleted project so it shows up in the main list again.
    pub fn restore(&self, id: &str) -> Result<Project, ProjectError> {
        let mut project = self.load(id)?;
        project.restore();
        self.repo.save(&project)?;
        Ok(project)
    }

    /// Removes a project's tracked metadata without touching its directory on
    /// disk — "stop indexing this". Works on any project regardless of
    /// `is_deleted`.
    pub fn untrack(&self, id: &str) -> Result<(), ProjectError> {
        self.load(id)?;
        self.repo.delete(id)?;
        Ok(())
    }

    /// Deletes a project's directory from disk, then either purges its
    /// metadata (`delete_metadata: true`) or keeps it soft-deleted in the
    /// bin. The only path that removes a directory.
    pub fn delete_directory(&self, id: &str, delete_metadata: bool) -> Result<(), ProjectError> {
        let mut project = self.load(id)?;
        remove_directory(&project.directory).map_err(ProjectError::DirectoryInaccessible)?;
        if delete_metadata {
            self.repo.delete(id)?;
        } else {
            project.mark_deleted();
            self.repo.save(&project)?;
        }
        Ok(())
    }

    /// Opens a project with its stored `open_with` app (or the system default
    /// when unset), after checking the app can still be found.
    pub fn open(&self, id: &str) -> Result<Project, ProjectError> {
        let project = self.load(id)?;
        Project::check_directory_health(&project.directory)?;
        let open_with = project
            .open_with
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if let Some(command) = &open_with {
            if !self.launcher.is_available(command) {
                return Err(ProjectError::OpenWithAppMissing(command.clone()));
            }
        }
        self.open_and_mark(project, open_with.as_deref())
    }

    /// Opens a project's directory with the system's file explorer, ignoring
    /// any `open_with` app configured for it.
    pub fn open_in_explorer(&self, id: &str) -> Result<Project, ProjectError> {
        let project = self.load(id)?;
        Project::check_directory_health(&project.directory)?;
        self.open_and_mark(project, None)
    }

    fn open_and_mark(
        &self,
        mut project: Project,
        open_with: Option<&str>,
    ) -> Result<Project, ProjectError> {
        self.launcher.open(&project.directory, open_with)?;
        project.mark_as_opened_recently();
        self.repo.save(&project)?;
        Ok(project)
    }

    pub fn find_by_directory(&self, directory: &str) -> Result<Option<Project>, ProjectError> {
        let normalized = crate::domain::normalize::normalize_directory(directory);
        Ok(self.repo.find_by_directory(&normalized)?)
    }

    /// Names of every non-deleted project. The input to
    /// [`taken_names_from`](crate::domain::naming::taken_names_from) wherever a
    /// caller needs to pick a name that will survive `create`'s duplicate check.
    pub fn active_project_names(&self) -> Result<Vec<String>, ProjectError> {
        Ok(self
            .repo
            .list()?
            .into_iter()
            .filter(|p| !p.is_deleted)
            .map(|p| p.name)
            .collect())
    }

    /// Returns the project registered for `directory`, creating one if there
    /// isn't one yet. The name is inferred exactly the way the GUI's
    /// `suggest_project_name` command infers it — the git remote's repo name
    /// when the directory is a repo with a remote, otherwise the folder name —
    /// and then disambiguated against the names already in use.
    ///
    /// Checks `find_by_directory` before running detection: an already-tracked
    /// directory must resolve in a single lookup, not pay for a full detector
    /// scan whose result is thrown away. The observer CLI calls this on
    /// directories it has already registered far more often than on new ones.
    pub fn ensure_project(&self, directory: &str) -> Result<Project, ProjectError> {
        if let Some(existing) = self.find_by_directory(directory)? {
            return Ok(existing);
        }
        let trackers = self.preview_detection(directory);
        let name =
            suggest_project_name(&trackers, directory).unwrap_or_else(|| "project".to_string());
        self.ensure_project_named(directory, &name)
    }

    /// Get-or-create for `directory`, using `name` when it has to create.
    ///
    /// The scanner's commit path: the user reviewed and possibly edited that
    /// name, so it is used rather than re-derived. If it collides with a name
    /// already in use it is disambiguated — `create` would otherwise reject it
    /// with `DuplicateName`, which across a two-hundred-directory import means
    /// losing a row for a reason the user cannot act on.
    ///
    /// **Idempotent, and never a rename.** A directory already tracked comes
    /// back untouched, whatever `name` says: re-scanning a folder is a no-op,
    /// not an edit to projects the user has since renamed by hand.
    pub fn ensure_project_named(
        &self,
        directory: &str,
        name: &str,
    ) -> Result<Project, ProjectError> {
        // A soft-deleted match is not "already tracked" — `create`'s own
        // duplicate check agrees (it filters `is_deleted` too, below) — so a
        // binned-then-recreated directory falls through to a fresh `create`
        // rather than resurrecting the old, still-hidden row.
        if let Some(existing) = self.find_by_directory(directory)?.filter(|p| !p.is_deleted) {
            return Ok(existing);
        }
        let taken = taken_names_from(self.active_project_names()?);
        let parent = Path::new(directory)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");
        let name = disambiguate(name, parent, &taken);
        self.create(name, directory.to_string(), None, None)
    }
}
