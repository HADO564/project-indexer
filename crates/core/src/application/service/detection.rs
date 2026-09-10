//! Running detectors against a project's directory.
//!
//! The all-or-nothing contract lives here: [`ProjectService::refresh_trackers`]
//! is deliberately the one path that refuses a partial result, which is why
//! the bulk scanner does not use it.

use serde::{Deserialize, Serialize};

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SweepFailure {
    pub project_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SweepReport {
    pub scanned: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failures: Vec<SweepFailure>,
}

impl ProjectService {
    /// All-or-nothing re-detection: any detector failure is returned to the
    /// caller and the stored trackers are left untouched.
    pub fn refresh_trackers(&self, id: &str) -> Result<Project, ProjectError> {
        let mut project = self.load(id)?;
        Project::check_directory_health(&project.directory)?;
        project.trackers = self
            .detectors
            .detect_project(Path::new(&project.directory))
            .into_result()
            .map_err(|e| ProjectError::Detection(e.to_string()))?;
        self.repo.save(&project)?;
        Ok(project)
    }

    /// Runs detection against a directory that isn't a project yet — nothing
    /// is read from or written to the store. Advisory, so best-effort.
    pub fn preview_detection(&self, directory: &str) -> Vec<Tracker> {
        let detection = self.detectors.detect_project(Path::new(directory));
        for error in detection.errors() {
            eprintln!("Detector error previewing '{directory}': {error}");
        }
        detection.trackers()
    }

    /// Loads a project and runs detection against its directory **without
    /// persisting**. A missing/inaccessible directory is reported via
    /// `directory_status` (with empty `results`), not as an error.
    pub fn inspect(&self, id: &str, only: Option<&str>) -> Result<ProjectInspection, ProjectError> {
        let project = self.load(id)?;
        let (directory_status, results) = match Project::check_directory_health(&project.directory)
        {
            Ok(()) => {
                let detection = self.detectors.inspect(Path::new(&project.directory), only);
                (
                    DirectoryState {
                        ok: true,
                        message: None,
                    },
                    results_from(detection),
                )
            }
            Err(error) => (
                DirectoryState {
                    ok: false,
                    message: Some(error.to_string()),
                },
                Vec::new(),
            ),
        };
        Ok(ProjectInspection {
            project,
            directory_status,
            results,
        })
    }

    /// Runs **one** detector across every live project and stores what it
    /// finds — the sweep that runs when the detector set gains a kind.
    ///
    /// Detection results are persisted, so a project registered before a
    /// detector existed carries an incomplete tracker set forever and nothing
    /// surfaces that. This is what closes that gap.
    ///
    /// **Best-effort, deliberately.** A project that fails is counted and the
    /// sweep carries on; only something that stops the sweep entirely — the
    /// store being unreadable — comes back as `Err`. That is why this uses
    /// `trackers()` + `errors()` and never [`Detection::into_result`], whose
    /// all-or-nothing contract is right for one project the user asked to
    /// refresh and catastrophic across hundreds.
    ///
    /// Projects are loaded one at a time rather than all at once, so peak
    /// memory is one project whatever the database holds.
    pub fn redetect_kind(&self, kind: &str) -> Result<SweepReport, ProjectError> {
        let ids = self.repo.list_ids()?;
        // Built in one go rather than assigning after `default()`, which clippy
        // flags as `field_reassign_with_default`.
        let mut report = SweepReport {
            scanned: ids.len(),
            ..Default::default()
        };

        for id in ids {
            // Gone between listing and now — not a failure, just nothing to do.
            let mut project = match self.load(&id) {
                Ok(p) => p,
                Err(_) => {
                    report.skipped += 1;
                    continue;
                }
            };

            // Get out fast on an unreachable path. One project on a
            // disconnected network drive must not stall the whole sweep.
            if Project::check_directory_health(&project.directory).is_err() {
                report.skipped += 1;
                continue;
            }

            // `Some(kind)` — only the one new detector, not the whole set.
            let detection = self
                .detectors
                .inspect(Path::new(&project.directory), Some(kind));

            let errors = detection.errors();
            if !errors.is_empty() {
                report.failures.push(SweepFailure {
                    project_id: project.id.clone(),
                    message: errors
                        .iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("; "),
                });
                continue;
            }

            // At most one tracker comes back, since only one detector ran.
            let found = detection.trackers().into_iter().next();
            let existing = project.trackers.iter().find(|t| t.is(kind));

            // Saving unconditionally would bump `updated_at` on every project
            // on every sweep, silently reordering anyone's recently-updated
            // sort. Most projects will not match a newly added detector, so
            // most iterations stop here.
            let changed = match (existing, &found) {
                (None, None) => false,
                (Some(old), Some(new)) => !same_tracker(old, new),
                _ => true,
            };
            if !changed {
                continue;
            }

            // Replace this kind's entry and leave every other kind alone.
            // Assigning `project.trackers = detection.trackers()` instead would
            // delete the git tracker while adding the new one.
            project.trackers.retain(|t| !t.is(kind));
            if let Some(tracker) = found {
                project.trackers.push(tracker);
            }

            self.repo.save(&project)?;
            report.updated += 1;
        }

        Ok(report)
    }
}

/// Value equality for two trackers of the same kind.
///
/// Compares the serialised form rather than requiring `PartialEq`, so a new
/// detector's info struct does not have to derive anything extra — keeping
/// "adding a detector is a folder plus two lines" true.
fn same_tracker(a: &Tracker, b: &Tracker) -> bool {
    match (serde_json::to_value(a), serde_json::to_value(b)) {
        (Ok(a), Ok(b)) => a == b,
        // A tracker that will not serialise cannot be compared; treat it as
        // changed so the fresh value is what gets written.
        _ => false,
    }
}
