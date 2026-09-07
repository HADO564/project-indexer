use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::application::ProjectService;
use crate::detectors::DetectorRunner;
use crate::domain::naming::{disambiguate, taken_names_from};
// `ScanMode` is deliberately not imported here — the service never inspects
// the mode, it just hands the request to the walk. The test module imports it
// itself.
use crate::domain::scan::{walk, ScanReport, ScanRequest};
use crate::domain::Project;
use crate::error::ProjectError;
use std::path::Path;

/// One reviewed row on its way into the store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSelection {
    pub directory: String,
    /// The name as the user left it in the review list — used rather than
    /// re-derived, because they may well have edited it.
    pub name: String,
}

/// Why one row could not be imported. Carried rather than returned as an
/// error, so the other rows still land.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportFailure {
    pub directory: String,
    pub message: String,
}

/// What one bulk import did.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReport {
    pub imported: Vec<Project>,
    /// Rows whose directory was already tracked. Not a failure — re-importing
    /// a folder is a no-op by design.
    pub skipped: usize,
    pub failures: Vec<ImportFailure>,
}

/// Bulk import: walk a folder, review what it found, register the selection.
///
/// Separate from [`ProjectService`] rather than more methods on it — that file
/// is already the largest in `core`, and bulk-import orchestration is a
/// different concern from the per-project surface. It also keeps the walk
/// testable without a database.
///
/// **User-triggered, bounded and finite.** Nothing here runs unprompted; there
/// is no timer, no watcher and no background task. "Autorunner" names *this*,
/// and the word "background" is reserved for a feature that does not exist.
pub struct ScanService {
    projects: Arc<ProjectService>,
    detectors: Arc<DetectorRunner>,
}

impl std::fmt::Debug for ScanService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScanService").finish_non_exhaustive()
    }
}

impl ScanService {
    pub fn new(projects: Arc<ProjectService>, detectors: Arc<DetectorRunner>) -> Self {
        Self {
            projects,
            detectors,
        }
    }

    /// Walks `request` and returns what it found, with names already resolved
    /// against the projects that exist and against each other, and
    /// already-tracked directories flagged.
    ///
    /// Names are assigned here rather than at import so the review list shows
    /// exactly what will be created — a rename appearing only after the user
    /// pressed Import would be a surprise.
    pub fn scan(&self, request: &ScanRequest) -> Result<ScanReport, ProjectError> {
        let mut report = walk(request, &self.detectors);

        let mut taken = taken_names_from(self.projects.active_project_names()?);
        for candidate in &mut report.candidates {
            candidate.already_tracked = self
                .projects
                .find_by_directory(&candidate.directory)?
                .is_some();

            let parent = Path::new(&candidate.directory)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("");
            let name = disambiguate(&candidate.suggested_name, parent, &taken);
            // Reserve it, so two candidates in one report cannot collide with
            // each other — the case `~/code/api` and `~/work/api` hits on the
            // very first scan.
            taken.insert(name.to_lowercase());
            candidate.suggested_name = name;
        }

        Ok(report)
    }

    /// Registers `selections`, best-effort.
    ///
    /// **Per-row failures are collected, never fatal.** A corrupt repository at
    /// row 47 must not cost rows 48 through 200 — which is also why this goes
    /// through `ensure_project_named` (get-or-create, best-effort detection)
    /// and never `refresh_trackers`, whose all-or-nothing `into_result()`
    /// would discard an entire sweep over one bad directory.
    pub fn import(&self, selections: &[ImportSelection]) -> Result<ImportReport, ProjectError> {
        let mut imported = Vec::new();
        let mut skipped = 0usize;
        let mut failures = Vec::new();

        for selection in selections {
            let already_tracked = match self.projects.find_by_directory(&selection.directory) {
                Ok(existing) => existing.is_some(),
                Err(e) => {
                    failures.push(ImportFailure {
                        directory: selection.directory.clone(),
                        message: e.to_string(),
                    });
                    continue;
                }
            };
            if already_tracked {
                skipped += 1;
                continue;
            }

            match self
                .projects
                .ensure_project_named(&selection.directory, &selection.name)
            {
                Ok(project) => imported.push(project),
                Err(e) => failures.push(ImportFailure {
                    directory: selection.directory.clone(),
                    message: e.to_string(),
                }),
            }
        }

        Ok(ImportReport {
            imported,
            skipped,
            failures,
        })
    }

