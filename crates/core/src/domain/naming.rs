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
    let from_remote = trackers.iter().find_map(|t| match t {
        Tracker::Git(g) => g.repo_url.as_deref().and_then(repo_name_from_url),
        _ => None,
    });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{GitInfo, Tracker};

    fn git_tracker(repo_url: Option<&str>) -> Tracker {
        Tracker::Git(GitInfo {
            repo_root: "/tmp/x".into(),
            dirty: false,
            detached_head: false,
            repo_url: repo_url.map(str::to_string),
            web_url: None,
            contributors: vec![],
            curr_branch: None,
            branches: None,
            commit_hash: None,
        })
    }

    #[test]
    fn repo_name_from_https_url() {
        assert_eq!(
            repo_name_from_url("https://github.com/user/my-repo.git").as_deref(),
            Some("my-repo")
        );
        assert_eq!(
            repo_name_from_url("https://github.com/user/my-repo").as_deref(),
            Some("my-repo")
        );
    }

    #[test]
    fn repo_name_from_ssh_url() {
        assert_eq!(
            repo_name_from_url("git@github.com:user/my-repo.git").as_deref(),
            Some("my-repo")
        );
    }

    #[test]
    fn repo_name_ignores_trailing_slash() {
        assert_eq!(
            repo_name_from_url("https://github.com/user/my-repo/").as_deref(),
            Some("my-repo")
        );
    }

    #[test]
    fn folder_name_from_windows_path() {
        assert_eq!(
            folder_name_from_directory("D:\\Projects\\Friction\\").as_deref(),
            Some("Friction")
        );
    }

    #[test]
    fn folder_name_from_unix_path() {
        assert_eq!(
            folder_name_from_directory("/home/user/friction").as_deref(),
            Some("friction")
        );
    }

    #[test]
    fn suggest_prefers_git_remote_name() {
        let t = [git_tracker(Some("https://github.com/user/cool-thing.git"))];
        assert_eq!(
            suggest_project_name(&t, "/home/user/local-dir").as_deref(),
            Some("cool-thing")
        );
    }

    #[test]
    fn suggest_falls_back_to_folder_name() {
        let t = [git_tracker(None)];
        assert_eq!(
            suggest_project_name(&t, "/home/user/local-dir").as_deref(),
            Some("local-dir")
        );
        assert_eq!(
            suggest_project_name(&[], "/home/user/local-dir").as_deref(),
            Some("local-dir")
        );
    }

    #[test]
    fn suggest_returns_none_for_empty_directory_and_no_trackers() {
        assert_eq!(suggest_project_name(&[], ""), None);
    }

    fn taken(names: &[&str]) -> std::collections::HashSet<String> {
        taken_names_from(names.iter().map(|s| s.to_string()))
    }

    #[test]
    fn disambiguate_keeps_a_free_name() {
        assert_eq!(disambiguate("api", "/home/user/work", &taken(&[])), "api");
    }

    #[test]
    fn disambiguate_qualifies_by_parent_on_collision() {
        assert_eq!(
            disambiguate("api", "/home/user/work", &taken(&["api"])),
            "work/api"
        );
    }

    #[test]
    fn disambiguate_suffixes_when_the_qualified_name_also_collides() {
        assert_eq!(
            disambiguate("api", "/home/user/work", &taken(&["api", "work/api"])),
            "work/api (2)"
        );
        assert_eq!(
            disambiguate(
                "api",
                "/home/user/work",
                &taken(&["api", "work/api", "work/api (2)"])
            ),
            "work/api (3)"
        );
    }

    /// The collision rule is case-insensitive, matching
    /// `check_for_duplicate_name_or_dir`, which compares with
    /// `eq_ignore_ascii_case`. A scanner that produced `API` next to an
    /// existing `api` would be rejected by `create` at commit time.
    #[test]
    fn disambiguate_matches_case_insensitively() {
        assert_eq!(
            disambiguate("API", "/home/user/work", &taken(&["api"])),
            "work/API"
        );
    }

    /// Windows paths reach this function too — the parent segment has to come
    /// off a backslash path as readily as a forward-slash one.
    #[test]
    fn disambiguate_reads_a_windows_parent() {
        assert_eq!(
            disambiguate("api", "D:\\work", &taken(&["api"])),
            "work/api"
        );
    }

    /// A parent that yields no usable segment (a drive root, an empty string)
    /// must still terminate, falling straight through to the suffix.
    #[test]
    fn disambiguate_falls_back_to_a_suffix_without_a_parent() {
        assert_eq!(disambiguate("api", "", &taken(&["api"])), "api (2)");
    }
}
