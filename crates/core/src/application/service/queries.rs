//! Reading lists of projects for the views — All, Bin, Favourites.
//!
//! Ordering is the caller's, via [`SortOptions`]; these do not sort by policy.

use super::*;

impl ProjectService {
    /// Non-deleted projects for the main list view, ordered per `options`.
    pub fn list(&self, options: SortOptions) -> Result<Vec<Project>, ProjectError> {
        let mut projects: Vec<Project> = self
            .repo
            .list()?
            .into_iter()
            .filter(|p| !p.is_deleted)
            .collect();
        sort_projects(&mut projects, options);
        Ok(projects)
    }

    /// Soft-deleted projects for the bin view, ordered per `options`.
    pub fn list_deleted(&self, options: SortOptions) -> Result<Vec<Project>, ProjectError> {
        Ok(filter_deleted(&self.repo.list()?, options))
    }

    /// Favorited, non-deleted projects, ordered per `options`.
    pub fn list_favorites(&self, options: SortOptions) -> Result<Vec<Project>, ProjectError> {
        let active: Vec<Project> = self
            .repo
            .list()?
            .into_iter()
            .filter(|p| !p.is_deleted)
            .collect();
        Ok(filter_favorites(&active, options))
    }
}