    /// The detector kinds this build registers, for the scan form's tick-list.
    ///
    /// Exists so shipping a new detector needs no frontend edit — invariant 1,
    /// "implement + register, zero frontend code". A hardcoded array of names
    /// in a Svelte file would break it the day Unity lands.
    pub fn detector_kinds(&self) -> Vec<String> {
        self.detectors.kinds()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detectors::DetectorRunner;
    use crate::domain::scan::ScanMode;
    use crate::error::LauncherError;
    use crate::infra::SqliteRepository;
    use crate::ports::AppLauncher;
    use std::path::{Path, PathBuf};

    struct NoLauncher;
    impl AppLauncher for NoLauncher {
        fn open(&self, _: &str, _: Option<&str>) -> Result<(), LauncherError> {
            Ok(())
        }
        fn is_available(&self, _: &str) -> bool {
            false
        }
    }

    fn services() -> (Arc<ProjectService>, ScanService) {
        let repo = Arc::new(SqliteRepository::in_memory().unwrap());
        let detectors = Arc::new(DetectorRunner::default());
        let projects = Arc::new(ProjectService::new(
            repo.clone(),
            Arc::new(NoLauncher),
            detectors.clone(),
            repo,
        ));
        let scan = ScanService::new(projects.clone(), detectors);
        (projects, scan)
    }

    fn temp_tree(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("project-indexer-tests-scansvc-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("should create temp dir");
        dir
    }

    fn repo_at(root: &Path, relative: &str) -> String {
        let path = root.join(relative);
        std::fs::create_dir_all(&path).expect("should create dir");
        git2::Repository::init(&path).expect("should init a git repo");
        path.to_string_lossy().to_string()
    }

    fn request(root: &Path) -> ScanRequest {
        ScanRequest {
            scan_root: root.to_string_lossy().to_string(),
            mode: ScanMode::Quick,
            detectors: vec!["git".to_string()],
            include_ignored: false,
        }
    }

    #[test]
    fn scan_assigns_collision_free_names_within_one_report() {
        let root = temp_tree("collide");
        repo_at(&root, "work/api");
        repo_at(&root, "code/api");
        let (_, scan) = services();

        let report = scan
            .scan(&ScanRequest {
                mode: ScanMode::Deep { depth: 2 },
                ..request(&root)
            })
            .unwrap();

        let mut names: Vec<String> = report
            .candidates
            .iter()
            .map(|c| c.suggested_name.clone())
            .collect();
        names.sort();
        assert_eq!(names, vec!["api".to_string(), "work/api".to_string()]);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn scan_flags_directories_that_are_already_tracked() {
        let root = temp_tree("tracked");
        let api = repo_at(&root, "api");
        let (projects, scan) = services();
        projects.ensure_project(&api).unwrap();

        let report = scan.scan(&request(&root)).unwrap();

        assert_eq!(report.candidates.len(), 1);
        assert!(report.candidates[0].already_tracked);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn import_registers_the_selected_directories() {
        let root = temp_tree("import");
        let api = repo_at(&root, "api");
        let site = repo_at(&root, "site");
        let (projects, scan) = services();

        let report = scan
            .import(&[
                ImportSelection {
                    directory: api,
                    name: "api".into(),
                },
                ImportSelection {
                    directory: site,
                    name: "site".into(),
                },
            ])
            .unwrap();

        assert_eq!(report.imported.len(), 2);
        assert!(report.failures.is_empty());
        assert_eq!(report.skipped, 0);
        assert_eq!(projects.list(Default::default()).unwrap().len(), 2);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn import_counts_an_already_tracked_directory_as_skipped() {
        let root = temp_tree("import-skip");
        let api = repo_at(&root, "api");
        let (projects, scan) = services();
        projects.ensure_project(&api).unwrap();

        let report = scan
            .import(&[ImportSelection {
                directory: api,
                name: "api".into(),
            }])
            .unwrap();

        assert_eq!(report.skipped, 1);
        assert!(report.imported.is_empty());
        assert_eq!(projects.list(Default::default()).unwrap().len(), 1);

        std::fs::remove_dir_all(&root).ok();
    }

    /// The rule that makes a bulk import usable: a row that cannot be
    /// registered costs that row and nothing else. Across two hundred
    /// directories, one bad path must not discard the other 199.
    #[test]
    fn a_failing_row_does_not_stop_the_rest() {
        let root = temp_tree("import-partial");
        let api = repo_at(&root, "api");
        let (projects, scan) = services();

        let report = scan
            .import(&[
                ImportSelection {
                    directory: String::new(),
                    name: "broken".into(),
                },
                ImportSelection {
                    directory: api,
                    name: "api".into(),
                },
            ])
            .unwrap();

        assert_eq!(report.imported.len(), 1);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(projects.list(Default::default()).unwrap().len(), 1);

        std::fs::remove_dir_all(&root).ok();
    }

    /// A rescan after committing is the "what is new since last time" case.
    #[test]
    fn rescanning_after_an_import_reports_everything_as_tracked() {
        let root = temp_tree("rescan");
        let api = repo_at(&root, "api");
        let (_, scan) = services();

        scan.import(&[ImportSelection {
            directory: api,
            name: "api".into(),
        }])
        .unwrap();
        let report = scan.scan(&request(&root)).unwrap();

        assert!(report.candidates.iter().all(|c| c.already_tracked));

        std::fs::remove_dir_all(&root).ok();
    }
}
