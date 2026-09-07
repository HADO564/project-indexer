use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::detectors::{DetectorOutcome, DetectorRunner};
use crate::domain::naming::suggest_project_name;

/// The ceiling on directories visited in one scan.
///
/// What makes a blocking scan acceptable without progress events or a cancel
/// button: a scan pointed at `C:\` terminates and says so, rather than
/// appearing to hang. See the spec's *Decisions*.
pub const MAX_DIRECTORIES: usize = 50_000;

/// Directories skipped **always**, regardless of `include_ignored`.
///
/// This is a correctness rule, not a performance one. A repository's
/// submodules are real repositories stored under `.git/modules`; a walk that
/// descended there would detect each one and offer to import it as a separate
/// project. No user intent makes that right, so the include-ignored option
/// deliberately cannot reach these.
const ALWAYS_PRUNED: &[&str] = &[".git", ".svn", ".hg"];

/// Directories skipped unless the user ticks "include ignored directories".
///
/// Default-on because these trees are enormous and rarely projects;
/// overridable because "never" is not quite true and the person scanning knows
/// their own disk better than this list does.
const CONVENTIONALLY_IGNORED: &[&str] =
    &["node_modules", "target", ".venv", "venv", "build", "dist"];

/// How far below the chosen root to look.
///
/// `Quick` is exactly `Deep { depth: 1 }` to the walker. It stays a separate
/// variant because it is a separate user choice — the `~/code` case, where
/// every child is a project and nothing deeper needs visiting — and spelling
/// that as a magic number in the UI would read worse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum ScanMode {
    Quick,
    Deep { depth: u32 },
}

impl ScanMode {
    fn max_depth(&self) -> u32 {
        match self {
            ScanMode::Quick => 1,
            ScanMode::Deep { depth } => *depth,
        }
    }
}

/// One scan's settings, as the user configured them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRequest {
    /// The folder to scan *inside* — `~/projects`, not a project's own
    /// directory. Named `scan_root` rather than `root` because `GitInfo`
    /// already has a `repo_root` meaning something else entirely.
    pub scan_root: String,
    #[serde(flatten)]
    pub mode: ScanMode,
    /// Detector kinds to scan for. Empty means nothing is found, deliberately.
    pub detectors: Vec<String>,
    pub include_ignored: bool,
}

/// A directory the scan thinks is a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub directory: String,
    pub suggested_name: String,
    pub matched_kinds: Vec<String>,
    /// Always `false` as [`walk`] emits it — the walk has no store. Filled in
    /// by `ScanService` before the report reaches the UI. Kept on `Candidate`
    /// rather than in a parallel list so the review table has one row type.
    pub already_tracked: bool,
}

/// What one scan found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    pub candidates: Vec<Candidate>,
    pub visited: usize,
    /// `true` when [`MAX_DIRECTORIES`] was reached, so the UI can say the
    /// results are incomplete rather than implying the disk holds no more.
    pub stopped_early: bool,
}

fn is_pruned(name: &str, include_ignored: bool) -> bool {
    if ALWAYS_PRUNED.contains(&name) {
        return true;
    }
    if include_ignored {
        return false;
    }
    CONVENTIONALLY_IGNORED.contains(&name) || name.starts_with('.')
}

/// Immediate subdirectories of `path`, sorted by name, excluding pruned ones.
///
/// Sorted so candidate order is stable across runs — name disambiguation
/// depends on the order candidates are processed in, so an unstable walk would
/// produce `work/api` on one run and `code/api` on the next.
///
/// Symlinks are not followed, which is also the cycle guarantee, and it is
/// structural rather than a check: `DirEntry::file_type()` does *not* follow
/// links on any platform, so a symlink pointing at a directory reports
/// `is_dir() == false` and is never enqueued. (No test covers this — creating
/// a symlink on Windows needs elevation or developer mode, so a test would
/// fail for most contributors rather than catching a regression. Changing this
/// filter to `entry.metadata()`, which *does* follow links, is what would
/// break it.)
fn child_directories(path: &Path, include_ignored: bool) -> Vec<PathBuf> {
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        // An unreadable directory is skipped, never fatal. A permissions error
        // partway through `C:\Users` must not discard what has been found.
        Err(e) => {
            eprintln!(
                "Scan: skipping unreadable directory '{}': {e}",
                path.display()
            );
            return Vec::new();
        }
    };

    let mut children: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|entry| !is_pruned(&entry.file_name().to_string_lossy(), include_ignored))
        .map(|entry| entry.path())
        .collect();
    children.sort();
    children
}

