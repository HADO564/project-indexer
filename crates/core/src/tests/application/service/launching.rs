//! Tests for `service/launching.rs`.

use super::*;

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
