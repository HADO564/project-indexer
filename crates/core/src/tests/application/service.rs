//! Tests for [`crate::application::service`].

use crate::domain::{Tracker, UpdateProject};
use crate::error::ProjectError;
use crate::ports::AppLauncher;

use crate::application::service::*;
use crate::application::GroupService;
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
    let repo = Arc::new(SqliteRepository::in_memory().unwrap());
    ProjectService::new(
        repo.clone(),
        launcher,
        Arc::new(DetectorRunner::default()),
        repo,
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
        repo.clone(),
        Arc::new(FakeLauncher::default()),
        Arc::new(DetectorRunner::new(vec![Box::new(Boom)])),
        repo,
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

/// Regression: `ensure_project` must resolve an already-tracked directory
/// from `find_by_directory` alone. It previously ran full detection first
/// and only then checked whether the directory was already registered —
/// the detection result was thrown away every time `ensure_project_named`
/// found an existing row, which is the common case for the observer CLI
/// re-polling directories it already knows about.
#[test]
fn ensure_project_skips_detection_for_an_already_tracked_directory() {
    use crate::detectors::{Detector, DetectorRunner};
    use crate::error::DetectorError;

    // Counts invocations rather than panicking/erroring: a clean count of
    // zero is unambiguous proof detection never ran, and doesn't rely on
    // catching an unwind.
    struct CountingDetector {
        calls: Arc<Mutex<u32>>,
    }
    impl Detector for CountingDetector {
        fn kind(&self) -> &'static str {
            "counting"
        }
        fn detect(&self, _: &std::path::Path) -> Result<Option<Tracker>, DetectorError> {
            *self.calls.lock().unwrap() += 1;
            Ok(None)
        }
    }

    let repo = Arc::new(SqliteRepository::in_memory().unwrap());
    let launcher = Arc::new(FakeLauncher::default());
    let dir = tmpdir("ensure-skip-detect");

    // Register the directory with an empty detector set, so setup itself
    // doesn't touch the counter.
    let setup = ProjectService::new(
        repo.clone(),
        launcher.clone(),
        Arc::new(DetectorRunner::new(vec![])),
        repo.clone(),
    );
    let first = setup.ensure_project(&dir).unwrap();

    // A second service over the same repo, wired with a detector that
    // records every call it gets.
    let calls = Arc::new(Mutex::new(0));
    let svc = ProjectService::new(
        repo.clone(),
        launcher,
        Arc::new(DetectorRunner::new(vec![Box::new(CountingDetector {
            calls: calls.clone(),
        })])),
        repo,
    );

    let second = svc.ensure_project(&dir).unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(
        *calls.lock().unwrap(),
        0,
        "ensure_project ran detection on an already-tracked directory"
    );
}

/// The bug this fixes: two `api` directories under different parents both
/// have to register. Before `disambiguate`, the second returned
/// `DuplicateName` and the observer CLI simply failed on it.
/// Note the expectation is derived, not hardcoded: `tmpdir` prefixes its
/// argument (`pi-svc-collide-work`), and that prefix is the parent folder
/// name `disambiguate` qualifies with. Spelling the prefix into the
/// assertion would couple this test to the helper's naming.
#[test]
fn ensure_project_disambiguates_a_colliding_name() {
    let svc = service(Arc::new(FakeLauncher::default()));
    let code = tmpdir("collide-code");
    let work = tmpdir("collide-work");
    let a = std::path::Path::new(&code).join("api");
    let b = std::path::Path::new(&work).join("api");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();

    let first = svc.ensure_project(a.to_str().unwrap()).unwrap();
    let second = svc.ensure_project(b.to_str().unwrap()).unwrap();

    let work_folder = std::path::Path::new(&work)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert_eq!(first.name, "api");
    assert_eq!(second.name, format!("{work_folder}/api"));
    assert_ne!(first.id, second.id);
}

#[test]
fn ensure_project_named_uses_the_given_name() {
    let svc = service(Arc::new(FakeLauncher::default()));
    let dir = tmpdir("named");

    let project = svc.ensure_project_named(&dir, "Chosen Name").unwrap();

    // `Project::new` runs every name through `remove_spaces`
    // (crates/core/src/domain/project.rs), so the space survives as an
    // underscore — the same normalization `create` already applies to
    // any caller-supplied name, not something `ensure_project_named`
    // introduces.
    assert_eq!(project.name, "Chosen_Name");
}

