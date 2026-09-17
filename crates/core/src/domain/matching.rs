//! Finding the project a typed query means — `indexer show app`.

use crate::domain::Project;

/// The shortest id prefix accepted in place of a full id, as git accepts a
/// short commit hash.
pub const SHORT_ID_LEN: usize = 8;

pub enum Resolution<'a> {
    Found(&'a Project),
    NotFound,
    /// Several projects match, best first.
    Ambiguous(Vec<&'a Project>),
}

/// Resolves `query` to one project, ignoring case: [`Resolution::Found`] for
/// one match, [`Resolution::Ambiguous`] for several, best first. The first
/// rule that matches anything decides:
///
/// 1. a project's exact name — several projects can share one, and then
///    they are all returned, most recently opened first;
/// 2. a project's full id, or an id prefix at least [`SHORT_ID_LEN`] long;
/// 3. a query containing `/`: the end of the directory, whole folder names
///    only (`work/app` matches `~/work/app`, `rk/app` does not);
/// 4. part of a name, those starting with the query first, then the most
///    recently opened.
///
/// An empty query, or one no rule matches, is [`Resolution::NotFound`].
pub fn resolve<'a>(projects: &'a [Project], query: &str) -> Resolution<'a> {
    let query = query.to_lowercase();
    if query.is_empty() {
        return Resolution::NotFound;
    }
    let exact = matches_exact(projects, &query);
    if !exact.is_empty() {
        return pick(exact);
    }
    if looks_like_id(&query) {
        let by_id = matches_id(projects, &query);
        if !by_id.is_empty() {
            return pick(by_id);
        }
    }
    if query.contains('/') {
        return pick(matches_path(projects, &query));
    }
    pick(matches_name(projects, &query))
}

fn pick(matches: Vec<&Project>) -> Resolution<'_> {
    match matches.len() {
        0 => Resolution::NotFound,
        1 => Resolution::Found(matches[0]),
        _ => Resolution::Ambiguous(matches),
    }
}

fn path_matcher(directory: &str, query: &str) -> bool {
    let suffix = format!("/{query}");
    directory.to_lowercase().ends_with(&suffix)
}

/// Ids are UUIDs: hex digits and dashes. Anything else can't be one, so a
/// name like `deadbeef-app` is still searched by name when no id matches.
fn looks_like_id(query: &str) -> bool {
    query.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

// Each rule below takes a query `resolve` has already lowercased.

/// Rule 1: projects named exactly `query`, most recently opened first.
fn matches_exact<'a>(projects: &'a [Project], query: &str) -> Vec<&'a Project> {
    let mut exact: Vec<&Project> = projects
        .iter()
        .filter(|p| p.name.to_lowercase() == query)
        .collect();
    exact.sort_by_key(|p| std::cmp::Reverse(p.last_opened_at));
    exact
}

/// Rule 2: the project with id `query`, or every project whose id starts
/// with it when it is at least [`SHORT_ID_LEN`] long.
fn matches_id<'a>(projects: &'a [Project], query: &str) -> Vec<&'a Project> {
    if let Some(project) = projects.iter().find(|p| p.id.to_lowercase() == query) {
        return vec![project];
    }
    if query.len() < SHORT_ID_LEN {
        return Vec::new();
    }
    projects
        .iter()
        .filter(|p| p.id.to_lowercase().starts_with(query))
        .collect()
}

/// Rule 3: projects whose directory ends with `query`, whole folder names only.
fn matches_path<'a>(projects: &'a [Project], query: &str) -> Vec<&'a Project> {
    projects
        .iter()
        .filter(|p| path_matcher(&p.directory, query))
        .collect()
}

/// Rule 4: projects whose name contains `query`, those starting with it
/// first, then the most recently opened.
fn matches_name<'a>(projects: &'a [Project], query: &str) -> Vec<&'a Project> {
    let mut by_name: Vec<&Project> = projects
        .iter()
        .filter(|p| p.name.to_lowercase().contains(query))
        .collect();
    by_name.sort_by_key(|p| {
        let starts = if p.name.to_lowercase().starts_with(query) {
            0
        } else {
            1
        };
        (starts, std::cmp::Reverse(p.last_opened_at))
    });
    by_name
}
