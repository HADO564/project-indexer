use crate::domain::Tracker;
use std::collections::HashSet;

/// `https://github.com/user/my-repo.git` / `git@github.com:user/my-repo.git` → `my-repo`.
pub fn repo_name_from_url(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches('/');
    let without_git = trimmed.strip_suffix(".git").unwrap_or(trimmed);
    without_git
        .split(['/', ':'])
        .rfind(|s| !s.is_empty())
        .map(str::to_string)
}

/// Last path segment of a directory, either separator style. `D:\Projects\Friction\` → `Friction`.
pub fn folder_name_from_directory(directory: &str) -> Option<String> {
    directory
        .trim()
        .trim_end_matches(['\\', '/'])
        .split(['\\', '/'])
        .rfind(|s| !s.is_empty())
        .map(str::to_string)
}

/// The git remote's repo name if the project is in git with a remote, else the folder name.
pub fn suggest_project_name(trackers: &[Tracker], directory: &str) -> Option<String> {
    let from_remote = trackers
        .iter()
        .filter(|t| t.is("git"))
        .find_map(|t| t.str_field("repo_url").and_then(repo_name_from_url));
    from_remote.or_else(|| folder_name_from_directory(directory))
}

/// Lowercases `names` into the set [`disambiguate`] expects.
///
/// Exists so the case rule lives in one place rather than at every call site.
/// It must stay case-*insensitive* to match
/// `Project::check_for_duplicate_name_or_dir`, which compares with
/// `eq_ignore_ascii_case` — a name this function considered free but `create`
/// considers a duplicate would fail at commit, after the user reviewed it.
pub fn taken_names_from(names: impl IntoIterator<Item = String>) -> HashSet<String> {
    names.into_iter().map(|n| n.trim().to_lowercase()).collect()
}

/// A free project name for a directory, given the names already in use.
///
/// Project names must be unique, so scanning `~/code` and `~/work` when both
/// hold an `api` folder collides on the first run. Resolution, in order:
///
/// 1. `preferred` if free — `api`
/// 2. else parent-qualified — `work/api`
/// 3. else suffixed — `work/api (2)`, counting up
///
/// Step 3 is the terminating fallback: the loop always finds a free integer,
/// so this never loops forever and always returns something usable. Qualifying
/// by parent is preferred over a bare suffix because `api (2)` tells you
/// nothing a month later, where `work/api` says which one it is.
///
/// Shared deliberately with `ProjectService::ensure_project` — the scanner and
/// the observer CLI meet the same collision and must not invent two answers.
pub fn disambiguate(preferred: &str, parent_dir: &str, taken: &HashSet<String>) -> String {
    let preferred = preferred.trim();
    let is_free = |candidate: &str| !taken.contains(&candidate.trim().to_lowercase());

    if is_free(preferred) {
        return preferred.to_string();
    }

    let qualified = match folder_name_from_directory(parent_dir) {
        Some(parent) if !parent.is_empty() => {
            let qualified = format!("{parent}/{preferred}");
            if is_free(&qualified) {
                return qualified;
            }
            qualified
        }
        // A drive root or an empty parent gives nothing to qualify with, so
        // suffix the bare name instead of producing a leading slash.
        _ => preferred.to_string(),
    };

    (2u32..)
        .map(|n| format!("{qualified} ({n})"))
        .find(|candidate| is_free(candidate))
        .expect("an unbounded counter always reaches a free name")
}
