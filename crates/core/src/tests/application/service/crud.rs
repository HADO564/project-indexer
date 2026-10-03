//! Tests for `service/crud.rs`.

use super::*;

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

// Moving a project with `update`: one folder, one project, and the trackers
// follow the folder.

fn mk_update_directory(path: &str) -> UpdateProject {
    UpdateProject {
        directory: Some(path.to_string()),
        ..Default::default()
    }
}

/// Finds git only in folders whose name ends in `-git`, so a move between
/// two folders shows whether detection ran on the new one.
struct GitWhereNamed;
impl crate::detectors::Detector for GitWhereNamed {
    fn kind(&self) -> &'static str {
        "git"
    }
    fn detect(
        &self,
        path: &std::path::Path,
    ) -> Result<Option<Tracker>, crate::error::DetectorError> {
        if !path.to_string_lossy().ends_with("-git") {
            return Ok(None);
        }
        Ok(Some(Tracker::Git(crate::detectors::git::GitInfo {
            repo_root: path.display().to_string(),
            dirty: false,
            detached_head: false,
            repo_url: None,
            web_url: None,
            contributors: Vec::new(),
            curr_branch: Some("main".into()),
            branches: None,
            commit_hash: None,
        })))
    }
}

fn service_finding_git_where_named() -> ProjectService {
    let repo = Arc::new(SqliteRepository::in_memory().unwrap());
    ProjectService::new(
        repo.clone(),
        Arc::new(FakeLauncher::default()),
        Arc::new(DetectorRunner::new(vec![Box::new(GitWhereNamed)])),
        repo,
    )
}

#[test]
fn update_moves_a_project_to_an_existing_folder() {
    let svc = service(Arc::new(FakeLauncher::default()));
    let p = svc
        .create("Mover".into(), tmpdir("move-from"), None, None)
        .unwrap();
    let to = tmpdir("move-to");
    let moved = svc.update(&p.id, mk_update_directory(&to)).unwrap();
    assert_eq!(moved.directory, to);
    assert_eq!(svc.get(&p.id).unwrap().directory, to);
}

#[test]
fn update_refuses_a_folder_another_project_has_and_changes_nothing() {
    let svc = service(Arc::new(FakeLauncher::default()));
    let taken = tmpdir("move-taken");
    svc.create("Owner".into(), taken.clone(), None, None)
        .unwrap();
    let p = svc
        .create("Mover".into(), tmpdir("move-blocked"), None, None)
        .unwrap();
    let before = svc.get(&p.id).unwrap();

    let err = svc.update(&p.id, mk_update_directory(&taken)).unwrap_err();

    assert!(matches!(err, ProjectError::DuplicateDirectory(_)));
    let after = svc.get(&p.id).unwrap();
    assert_eq!(after.directory, before.directory);
    assert_eq!(after.updated_at, before.updated_at);
}

#[test]
fn update_may_take_a_folder_only_a_binned_project_had() {
    // `create` ignores the bin too: a binned project's folder is gone, so
    // whatever is there now is free to track.
    use crate::ports::ProjectRepository;
    let repo = Arc::new(SqliteRepository::in_memory().unwrap());
    let svc = ProjectService::new(
        repo.clone(),
        Arc::new(FakeLauncher::default()),
        Arc::new(DetectorRunner::default()),
        repo.clone(),
    );
    let dir = tmpdir("move-binned");
    let mut old = svc.create("Old".into(), dir.clone(), None, None).unwrap();
    old.mark_deleted();
    repo.save(&old).unwrap();
    let p = svc
        .create("Mover".into(), tmpdir("move-to-binned"), None, None)
        .unwrap();
    assert_eq!(
        svc.update(&p.id, mk_update_directory(&dir))
            .unwrap()
            .directory,
        dir
    );
}

#[test]
fn update_to_the_same_folder_is_not_a_move() {
    // Its own folder is not "taken by another project", and nothing is
    // re-detected.
    let svc = service_finding_git_where_named();
    let dir = tmpdir("stay-git");
    let p = svc
        .create("Stayer".into(), dir.clone(), None, None)
        .unwrap();
    assert_eq!(p.trackers.len(), 1);
    let same = svc.update(&p.id, mk_update_directory(&dir)).unwrap();
    assert_eq!(same.directory, dir);
    let kinds = |trackers: &[Tracker]| trackers.iter().map(Tracker::kind).collect::<Vec<_>>();
    assert_eq!(kinds(&same.trackers), kinds(&p.trackers));
}

#[test]
fn update_re_detects_the_trackers_of_a_moved_project() {
    let svc = service_finding_git_where_named();
    let p = svc
        .create("Plain".into(), tmpdir("move-plain"), None, None)
        .unwrap();
    assert!(p.trackers.is_empty());

    let to = tmpdir("move-into-git");
    let moved = svc.update(&p.id, mk_update_directory(&to)).unwrap();
    assert_eq!(moved.trackers.len(), 1, "detected in the new folder");
    assert_eq!(moved.trackers[0].kind(), "Git");

    let back = svc
        .update(&p.id, mk_update_directory(&tmpdir("move-plain")))
        .unwrap();
    assert!(
        back.trackers.is_empty(),
        "the old folder's trackers do not linger"
    );
}

#[test]
fn update_refuses_a_folder_that_does_not_exist() {
    let svc = service(Arc::new(FakeLauncher::default()));
    let p = svc
        .create("Mover".into(), tmpdir("move-nowhere"), None, None)
        .unwrap();
    let missing = std::env::temp_dir().join("pi-svc-there-is-no-such-folder");
    let err = svc
        .update(&p.id, mk_update_directory(&missing.to_string_lossy()))
        .unwrap_err();
    assert!(matches!(err, ProjectError::InvalidDirectory(_)));
}
