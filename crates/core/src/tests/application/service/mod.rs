//! Tests for [`crate::application::service`], split to match the module's
//! own files. Shared fixtures live here so each child can `use super::*`.

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

mod bin;
mod crud;
mod detection;
mod directory;
mod launching;