/// Get-or-create: a directory already tracked comes back as-is, and the
/// supplied name does not rename it. Re-scanning a folder must be a no-op,
/// not an edit.
#[test]
fn ensure_project_named_is_idempotent_and_does_not_rename() {
    let svc = service(Arc::new(FakeLauncher::default()));
    let dir = tmpdir("named-idempotent");

    let first = svc.ensure_project_named(&dir, "First").unwrap();
    let second = svc.ensure_project_named(&dir, "Second").unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(second.name, "First");
    assert_eq!(svc.list(Default::default()).unwrap().len(), 1);
}

#[test]
fn ensure_project_named_disambiguates_a_colliding_name() {
    let svc = service(Arc::new(FakeLauncher::default()));
    let first_dir = tmpdir("named-collide-a");
    let parent = tmpdir("clients");
    let second_dir = std::path::Path::new(&parent).join("api");
    std::fs::create_dir_all(&second_dir).unwrap();

    svc.ensure_project_named(&first_dir, "api").unwrap();
    let second = svc
        .ensure_project_named(second_dir.to_str().unwrap(), "api")
        .unwrap();

    // Derived, not hardcoded — `tmpdir` prefixes its argument, and that
    // prefixed folder name is what the qualifier uses.
    let parent_folder = std::path::Path::new(&parent)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert_eq!(second.name, format!("{parent_folder}/api"));
}

fn mk_update_group_id(id: Option<&str>) -> UpdateProject {
    serde_json::from_value(serde_json::json!({ "group_id": id })).unwrap()
}

#[test]
fn update_assigns_project_to_an_existing_group() {
    let repo = Arc::new(SqliteRepository::in_memory().unwrap());
    let groups = GroupService::new(repo.clone());
    let svc = ProjectService::new(
        repo.clone(),
        Arc::new(FakeLauncher::default()),
        Arc::new(DetectorRunner::default()),
        repo,
    );
    let group = groups
        .create("Work".into(), "cyan".into(), "briefcase".into())
        .unwrap();
    let p = svc
        .create("Assignable".into(), tmpdir("group-assign"), None, None)
        .unwrap();

    let updated = svc
        .update(&p.id, mk_update_group_id(Some(&group.id)))
        .unwrap();

    assert_eq!(updated.group_id.as_deref(), Some(group.id.as_str()));
    assert_eq!(
        svc.get(&p.id).unwrap().group_id.as_deref(),
        Some(group.id.as_str())
    );
}

#[test]
fn update_rejects_assignment_to_a_nonexistent_group_and_leaves_project_unchanged() {
    let svc = service(Arc::new(FakeLauncher::default()));
    let p = svc
        .create("Untouched".into(), tmpdir("group-missing"), None, None)
        .unwrap();
    let before = svc.get(&p.id).unwrap();

    let err = svc.update(&p.id, mk_update_group_id(Some("no-such-group")));

    assert!(matches!(err, Err(ProjectError::GroupNotFound(id)) if id == "no-such-group"));
    let after = svc.get(&p.id).unwrap();
    assert_eq!(after.group_id, before.group_id);
    assert_eq!(after.updated_at, before.updated_at);
}

#[test]
fn update_clears_group_without_needing_one_to_exist() {
    let repo = Arc::new(SqliteRepository::in_memory().unwrap());
    let groups = GroupService::new(repo.clone());
    let svc = ProjectService::new(
        repo.clone(),
        Arc::new(FakeLauncher::default()),
        Arc::new(DetectorRunner::default()),
        repo,
    );
    let group = groups
        .create("Personal".into(), "gold".into(), "star".into())
        .unwrap();
    let p = svc
        .create("Clearable".into(), tmpdir("group-clear"), None, None)
        .unwrap();
    svc.update(&p.id, mk_update_group_id(Some(&group.id)))
        .unwrap();

    let cleared = svc.update(&p.id, mk_update_group_id(None)).unwrap();

    assert!(cleared.group_id.is_none());
    assert!(svc.get(&p.id).unwrap().group_id.is_none());
}
