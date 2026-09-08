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
    /// Always `false` as [`walk`] emits it — the walk has no `taken` set to
    /// collide against. Set `true` by `ScanService::scan` when disambiguation
    /// changed `suggested_name` from what the walk proposed, so the review UI
    /// can flag an auto-rename as distinct from one the user typed themselves.
    pub disambiguated: bool,
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
                disambiguated: false,
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
