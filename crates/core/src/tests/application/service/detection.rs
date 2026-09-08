//! Tests for `service/detection.rs`.

use super::*;

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
