//! Registering, reading back and mutating project records.
//!
//! `ensure_project*` is the get-or-create entry point the folder scanner and
//! the observer CLI both commit through; `create` is the one that enforces the
//! duplicate-name and duplicate-directory rules.

use super::*;

impl ProjectService {
    /// Best-effort: a project is still worth tracking even if we can't tell
    /// what kind of project it is. Detection is resilient — whatever
    /// detectors succeeded still count — and `refresh_trackers` lets the
    /// caller retry the rest explicitly, where a failure is worth surfacing.
    pub fn create(
        &self,
        name: String,
        directory: String,
        description: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<Project, ProjectError> {
        let existing_active: Vec<Project> = self
            .repo
            .list()?
            .into_iter()
            .filter(|p| !p.is_deleted)
            .collect();
        Project::check_for_duplicate_name_or_dir(&name, &directory, &existing_active)?;

        let mut project = Project::new(name, directory, description, tags)?;
        let detection = self.detectors.detect_project(Path::new(&project.directory));
        project.trackers = detection.trackers();
        for error in detection.errors() {
            eprintln!("Detector error for '{}': {}", project.directory, error);
        }
        self.repo.save(&project)?;
        Ok(project)
    }

    pub fn update(&self, id: &str, update: UpdateProject) -> Result<Project, ProjectError> {
        let mut project = self.load(id)?;
        // Checked here rather than in `Project::update` because it needs the
        // group store — a foreign key the DB would otherwise reject only
        // after the rest of the update was already applied in memory.
        // `Some(None)` (clearing the group) needs no check.
        if let Some(Some(group_id)) = &update.group_id {
            if self.groups.get_group(group_id)?.is_none() {
                return Err(ProjectError::GroupNotFound(group_id.clone()));
            }
        }
        project.update(update)?;
        self.repo.save(&project)?;
        Ok(project)
    }

    pub fn get(&self, id: &str) -> Result<Project, ProjectError> {
        self.load(id)
    }

    pub fn find_by_directory(&self, directory: &str) -> Result<Option<Project>, ProjectError> {
        let normalized = crate::domain::normalize::normalize_directory(directory);
        Ok(self.repo.find_by_directory(&normalized)?)
    }

    /// Names of every non-deleted project. The input to
    /// [`taken_names_from`](crate::domain::naming::taken_names_from) wherever a
    /// caller needs to pick a name that will survive `create`'s duplicate check.
    pub fn active_project_names(&self) -> Result<Vec<String>, ProjectError> {
        Ok(self
            .repo
            .list()?
            .into_iter()
            .filter(|p| !p.is_deleted)
            .map(|p| p.name)
            .collect())
    }

    /// Returns the project registered for `directory`, creating one if there
    /// isn't one yet. The name is inferred exactly the way the GUI's
    /// `suggest_project_name` command infers it — the git remote's repo name
    /// when the directory is a repo with a remote, otherwise the folder name —
    /// and then disambiguated against the names already in use.
    ///
    /// Checks `find_by_directory` before running detection: an already-tracked
    /// directory must resolve in a single lookup, not pay for a full detector
    /// scan whose result is thrown away. The observer CLI calls this on
    /// directories it has already registered far more often than on new ones.
    pub fn ensure_project(&self, directory: &str) -> Result<Project, ProjectError> {
        if let Some(existing) = self.find_by_directory(directory)? {
            return Ok(existing);
        }
        let trackers = self.preview_detection(directory);
        let name =
            suggest_project_name(&trackers, directory).unwrap_or_else(|| "project".to_string());
        self.ensure_project_named(directory, &name)
    }

    /// Get-or-create for `directory`, using `name` when it has to create.
    ///
    /// The scanner's commit path: the user reviewed and possibly edited that
    /// name, so it is used rather than re-derived. If it collides with a name
    /// already in use it is disambiguated — `create` would otherwise reject it
    /// with `DuplicateName`, which across a two-hundred-directory import means
    /// losing a row for a reason the user cannot act on.
    ///
    /// **Idempotent, and never a rename.** A directory already tracked comes
    /// back untouched, whatever `name` says: re-scanning a folder is a no-op,
    /// not an edit to projects the user has since renamed by hand.
    pub fn ensure_project_named(
        &self,
        directory: &str,
        name: &str,
    ) -> Result<Project, ProjectError> {
        // A soft-deleted match is not "already tracked" — `create`'s own
        // duplicate check agrees (it filters `is_deleted` too, below) — so a
        // binned-then-recreated directory falls through to a fresh `create`
        // rather than resurrecting the old, still-hidden row.
        if let Some(existing) = self.find_by_directory(directory)?.filter(|p| !p.is_deleted) {
            return Ok(existing);
        }
        let taken = taken_names_from(self.active_project_names()?);
        let parent = Path::new(directory)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");
        let name = disambiguate(name, parent, &taken);
        self.create(name, directory.to_string(), None, None)
    }
}