/// Walks `request.scan_root` looking for directories the ticked detectors
/// recognize.
///
/// Iterative BFS over an explicit queue rather than recursion, so a
/// pathological tree cannot blow the stack.
///
/// **A matched directory ends that branch.** A repository inside a repository
/// is usually vendored or a submodule, not a separate thing to track. Note the
/// consequence of filtering by the ticked detectors: with only git ticked, an
/// Unreal-only directory is *not* a boundary, so the walk descends through it
/// looking for repositories — which is the right reading of "import my repos,
/// not my games".
pub fn walk(request: &ScanRequest, detectors: &DetectorRunner) -> ScanReport {
    walk_limited(request, detectors, MAX_DIRECTORIES)
}

/// [`walk`] with the directory ceiling as a parameter.
///
/// Exists so the stop-early behaviour is testable without building 50,000
/// directories. `walk` is the only caller outside tests; the behaviour at the
/// ceiling is what matters, not the specific number.
pub fn walk_limited(
    request: &ScanRequest,
    detectors: &DetectorRunner,
    max_directories: usize,
) -> ScanReport {
    let ticked: Vec<&str> = request.detectors.iter().map(String::as_str).collect();
    let max_depth = request.mode.max_depth();

    let mut candidates = Vec::new();
    let mut visited = 0usize;
    let mut stopped_early = false;

    let mut queue: VecDeque<(PathBuf, u32)> = VecDeque::new();
    for child in child_directories(Path::new(&request.scan_root), request.include_ignored) {
        queue.push_back((child, 1));
    }

    while let Some((path, depth)) = queue.pop_front() {
        if visited >= max_directories {
            stopped_early = true;
            break;
        }
        visited += 1;

        let detection = detectors.inspect_kinds(&path, Some(&ticked));
        for error in detection.errors() {
            eprintln!("Scan: detector error at '{}': {error}", path.display());
        }
        let matched_kinds: Vec<String> = detection
            .outcomes
            .iter()
            .filter_map(|o| match o {
                DetectorOutcome::Detected { kind, .. } => Some((*kind).to_string()),
                _ => None,
            })
            .collect();

        if !matched_kinds.is_empty() {
            let directory = path.to_string_lossy().to_string();
            let suggested_name = suggest_project_name(&detection.trackers(), &directory)
                .unwrap_or_else(|| "project".to_string());
            candidates.push(Candidate {
                directory,
                suggested_name,
                matched_kinds,
                already_tracked: false,
            });
            // A project is a boundary — do not descend into it.
            continue;
        }

        if depth < max_depth {
            for child in child_directories(&path, request.include_ignored) {
                queue.push_back((child, depth + 1));
            }
        }
    }

    // BFS visits breadth-first, so candidates arrive grouped by depth rather
    // than in path order. Sorting here is what the determinism test pins.
    candidates.sort_by(|a, b| a.directory.cmp(&b.directory));

    ScanReport {
        candidates,
        visited,
        stopped_early,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detectors::git::Gitector;
    use std::path::{Path, PathBuf};

    fn runner() -> DetectorRunner {
        DetectorRunner::new(vec![Box::new(Gitector)])
    }

    fn temp_tree(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("project-indexer-tests-scan-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("should create temp dir");
        dir
    }

    fn repo_at(root: &Path, relative: &str) -> PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(&path).expect("should create dir");
        git2::Repository::init(&path).expect("should init a git repo");
        path
    }

    fn plain_at(root: &Path, relative: &str) -> PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(&path).expect("should create dir");
        path
    }

    fn request(root: &Path, mode: ScanMode) -> ScanRequest {
        ScanRequest {
            scan_root: root.to_string_lossy().to_string(),
            mode,
            detectors: vec!["git".to_string()],
            include_ignored: false,
        }
    }

    fn directories(report: &ScanReport) -> Vec<String> {
        report
            .candidates
            .iter()
            .map(|c| {
                Path::new(&c.directory)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn quick_scan_finds_children_and_stops() {
        let root = temp_tree("quick");
        repo_at(&root, "api");
        repo_at(&root, "site");
        repo_at(&root, "nested/deep-repo");

        let report = walk(&request(&root, ScanMode::Quick), &runner());

        assert_eq!(directories(&report), vec!["api", "site"]);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn deep_scan_descends_to_the_requested_depth() {
        let root = temp_tree("deep");
        // Not named "target": that collides with the Rust build directory in
        // `CONVENTIONALLY_IGNORED` and would be pruned before ever being
        // visited, regardless of depth.
        repo_at(&root, "one/two/leaf");

        let shallow = walk(&request(&root, ScanMode::Deep { depth: 2 }), &runner());
        assert!(shallow.candidates.is_empty());

        let deep = walk(&request(&root, ScanMode::Deep { depth: 3 }), &runner());
        assert_eq!(directories(&deep), vec!["leaf"]);

        std::fs::remove_dir_all(&root).ok();
    }

    /// A repository inside a repository is vendored or a submodule, not a
    /// separate thing to track — so a matched directory ends that branch.
    #[test]
    fn a_project_is_a_boundary_and_its_children_are_not_visited() {
        let root = temp_tree("boundary");
        let outer = repo_at(&root, "outer");
        repo_at(&outer, "vendor/inner");

        let report = walk(&request(&root, ScanMode::Deep { depth: 5 }), &runner());

        assert_eq!(directories(&report), vec!["outer"]);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn ignored_directories_are_pruned_by_default_and_included_on_request() {
        let root = temp_tree("prune");
        repo_at(&root, "node_modules/some-dep");

        let pruned = walk(&request(&root, ScanMode::Deep { depth: 3 }), &runner());
        assert!(pruned.candidates.is_empty());

        let mut with_ignored = request(&root, ScanMode::Deep { depth: 3 });
        with_ignored.include_ignored = true;
        let included = walk(&with_ignored, &runner());
        assert_eq!(directories(&included), vec!["some-dep"]);

        std::fs::remove_dir_all(&root).ok();
    }

    /// The correctness rule, not a performance one: a repository's submodules
    /// live under `.git/modules` and are real repositories. Reaching them would
    /// offer to import every submodule as its own project, so `.git` is pruned
    /// even when the user asked to include ignored directories.
    #[test]
    fn vcs_metadata_is_pruned_even_when_ignored_directories_are_included() {
        let root = temp_tree("vcs-metadata");
        let outer = plain_at(&root, "outer");
        repo_at(&outer, ".git/modules/sub");

        let mut req = request(&root, ScanMode::Deep { depth: 6 });
        req.include_ignored = true;
        let report = walk(&req, &runner());

        assert!(
            report.candidates.is_empty(),
            "submodule repositories under .git must never be offered: {:?}",
            directories(&report)
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn nothing_ticked_finds_nothing() {
        let root = temp_tree("none-ticked");
        repo_at(&root, "api");

        let mut req = request(&root, ScanMode::Quick);
        req.detectors.clear();
        let report = walk(&req, &runner());

        assert!(report.candidates.is_empty());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn candidates_carry_a_suggested_name_and_matched_kinds() {
        let root = temp_tree("names");
        repo_at(&root, "api");

        let report = walk(&request(&root, ScanMode::Quick), &runner());

        let candidate = &report.candidates[0];
        assert_eq!(candidate.suggested_name, "api");
        assert_eq!(candidate.matched_kinds, vec!["git".to_string()]);
        assert!(!candidate.already_tracked);

        std::fs::remove_dir_all(&root).ok();
    }

    /// Name assignment depends on candidate order, so the order has to be
    /// stable — otherwise the same tree could produce `work/api` on one run
    /// and `code/api` on the next.
    #[test]
    fn the_same_tree_scanned_twice_produces_the_same_report() {
        let root = temp_tree("determinism");
        repo_at(&root, "zebra");
        repo_at(&root, "alpha");
        repo_at(&root, "middle");

        let first = walk(&request(&root, ScanMode::Quick), &runner());
        let second = walk(&request(&root, ScanMode::Quick), &runner());

        assert_eq!(directories(&first), vec!["alpha", "middle", "zebra"]);
        assert_eq!(directories(&first), directories(&second));

        std::fs::remove_dir_all(&root).ok();
    }

    /// A root that does not exist is an empty report, not a panic. The user
    /// can type a path by hand, and it can vanish between typing and scanning.
    #[test]
    fn a_missing_root_yields_an_empty_report() {
        let report = walk(
            &request(Path::new("/definitely/not/a/real/path"), ScanMode::Quick),
            &runner(),
        );

        assert!(report.candidates.is_empty());
        assert!(!report.stopped_early);
    }

    /// The bound that makes a blocking scan acceptable. Tested through
    /// `walk_limited` with a tiny limit rather than by building 50,000
    /// directories — the behaviour at the ceiling is what matters, not the
    /// specific number.
    #[test]
    fn reaching_the_limit_stops_the_walk_and_says_so() {
        let root = temp_tree("limit");
        plain_at(&root, "a");
        plain_at(&root, "b");
        plain_at(&root, "c");

        let report = walk_limited(&request(&root, ScanMode::Quick), &runner(), 2);

        assert_eq!(report.visited, 2);
        assert!(report.stopped_early);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_walk_that_finishes_is_not_marked_stopped_early() {
        let root = temp_tree("limit-not-hit");
        plain_at(&root, "a");

        let report = walk_limited(&request(&root, ScanMode::Quick), &runner(), 50);

        assert!(!report.stopped_early);

        std::fs::remove_dir_all(&root).ok();
    }
}
