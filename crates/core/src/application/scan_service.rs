use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::application::ProjectService;
use crate::detectors::DetectorRunner;
use crate::domain::naming::{disambiguate, taken_names_from};
// `ScanMode` is deliberately not imported here — the service never inspects
// the mode, it just hands the request to the walk. The test module imports it
// itself.
use crate::domain::scan::{walk, ScanReport, ScanRequest};
use crate::domain::Project;
use crate::error::ProjectError;
use std::path::Path;

/// One reviewed row on its way into the store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSelection {
    pub directory: String,
    /// The name as the user left it in the review list — used rather than
    /// re-derived, because they may well have edited it.
    pub name: String,
}

/// Why one row could not be imported. Carried rather than returned as an
/// error, so the other rows still land.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportFailure {
    pub directory: String,
    pub message: String,
}

/// What one bulk import did.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReport {
    pub imported: Vec<Project>,
    /// Rows whose directory was already tracked. Not a failure — re-importing
    /// a folder is a no-op by design.
    pub skipped: usize,
    pub failures: Vec<ImportFailure>,
}

/// Bulk import: walk a folder, review what it found, register the selection.
///
/// Separate from [`ProjectService`] rather than more methods on it — that file
/// is already the largest in `core`, and bulk-import orchestration is a
/// different concern from the per-project surface. It also keeps the walk
/// testable without a database.
///
/// **User-triggered, bounded and finite.** Nothing here runs unprompted; there
/// is no timer, no watcher and no background task. "Autorunner" names *this*,
/// and the word "background" is reserved for a feature that does not exist.
pub struct ScanService {
    projects: Arc<ProjectService>,
    detectors: Arc<DetectorRunner>,
}

impl std::fmt::Debug for ScanService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScanService").finish_non_exhaustive()
    }
}

impl ScanService {
    pub fn new(projects: Arc<ProjectService>, detectors: Arc<DetectorRunner>) -> Self {
        Self {
            projects,
            detectors,
        }
    }

    /// Walks `request` and returns what it found, with names already resolved
    /// against the projects that exist and against each other, and
    /// already-tracked directories flagged.
    ///
    /// Names are assigned here rather than at import so the review list shows
    /// exactly what will be created — a rename appearing only after the user
    /// pressed Import would be a surprise.
    pub fn scan(&self, request: &ScanRequest) -> Result<ScanReport, ProjectError> {
        let mut report = walk(request, &self.detectors);

        let mut taken = taken_names_from(self.projects.active_project_names()?);
        for candidate in &mut report.candidates {
            // A soft-deleted (binned) row is not "tracked" — `create`'s own
            // duplicate check already ignores those, and a scan that disagreed
            // would show the directory as tracked, grey out its checkbox, and
            // leave a re-cloned repo impossible to re-import.
            candidate.already_tracked = self
                .projects
                .find_by_directory(&candidate.directory)?
                .filter(|p| !p.is_deleted)
                .is_some();

            // An already-tracked directory keeps its walk-produced name as-is:
            // `taken` already holds its current name (seeded from
            // `active_project_names`), so running it through `disambiguate`
            // would see it collide with itself and needlessly qualify it —
            // "api" would come back suggesting "<parent>/api" on every rescan.
            if candidate.already_tracked {
                continue;
            }

            let parent = Path::new(&candidate.directory)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("");
            let name = disambiguate(&candidate.suggested_name, parent, &taken);
            candidate.disambiguated = name != candidate.suggested_name;
            // Reserve it, so two candidates in one report cannot collide with
            // each other — the case `~/code/api` and `~/work/api` hits on the
            // very first scan.
            taken.insert(name.to_lowercase());
            candidate.suggested_name = name;
        }

        Ok(report)
    }

    /// Registers `selections`, best-effort.
    ///
    /// **Per-row failures are collected, never fatal.** A corrupt repository at
    /// row 47 must not cost rows 48 through 200 — which is also why this goes
    /// through `ensure_project_named` (get-or-create, best-effort detection)
    /// and never `refresh_trackers`, whose all-or-nothing `into_result()`
    /// would discard an entire sweep over one bad directory.
    pub fn import(&self, selections: &[ImportSelection]) -> Result<ImportReport, ProjectError> {
        let mut imported = Vec::new();
        let mut skipped = 0usize;
        let mut failures = Vec::new();

        for selection in selections {
            // Same rule as `scan`: a soft-deleted row must not count as
            // tracked, or a re-cloned, re-scanned directory would be silently
            // counted as skipped instead of imported.
            let already_tracked = match self.projects.find_by_directory(&selection.directory) {
                Ok(existing) => existing.filter(|p| !p.is_deleted).is_some(),
                Err(e) => {
                    failures.push(ImportFailure {
                        directory: selection.directory.clone(),
                        message: e.to_string(),
                    });
                    continue;
                }
            };
            if already_tracked {
                skipped += 1;
                continue;
            }

            match self
                .projects
                .ensure_project_named(&selection.directory, &selection.name)
            {
                Ok(project) => imported.push(project),
                Err(e) => failures.push(ImportFailure {
                    directory: selection.directory.clone(),
                    message: e.to_string(),
                }),
            }
        }

        Ok(ImportReport {
            imported,
            skipped,
            failures,
        })
    }

    /// The detector kinds this build registers, for the scan form's tick-list.
    ///
    /// Exists so shipping a new detector needs no frontend edit — invariant 1,
    /// "implement + register, zero frontend code". A hardcoded array of names
    /// in a Svelte file would break it the day Unity lands.
    pub fn detector_kinds(&self) -> Vec<String> {
        self.detectors.kinds()
    }
}
