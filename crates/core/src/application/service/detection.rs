//! Running detectors against a project's directory.
//!
//! The all-or-nothing contract lives here: [`ProjectService::refresh_trackers`]
//! is deliberately the one path that refuses a partial result, which is why
//! the bulk scanner does not use it.

use super::*;

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
}
