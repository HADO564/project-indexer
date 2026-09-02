use std::path::Path;
use std::sync::Arc;

use crate::application::inspection::{results_from, DirectoryState, ProjectInspection};
use crate::detectors::DetectorRunner;
use crate::domain::naming::suggest_project_name;
use crate::domain::sorting::{filter_deleted, filter_favorites, sort_projects, SortOptions};
use crate::domain::{Project, Tracker, UpdateProject};
use crate::error::ProjectError;
use crate::platform::{check_directory_status, remove_directory, DirectoryStatus};
use crate::ports::{AppLauncher, ProjectRepository};

/// All the orchestration that used to live in the Tauri command handlers:
/// one method per current command, with the logic lifted unchanged. The
/// Tauri layer becomes a thin adapter over this, and a future CLI can drive
/// the exact same flows.
pub struct ProjectService {
    repo: Arc<dyn ProjectRepository>,
    launcher: Arc<dyn AppLauncher>,
    detectors: Arc<DetectorRunner>,
}

impl ProjectService {
    pub fn new(
        repo: Arc<dyn ProjectRepository>,
        launcher: Arc<dyn AppLauncher>,
        detectors: Arc<DetectorRunner>,
    ) -> Self {
        Self {
            repo,
            launcher,
            detectors,
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

    /// Returns the project registered for `directory`, creating one (with a
    /// name suggested from the directory) if there isn't one yet.
    pub fn ensure_project(&self, directory: &str) -> Result<Project, ProjectError> {
        if let Some(existing) = self.find_by_directory(directory)? {
            return Ok(existing);
        }
        let name = suggest_project_name(&[], directory).unwrap_or_else(|| "project".to_string());
        self.create(name, directory.to_string(), None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detectors::DetectorRunner;
    use crate::infra::SqliteRepository;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FakeLauncher {
        available: bool,
        opened: Mutex<Vec<(String, Option<String>)>>,
    }
    impl AppLauncher for FakeLauncher {
        fn open(&self, dir: &str, with: Option<&str>) -> Result<(), crate::error::LauncherError> {
            self.opened
                .lock()
                .unwrap()
                .push((dir.into(), with.map(str::to_string)));
            Ok(())
        }
        fn is_available(&self, _: &str) -> bool {
            self.available
        }
    }

    fn service(launcher: Arc<FakeLauncher>) -> ProjectService {
        ProjectService::new(
            Arc::new(SqliteRepository::in_memory().unwrap()),
            launcher,
            Arc::new(DetectorRunner::default()),
        )
    }

    fn tmpdir(name: &str) -> String {
        let d = std::env::temp_dir().join(format!("pi-svc-{name}"));
        std::fs::create_dir_all(&d).unwrap();
        d.to_string_lossy().into_owned()
    }

    fn mk_update_open_with(path: &str) -> UpdateProject {
        serde_json::from_value(serde_json::json!({ "open_with": path })).unwrap()
    }

    #[test]
    fn create_then_get_and_list() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let p = svc.create("Alpha".into(), tmpdir("a"), None, None).unwrap();
        assert_eq!(svc.get(&p.id).unwrap().name, "Alpha");
        assert_eq!(svc.list(Default::default()).unwrap().len(), 1);
    }

    #[test]
    fn create_rejects_duplicate_name() {
        let svc = service(Arc::new(FakeLauncher::default()));
        svc.create("Dup".into(), tmpdir("dup1"), None, None)
            .unwrap();
        let err = svc
            .create("dup".into(), tmpdir("dup2"), None, None)
            .unwrap_err();
        assert!(matches!(err, ProjectError::DuplicateName(_)));
    }

    #[test]
    fn create_rejects_duplicate_directory() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let dir = tmpdir("samedir");
        svc.create("One".into(), dir.clone(), None, None).unwrap();
        let err = svc.create("Two".into(), dir, None, None).unwrap_err();
        assert!(matches!(err, ProjectError::DuplicateDirectory(_)));
    }

    #[test]
    fn get_unknown_is_not_found() {
        let svc = service(Arc::new(FakeLauncher::default()));
        assert!(matches!(svc.get("nope"), Err(ProjectError::NotFound(_))));
    }

    #[test]
    fn open_missing_directory_is_deleted_or_moved() {
        let svc = service(Arc::new(FakeLauncher {
            available: true,
            ..Default::default()
        }));
        let dir = tmpdir("open-gone");
        let p = svc.create("Gone".into(), dir.clone(), None, None).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(matches!(
            svc.open(&p.id),
            Err(ProjectError::DirectoryDeletedOrMoved(_))
        ));
    }

    #[test]
    fn open_with_missing_app_is_reported() {
        let launcher = Arc::new(FakeLauncher {
            available: false,
            ..Default::default()
        });
        let svc = service(launcher);
        let p = svc
            .create("App".into(), tmpdir("open-app"), None, None)
            .unwrap();
        svc.update(&p.id, mk_update_open_with("/nonexistent/editor"))
            .unwrap();
        assert!(matches!(
            svc.open(&p.id),
            Err(ProjectError::OpenWithAppMissing(_))
        ));
    }

    #[test]
    fn open_success_marks_opened_and_calls_launcher() {
        let launcher = Arc::new(FakeLauncher {
            available: true,
            ..Default::default()
        });
        let svc = service(launcher.clone());
        let dir = tmpdir("open-ok");
        let p = svc.create("OK".into(), dir.clone(), None, None).unwrap();
        let opened = svc.open(&p.id).unwrap();
        assert!(opened.last_opened_at.is_some());
        assert_eq!(launcher.opened.lock().unwrap().len(), 1);
        assert!(svc.get(&p.id).unwrap().last_opened_at.is_some());
    }

    #[test]
    fn delete_requires_bin() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let p = svc
            .create("Live".into(), tmpdir("del-live"), None, None)
            .unwrap();
        assert!(matches!(
            svc.delete(&p.id),
            Err(ProjectError::ProjectNotInBin(_))
        ));
    }

    #[test]
    fn untrack_then_recreate() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let dir = tmpdir("untrack");
        let p = svc.create("U".into(), dir.clone(), None, None).unwrap();
        svc.untrack(&p.id).unwrap();
        assert!(svc.get(&p.id).is_err());
        svc.create("U again".into(), dir, None, None).unwrap();
    }

