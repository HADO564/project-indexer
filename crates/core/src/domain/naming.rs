use crate::domain::Tracker;

/// `https://github.com/user/my-repo.git` / `git@github.com:user/my-repo.git` → `my-repo`.
pub fn repo_name_from_url(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches('/');
    let without_git = trimmed.strip_suffix(".git").unwrap_or(trimmed);
    without_git
        .split(['/', ':'])
        .filter(|s| !s.is_empty())
        .next_back()
        .map(str::to_string)
}

/// Last path segment of a directory, either separator style. `D:\Projects\Friction\` → `Friction`.
pub fn folder_name_from_directory(directory: &str) -> Option<String> {
    directory
        .trim()
        .trim_end_matches(['\\', '/'])
        .split(['\\', '/'])
        .filter(|s| !s.is_empty())
        .next_back()
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
}
