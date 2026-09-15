//! Finding the one project a typed query means — `indexer show app`.

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

/// Resolves `query` to a project, ignoring case. The first rule that matches
/// anything decides:
///
/// 1. a project's exact name — several projects can share one, and then
///    they are all returned, most recently opened first;
/// 2. a project's full id, or an id prefix at least [`SHORT_ID_LEN`] long;
/// 3. a query containing `/`: the end of the directory, whole folder names
///    only (`work/app` matches `~/work/app`, `rk/app` does not);
/// 4. part of a name, those starting with the query first, then the most
///    recently opened.
pub fn resolve<'a>(projects: &'a [Project], query: &str) -> Resolution<'a> {
    let query = query.to_lowercase();
    if query.is_empty() {
        return Resolution::NotFound;
    }
    let mut exact: Vec<&Project> = projects
        .iter()
        .filter(|p| p.name.to_lowercase() == query)
        .collect();
    if !exact.is_empty() {
        exact.sort_by_key(|p| std::cmp::Reverse(p.last_opened_at));
        return pick(exact);
    }
    if looks_like_id(&query) {
        if let Some(project) = projects.iter().find(|p| p.id.to_lowercase() == query) {
            return Resolution::Found(project);
        }
        if query.len() >= SHORT_ID_LEN {
            let matches: Vec<&Project> = projects
                .iter()
                .filter(|p| p.id.to_lowercase().starts_with(&query))
                .collect();
            if !matches.is_empty() {
                return pick(matches);
            }
        }
    }
    if query.contains('/') {
        let matches: Vec<&Project> = projects
            .iter()
            .filter(|p| path_matcher(p.directory.as_str(), &query))
            .collect();
        return pick(matches);
    }
    let mut matches: Vec<&Project> = projects
        .iter()
        .filter(|p| p.name.to_lowercase().contains(&query))
        .collect();
    matches.sort_by_key(|p| {
        let starts = if p.name.to_lowercase().starts_with(&query) {
            0
        } else {
            1
        };
        (starts, std::cmp::Reverse(p.last_opened_at))
    });
    pick(matches)
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
