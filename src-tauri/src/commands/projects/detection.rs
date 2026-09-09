//! Re-running detectors against a project, and previewing a directory.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

use indexer_core::application::ProjectService;
use indexer_core::domain::{Project, Tracker};
use indexer_core::error::ProjectError;

/// Re-runs project-type detection (git, and whatever else is registered)
/// against an existing project's directory and persists the result.
///
/// Unlike the best-effort detection in [`create_project`], this is
/// all-or-nothing: any detector failure is returned to the caller and the
/// stored trackers are left untouched. It's an explicit, user-triggered
/// retry, so a half-applied refresh — a persisted tracker set silently
/// missing whatever the failing detector produces — is worse than a visible
/// failure. This is a recorded decision, not incidental; the alternative
/// (persist successes, surface per-detector errors) is documented in
/// `docs/architecture.md` and guarded by a runner test.
#[tauri::command]
pub fn refresh_project_trackers(
    service: State<'_, Arc<ProjectService>>,
    id: String,
) -> Result<Project, ProjectError> {
    service.refresh_trackers(&id)
}

static SWEEP_RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SweepFinished {
    kinds: Vec<String>,
    scanned: usize,
    updated: usize,
    failures: usize,
    error: Option<String>,
}

pub(crate) fn spawn_sweep(
    app: AppHandle,
    service: Arc<ProjectService>,
    kinds: Vec<String>,
) -> bool {
    if SWEEP_RUNNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return false;
    }

    std::thread::spawn(move || {
        let _guard = SweepGuard;

        let mut scanned = 0;
        let mut updated = 0;
        let mut failures = 0;
        let mut error = None;

        for kind in &kinds {
            match service.redetect_kind(kind) {
                Ok(report) => {
                    scanned += report.scanned;
                    updated += report.updated;
                    failures += report.failures.len();
                }
                Err(e) => {
                    error = Some(e.to_string());
                    break;
                }
            }
        }

        let _ = app.emit(
            "sweep://done",
            SweepFinished {
                kinds,
                scanned,
                updated,
                failures,
                error,
            },
        );
    });

    true
}

#[tauri::command]
pub fn redetect_kind(
    app: AppHandle,
    service: State<'_, Arc<ProjectService>>,
    kind: String,
) -> bool {
    spawn_sweep(app, service.inner().clone(), vec![kind])
}

struct SweepGuard;

impl Drop for SweepGuard {
    fn drop(&mut self) {
        SWEEP_RUNNING.store(false, Ordering::SeqCst);
    }
}

/// Runs detection against a directory that isn't a project yet — nothing is
/// read from or written to the store. Lets the frontend preview what a
/// directory looks like (e.g. to suggest a name from its git remote) before
/// the user commits to [`create_project`].
///
/// Advisory, so it's best-effort: a detector that fails just contributes
/// nothing to the preview rather than failing the whole call.
#[tauri::command]
pub fn detect_project_trackers(
    service: State<'_, Arc<ProjectService>>,
    directory: String,
) -> Vec<Tracker> {
    service.preview_detection(&directory)
}
