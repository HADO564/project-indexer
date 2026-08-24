use crate::models::Project;
use std::cmp::Ordering;

/// Orders projects for the list view: most recently opened first, then
/// never-opened ones.
///
/// Ties fall back to `created_at` because the projects arrive from a
/// `HashMap`, whose iteration order is arbitrary and reseeded every run.
/// Without a tiebreaker, `sort_by` would faithfully preserve that random
/// order and never-opened projects would shuffle between launches.
pub fn sort_projects(projects: &mut [Project]) {
    projects.sort_by(|a, b| {
        match (&a.last_opened_at, &b.last_opened_at) {
            // Reversed: the later timestamp sorts first.
            (Some(a_opened), Some(b_opened)) => b_opened.cmp(a_opened),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
        .then_with(|| a.created_at.cmp(&b.created_at))
        .then_with(|| a.id.cmp(&b.id))
    });
}
