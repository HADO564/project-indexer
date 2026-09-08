//! Tests for `service/bin.rs`.

use super::*;

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
fn restore_clears_deleted() {
    let svc = service(Arc::new(FakeLauncher::default()));
    let dir = tmpdir("restore");
    let p = svc.create("Rst".into(), dir.clone(), None, None).unwrap();
    svc.delete_directory(&p.id, false).unwrap();
    assert!(svc.get(&p.id).unwrap().is_deleted);
    let restored = svc.restore(&p.id).unwrap();
    assert!(!restored.is_deleted);
}
