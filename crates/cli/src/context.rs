//! Opens the database and builds the core services once, wired exactly as
//! `src-tauri/src/lib.rs` wires them.

use std::sync::Arc;

use anyhow::Context as _;
use indexer_core::{DetectorRunner, GroupService, ProjectService, ScanService, SqliteRepository};

use crate::confirm::Confirmer;
use crate::launcher::SystemLauncher;
use crate::paths;

/// Everything a command needs. In the TUI this will also carry the selected
/// project, which commands default to when none is given.
pub struct Context {
    pub projects: Arc<ProjectService>,
    /// Built for every run, and unused until the group commands exist. Kept
    /// here rather than created on demand so `Context` stays the one place
    /// that wires core's services, exactly as the app's `lib.rs` does.
    #[allow(dead_code)]
    pub groups: Arc<GroupService>,
    pub scan: Arc<ScanService>,
    pub confirmer: Box<dyn Confirmer>,
}

impl Context {
    pub fn open(confirmer: Box<dyn Confirmer>) -> anyhow::Result<Self> {
        let path = paths::database_path()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("could not create {}", dir.display()))?;
        }
        let repo = SqliteRepository::open(&path).with_context(|| {
            format!("failed to open the project database at {}", path.display())
        })?;

        // One repository, every port: `Arc<SqliteRepository>` coerces to each.
        let repo = Arc::new(repo);
        let detectors = Arc::new(DetectorRunner::default());
        let projects = Arc::new(ProjectService::new(
            repo.clone(),
            Arc::new(SystemLauncher),
            detectors.clone(),
            repo.clone(),
        ));
        let scan = Arc::new(ScanService::new(projects.clone(), detectors));
        let groups = Arc::new(GroupService::new(repo));

        Ok(Self {
            projects,
            groups,
            scan,
            confirmer,
        })
    }
}
