//! Matching a typed query against projects: [`resolve`] finds the one project
//! `indexer show app` means, and [`filter`] every row `indexer list app` shows.
//! [`containing`] is the one a folder is in, for `show .`.

use std::path::Path;

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
///    only (`work/app` matches `~/work/app`, `rk/app` does not), where `...`
///    stands for any number of folders (`work/.../app` matches
///    `~/work/client/acme/app`);
/// 4. part of a name, those starting with the query first, then the most
///    recently opened.
///
/// Rules 3 and 4 are [`filter`], so `show` and `list` never disagree about
/// what a path or part of a name matches. An empty query, or one no rule
/// matches, is [`Resolution::NotFound`].
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

    pick(filter(projects, &query))
}

/// Every project `query` matches, best first, ignoring case — the rows
/// `indexer list <query>` shows.
///
/// These are rules 3 and 4 of [`resolve`], not all four:
///
/// - a query containing `/` matches the end of the directory, whole folder
///   names only (`work/app` matches `~/work/app`), with `...` for any number
///   of folders (`work/.../app`);
/// - anything else matches part of a name, ranked the exact name first, then
///   names starting with it, then the rest, each most recently opened first.
///
/// So `app` lists `app-gateway` and `my-app` beneath the projects named
/// `app`, where `resolve` would stop at the exact name. An empty query, or
/// one nothing matches, returns an empty `Vec`.
pub fn filter<'a>(projects: &'a [Project], query: &str) -> Vec<&'a Project> {
    let query = query.to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    if query.contains('/') {
        return matches_path(projects, &query);
    }
    matches_name(projects, &query)
}

/// The project `dir` is in: the one whose directory is `dir` or holds it, the
/// way git finds the repository from any folder inside it. `None` when no
/// project holds `dir`.
///
/// When projects nest, the innermost wins: in `~/work/mono/packages/web/src`
/// it is the `web` package, not `mono` around it. Folders compare whole, so
/// `~/work/app-two` is not inside `~/work/app`.
///
/// `dir` is compared as given, so the caller makes it absolute and canonical,
/// as `add` stores every directory.
pub fn containing<'a>(projects: &'a [Project], dir: &Path) -> Option<&'a Project> {
    // `Path::starts_with` compares whole folders, not letters: the reason
    // `app-two` is not inside `app`.
    projects
        .iter()
        .filter(|p| dir.starts_with(&p.directory))
        // The innermost of nested projects is the one with the most folders.
        .max_by_key(|p| Path::new(&p.directory).components().count())
}

fn pick(matches: Vec<&Project>) -> Resolution<'_> {
    match matches.len() {
        0 => Resolution::NotFound,
        1 => Resolution::Found(matches[0]),
        _ => Resolution::Ambiguous(matches),
    }
}

/// Whether `directory` ends with the folders of `query`, compared whole and
/// from the end. A `...` in `query` stands for any number of folders, none
/// included: `work/.../app` matches `~/work/app` and
/// `~/work/client/acme/app`, but not `~/work/app/src`.
///
/// Folder by folder rather than one `ends_with` on the text, which has no
/// way to say "anything here".
fn path_matcher(directory: &str, query: &str) -> bool {
    let directory = directory.to_lowercase();
    let folders: Vec<&str> = directory.split('/').collect();
    let wanted: Vec<&str> = query.split('/').collect();
    ends_with_folders(&folders, &wanted)
}

/// Whether `folders` ends with `wanted`, each a list of single folder names,
/// checked from the last one back. It calls itself on what is left after each
/// folder that matches, so every call has fewer to check.
fn ends_with_folders(folders: &[&str], wanted: &[&str]) -> bool {
    // Every wanted folder has matched; whatever comes before them is the
    // rest of the path, which a path ending doesn't care about.
    let Some((last, before)) = wanted.split_last() else {
        return true;
    };
    // `...` can skip any number of folders: try keeping each count of them
    // before it until the rest of `wanted` fits one.
    if *last == "..." {
        return (0..=folders.len()).any(|kept| ends_with_folders(&folders[..kept], before));
    }
    match folders.split_last() {
        Some((folder, rest)) if folder == last => ends_with_folders(rest, before),
        _ => false,
    }
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

/// Rule 4: projects whose name contains `query`, ranked: the exact name,
/// then names starting with it, then the rest, each most recently opened
/// first.
fn matches_name<'a>(projects: &'a [Project], query: &str) -> Vec<&'a Project> {
    let mut by_name: Vec<&Project> = projects
        .iter()
        .filter(|p| p.name.to_lowercase().contains(query))
        .collect();
    by_name.sort_by_key(|p| {
        let name = p.name.to_lowercase();
        let ranking = if name == query {
            0
        } else if name.starts_with(query) {
            1
        } else {
            2
        };
        (ranking, std::cmp::Reverse(p.last_opened_at))
    });
    by_name
}