    #[test]
    fn delete_directory_soft_keeps_record_in_bin() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let dir = tmpdir("deldir-soft");
        let p = svc.create("S".into(), dir.clone(), None, None).unwrap();
        svc.delete_directory(&p.id, false).unwrap();
        assert!(svc.get(&p.id).unwrap().is_deleted);
        assert!(!std::path::Path::new(&dir).exists());
    }

    #[test]
    fn delete_directory_hard_purges() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let dir = tmpdir("deldir-hard");
        let p = svc.create("H".into(), dir, None, None).unwrap();
        svc.delete_directory(&p.id, true).unwrap();
        assert!(matches!(svc.get(&p.id), Err(ProjectError::NotFound(_))));
    }

    #[test]
    fn refresh_all_or_nothing_leaves_stored_trackers_on_detector_failure() {
        use crate::detectors::{Detector, DetectorRunner};
        use crate::error::DetectorError;
        struct Boom;
        impl Detector for Boom {
            fn kind(&self) -> &'static str {
                "boom"
            }
            fn detect(&self, _: &std::path::Path) -> Result<Option<Tracker>, DetectorError> {
                Err(DetectorError::Other("boom".into()))
            }
        }
        let repo = Arc::new(SqliteRepository::in_memory().unwrap());
        let svc = ProjectService::new(
            repo,
            Arc::new(FakeLauncher::default()),
            Arc::new(DetectorRunner::new(vec![Box::new(Boom)])),
        );
        let p = svc
            .create("R".into(), tmpdir("refresh"), None, None)
            .unwrap();
        let before = svc.get(&p.id).unwrap().trackers.len();
        assert!(svc.refresh_trackers(&p.id).is_err());
        assert_eq!(svc.get(&p.id).unwrap().trackers.len(), before);
    }

    #[test]
    fn inspect_reports_bad_directory_without_erroring() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let dir = tmpdir("inspect-gone");
        let p = svc.create("I".into(), dir.clone(), None, None).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let ins = svc.inspect(&p.id, None).unwrap();
        assert!(!ins.directory_status.ok);
        assert!(ins.results.is_empty());
    }

    #[test]
    fn restore_clears_deleted() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let dir = tmpdir("restore");
        let p = svc.create("Rst".into(), dir.clone(), None, None).unwrap();
        svc.delete_directory(&p.id, false).unwrap();
        assert!(svc.get(&p.id).unwrap().is_deleted);
        let restored = svc.restore(&p.id).unwrap();
        assert!(!restored.is_deleted);
    }

    #[test]
    fn ensure_project_is_idempotent() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let dir = tmpdir("ensure");
        let a = svc.ensure_project(&dir).unwrap();
        let b = svc.ensure_project(&dir).unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(svc.list(Default::default()).unwrap().len(), 1);
    }
}
