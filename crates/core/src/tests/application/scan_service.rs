//! Tests for [`crate::application::scan_service`].

use crate::application::ProjectService;
use crate::domain::scan::ScanRequest;
use std::sync::Arc;

use crate::application::scan_service::*;
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

/// The badge the review UI hangs off of: a candidate whose name collided
/// and got auto-qualified must say so, distinguishably from one the user
/// edited by hand.
#[test]
fn scan_flags_a_candidate_whose_name_was_disambiguated() {
    let root = temp_tree("disambiguated");
    repo_at(&root, "work/api");
    repo_at(&root, "code/api");
    let (_, scan) = services();

    let report = scan
        .scan(&ScanRequest {
            mode: ScanMode::Deep { depth: 2 },
            ..request(&root)
        })
        .unwrap();

    let mut by_name: Vec<(&str, bool)> = report
        .candidates
        .iter()
        .map(|c| (c.suggested_name.as_str(), c.disambiguated))
        .collect();
    by_name.sort();
    // `walk` hands candidates back sorted by directory path, so
    // "code/api" is assigned before "work/api" and keeps "api"
    // untouched; "work/api" collides and is qualified using its own
    // parent, which is `disambiguated: true`.
    assert_eq!(by_name, vec![("api", false), ("work/api", true)]);

    std::fs::remove_dir_all(&root).ok();
}

/// Binning keeps a project's metadata but the directory is no longer
/// "tracked" in any sense a fresh scan should honor — re-cloning into the
/// same path must scan and import like any other new directory, not stay
/// stuck behind the old, now-hidden row.
#[test]
fn a_binned_then_recreated_directory_is_not_already_tracked() {
    let root = temp_tree("binned");
    let api = repo_at(&root, "api");
    let (projects, scan) = services();

    let original = projects.ensure_project(&api).unwrap();
    // Bin it while keeping its metadata (soft delete: `delete_metadata:
    // false`), then simulate a re-clone into the same path.
    projects.delete_directory(&original.id, false).unwrap();
    std::fs::create_dir_all(&api).expect("recreate the directory");
    git2::Repository::init(&api).expect("re-clone should init a fresh repo");

    let report = scan.scan(&request(&root)).unwrap();
    assert_eq!(report.candidates.len(), 1);
    assert!(
        !report.candidates[0].already_tracked,
        "a binned directory must not show as already tracked"
    );

    let import_report = scan
        .import(&[ImportSelection {
            directory: api.clone(),
            name: "api".into(),
        }])
        .unwrap();
    assert_eq!(import_report.skipped, 0);
    assert_eq!(import_report.imported.len(), 1);
    assert!(!import_report.imported[0].is_deleted);
    assert_ne!(import_report.imported[0].id, original.id);

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

/// An already-tracked candidate must keep its plain name. Its own name is
/// already in the `taken` set (seeded from `active_project_names`), so
/// running it through `disambiguate` would see it collide with itself and
/// needlessly parent-qualify it — "api" would come back suggesting
/// "<parent>/api" on every rescan, which is not a real collision.
#[test]
fn rescanning_an_already_tracked_directory_keeps_its_plain_name() {
    let root = temp_tree("rescan-name");
    let api = repo_at(&root, "api");
    let (_, scan) = services();

    scan.import(&[ImportSelection {
        directory: api,
        name: "api".into(),
    }])
    .unwrap();
    let report = scan.scan(&request(&root)).unwrap();

    assert_eq!(report.candidates.len(), 1);
    assert!(report.candidates[0].already_tracked);
    assert_eq!(report.candidates[0].suggested_name, "api");

    std::fs::remove_dir_all(&root).ok();
}
