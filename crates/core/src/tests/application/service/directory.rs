//! Tests for `service/directory.rs`.

use super::*;

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
