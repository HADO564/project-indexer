# Project View Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `ProjectDetailModal` with a dedicated `/project/[id]` route showing per-tracker tabs, live per-detector status, and type-driven field affordances (open links, reveal paths, copy).

**Architecture:** Backend gains `Detector::kind()`, a `Detection` type carrying one outcome per registered detector, and a read-only `inspect_project` command. Frontend gets one client-side route and one generic `TrackerPanel` that renders any tracker from a field-type inferred in `trackers.ts` — no per-detector frontend code.

**Tech Stack:** Rust / Tauri v2 / git2 / serde / thiserror (backend); SvelteKit 2 (SPA, adapter-static) / Svelte 5 runes / Tailwind v4 / TypeScript / vitest (frontend).

**Spec:** `docs/superpowers/specs/2026-08-31-project-view-design.md`

## Global Constraints

- Backend: `cargo test --lib` (run in `src-tauri/`) must stay green; `cargo fmt --check` clean; no new `cargo clippy` warnings beyond the 2 pre-existing (`sort_by_key`, `module has the same name as its containing module`). Do **not** run `cargo fmt` on the whole tree — the repo is rustfmt-clean as of commit `8602bd0`, format only what you touch.
- Frontend: `npm run check` (svelte-check) must report 0 errors; `npm run build` must succeed.
- Rust edition / toolchain: as configured; `Option::is_some_and` (Rust 1.70+) is available.
- Stored-record compatibility: any new field on a serialized struct (`GitInfo`, `Project`) must be `Option<T>` or `#[serde(default)]` — enforced by `loads_a_record_missing_every_absorbable_field`.
- Commit messages: conventional-commit style (`feat:`, `refactor:`, `test:`, `docs:`), ending with the `Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>` trailer.
- The `Detection::into_result` all-or-nothing semantics are a **recorded decision** (`docs/architecture.md`). Preserve them exactly; the guard test `into_result_discards_partial_trackers_on_any_error` must keep asserting the same thing.

---

## File Structure

**Backend (`src-tauri/src/`)**

| File | Responsibility | Change |
|---|---|---|
| `detectors/detector.rs` | `Detector` trait | add `kind()` |
| `detectors/git/gitector.rs` | git detection | `kind()`; `web_url()` helper; populate `web_url` |
| `detectors/unreal/unreal.rs` | unreal detection | `kind()` |
| `detectors/runner.rs` | `Detection`, `DetectorOutcome`, `DetectorRunner` | refactor to `outcomes`; add `inspect()` |
| `detectors/mod.rs` | detector re-exports | export `DetectorOutcome` |
| `models/git.rs` | `GitInfo` | add `web_url: Option<String>` |
| `commands/inspect.rs` | **new** — `inspect_project` command + `ProjectInspection`/`DetectorResult` DTOs + mapping | create |
| `commands/mod.rs` | command module list | add `pub mod inspect;` |
| `commands/projects.rs` | project commands | 3 call sites use `Detection` accessors |
| `lib.rs` | Tauri builder | register `inspect_project` |

**Frontend (`src/`)**

| File | Responsibility | Change |
|---|---|---|
| `lib/api/types.ts` | Rust type mirrors | `GitInfo.web_url`; `ProjectInspection`, `DetectorResult`, `DetectorStatus`, `DirectoryStatus` |
| `lib/api/projects.ts` | project command wrappers | add `inspectProject()` |
| `lib/api/opener.ts` | opener wrappers | add `openExternalUrl()`, `revealPath()` |
| `lib/trackers.ts` | generic tracker helpers | `trackerFields()` returns typed fields |
| `lib/trackers.test.ts` | **new** — unit tests for inference | create |
| `lib/components/TrackerPanel.svelte` | **new** — renders one tracker's typed fields | create |
| `lib/components/ProjectIdentity.svelte` | **new** — the identity `<dl>` block | create |
| `lib/components/ProjectDetailModal.svelte` | (interim) adopt `TrackerPanel`; then **deleted** | modify then delete |
| `lib/components/ProjectCard.svelte` | project row | "Details" → `<a href>` |
| `lib/components/ProjectList.svelte` | list | drop `onShowDetails` |
| `routes/+page.svelte` | home | drop detail-modal state |
| `routes/project/[id]/+page.svelte` | **new** — the project view | create |
| `routes/project/[id]/+page.ts` | **new** — route config | create |
| `vite.config.ts` | build config | add vitest `test` block |
| `package.json` | scripts / deps | `vitest` dev-dep; `test` script |
| `src-tauri/capabilities/default.json` | window permissions | add `opener:allow-reveal-item-in-dir` if needed |

**Docs:** `architecture.md`, `knowledgebase.md`, `checklist.md`, `accomplishments.md`.

---

## Task 1: `Detector::kind()`

**Files:**
- Modify: `src-tauri/src/detectors/detector.rs`
- Modify: `src-tauri/src/detectors/git/gitector.rs:11-12` (impl block)
- Modify: `src-tauri/src/detectors/unreal/unreal.rs:13-14` (impl block)
- Modify: `src-tauri/src/detectors/runner.rs` (test `Boom` struct)
- Test: `src-tauri/src/detectors/git/gitector.rs` (tests mod), `src-tauri/src/detectors/unreal/unreal.rs` (tests mod)

**Interfaces:**
- Produces: `Detector::kind(&self) -> &'static str`. `Gitector::kind()` → `"git"`, `UnrealDetector::kind()` → `"unreal"`.

- [ ] **Step 1: Write the failing tests**

In `src-tauri/src/detectors/git/gitector.rs`, inside `mod tests`, add:

```rust
    #[test]
    fn kind_is_git() {
        assert_eq!(Gitector.kind(), "git");
    }
```

In `src-tauri/src/detectors/unreal/unreal.rs`, inside `mod tests`, add:

```rust
    #[test]
    fn kind_is_unreal() {
        assert_eq!(UnrealDetector.kind(), "unreal");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib kind_is_`
Expected: FAIL — `no method named kind found`.

- [ ] **Step 3: Add the trait method**

In `src-tauri/src/detectors/detector.rs`, add to the `Detector` trait (above `detect`):

```rust
    /// Stable, lowercase identity for this detector, e.g. `"git"`. Used to
    /// tag detection outcomes and to target a single detector on re-detect.
    fn kind(&self) -> &'static str;
```

- [ ] **Step 4: Implement for both detectors**

In `src-tauri/src/detectors/git/gitector.rs`, inside `impl Detector for Gitector`, add above `fn detect`:

```rust
    fn kind(&self) -> &'static str {
        "git"
    }
```

In `src-tauri/src/detectors/unreal/unreal.rs`, inside `impl Detector for UnrealDetector`, add above `fn detect`:

```rust
    fn kind(&self) -> &'static str {
        "unreal"
    }
```

In `src-tauri/src/detectors/runner.rs`, update the test `Boom` struct's impl:

```rust
    impl Detector for Boom {
        fn kind(&self) -> &'static str {
            "boom"
        }
        fn detect(&self, _path: &Path) -> Result<Option<Tracker>, DetectorError> {
            Err(DetectorError::Other("boom".into()))
        }
    }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --lib`
Expected: PASS — all tests green (count unchanged + 2 new).

- [ ] **Step 6: Format and commit**

```bash
cd src-tauri && cargo fmt
git add src-tauri/src/detectors/
git commit -m "$(cat <<'EOF'
feat(detectors): add Detector::kind() identity

Stable lowercase identity per detector ("git", "unreal"), needed to tag
per-detector detection outcomes and target a single detector on re-detect.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: `Detection` → per-detector outcomes

**Files:**
- Modify: `src-tauri/src/detectors/runner.rs` (the `Detection` struct, `impl`, `DetectorRunner::detect_project`, add `inspect`, migrate tests)
- Modify: `src-tauri/src/detectors/mod.rs:7-9` (re-exports)
- Modify: `src-tauri/src/commands/projects.rs` (3 call sites: `create_project`, `refresh_project_trackers`, `detect_project_trackers`)

**Interfaces:**
- Consumes: `Detector::kind()` (Task 1).
- Produces:
  - `pub struct Detection { pub outcomes: Vec<DetectorOutcome> }`
  - `pub enum DetectorOutcome { Detected { kind: &'static str, tracker: Tracker }, NotDetected { kind: &'static str }, Failed { kind: &'static str, error: DetectorError } }`
  - `Detection::trackers(&self) -> Vec<Tracker>`
  - `Detection::errors(&self) -> Vec<&DetectorError>`
  - `Detection::into_result(self) -> Result<Vec<Tracker>, DetectorError>` (unchanged semantics)
  - `DetectorRunner::detect_project(&self, path: &Path) -> Detection` (unchanged signature)
  - `DetectorRunner::inspect(&self, path: &Path, only: Option<&str>) -> Detection`

- [ ] **Step 1: Write the failing tests**

Replace the entire `#[cfg(test)] mod tests { … }` block in `src-tauri/src/detectors/runner.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::detectors::git::Gitector;
    use crate::models::git::GitInfo;
    use std::path::PathBuf;

    fn sample_git_tracker() -> Tracker {
        Tracker::Git(GitInfo {
            repo_root: "/tmp/x".to_string(),
            dirty: false,
            detached_head: false,
            repo_url: None,
            contributors: Vec::new(),
            curr_branch: Some("main".to_string()),
            branches: None,
            commit_hash: None,
        })
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn detector_runner_is_send_and_sync() {
        assert_send_sync::<DetectorRunner>();
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("project-indexer-tests-runner-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("should create temp dir");
        dir
    }

    #[test]
    fn detected_directory_yields_a_detected_outcome() {
        let dir = temp_dir("git-repo");
        git2::Repository::init(&dir).expect("should init a git repo");

        let runner = DetectorRunner::new(vec![Box::new(Gitector)]);
        let detection = runner.detect_project(&dir);

        assert!(matches!(
            detection.outcomes.as_slice(),
            [DetectorOutcome::Detected { kind: "git", .. }]
        ));
        assert!(matches!(detection.trackers().as_slice(), [Tracker::Git(_)]));
        assert!(detection.errors().is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unrecognized_directory_yields_a_not_detected_outcome() {
        let dir = temp_dir("plain");

        let runner = DetectorRunner::new(vec![Box::new(Gitector)]);
        let detection = runner.detect_project(&dir);

        assert!(matches!(
            detection.outcomes.as_slice(),
            [DetectorOutcome::NotDetected { kind: "git" }]
        ));
        assert!(detection.trackers().is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    struct Boom;
    impl Detector for Boom {
        fn kind(&self) -> &'static str {
            "boom"
        }
        fn detect(&self, _path: &Path) -> Result<Option<Tracker>, DetectorError> {
            Err(DetectorError::Other("boom".into()))
        }
    }

    #[test]
    fn one_detector_failing_keeps_the_others_results() {
        let dir = temp_dir("resilient");
        git2::Repository::init(&dir).expect("should init a git repo");

        let runner = DetectorRunner::new(vec![Box::new(Boom), Box::new(Gitector)]);
        let detection = runner.detect_project(&dir);

        assert!(matches!(detection.trackers().as_slice(), [Tracker::Git(_)]));
        assert_eq!(detection.errors().len(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn inspect_runs_only_the_named_detector() {
        let dir = temp_dir("only");
        git2::Repository::init(&dir).expect("should init a git repo");

        let runner = DetectorRunner::new(vec![Box::new(Gitector)]);

        assert_eq!(runner.inspect(&dir, Some("git")).outcomes.len(), 1);
        assert_eq!(runner.inspect(&dir, Some("unreal")).outcomes.len(), 0);
        assert_eq!(runner.inspect(&dir, Some("nonsense")).outcomes.len(), 0);
        assert_eq!(runner.inspect(&dir, None).outcomes.len(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn into_result_returns_every_tracker_when_no_detector_failed() {
        let detection = Detection {
            outcomes: vec![DetectorOutcome::Detected {
                kind: "git",
                tracker: sample_git_tracker(),
            }],
        };

        let trackers = detection.into_result().expect("clean detection is Ok");
        assert!(matches!(trackers.as_slice(), [Tracker::Git(_)]));
    }

    /// The deliberate all-or-nothing contract behind `refresh_project_trackers`
    /// (see `Detection::into_result` and `docs/architecture.md`): a partial
    /// success is reported as a failure, never half-persisted. If this test is
    /// changed, the persistence behaviour is changing — do it on purpose.
    #[test]
    fn into_result_discards_partial_trackers_on_any_error() {
        let detection = Detection {
            outcomes: vec![
                DetectorOutcome::Detected {
                    kind: "git",
                    tracker: sample_git_tracker(),
                },
                DetectorOutcome::Failed {
                    kind: "unity",
                    error: DetectorError::Other("unity detector blew up".into()),
                },
            ],
        };

        assert!(detection.into_result().is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib detectors::runner`
Expected: FAIL to compile — `Detection` has no field `outcomes`, no `DetectorOutcome` type. (`sample_git_tracker` has no `web_url` field yet — Task 3 adds it.)

- [ ] **Step 3: Refactor `Detection` and `DetectorRunner`**

In `src-tauri/src/detectors/runner.rs`, replace the `Detection` struct + its `impl` + `DetectorRunner::detect_project` (lines 8–87) with:

```rust
/// The outcome of running the registered detectors against a path: one
/// [`DetectorOutcome`] per detector consulted, in registration order.
///
/// Detectors are isolated from one another — a detector that errors is
/// recorded as [`DetectorOutcome::Failed`] without discarding what the others
/// produced. Best-effort callers use [`trackers`](Self::trackers) /
/// [`errors`](Self::errors); a caller that needs all-or-nothing uses
/// [`into_result`](Self::into_result).
#[derive(Debug, Default)]
pub struct Detection {
    pub outcomes: Vec<DetectorOutcome>,
}

/// What one detector reported for one path.
#[derive(Debug)]
pub enum DetectorOutcome {
    /// The detector recognized the path.
    Detected {
        kind: &'static str,
        tracker: Tracker,
    },
    /// The detector ran cleanly and did not recognize the path — a normal
    /// outcome, not a failure.
    NotDetected { kind: &'static str },
    /// The detector hit a genuine problem inspecting the path.
    Failed {
        kind: &'static str,
        error: DetectorError,
    },
}

impl Detection {
    /// Trackers from the detectors that matched, in registration order.
    pub fn trackers(&self) -> Vec<Tracker> {
        self.outcomes
            .iter()
            .filter_map(|o| match o {
                DetectorOutcome::Detected { tracker, .. } => Some(tracker.clone()),
                _ => None,
            })
            .collect()
    }

    /// Errors from the detectors that failed.
    pub fn errors(&self) -> Vec<&DetectorError> {
        self.outcomes
            .iter()
            .filter_map(|o| match o {
                DetectorOutcome::Failed { error, .. } => Some(error),
                _ => None,
            })
            .collect()
    }

    /// The all-or-nothing view: `Ok(trackers)` only if no detector failed,
    /// otherwise `Err` with the first failure and the partial trackers
    /// **discarded**.
    ///
    /// Deliberate domain decision (see `docs/architecture.md`). Detection
    /// results are persisted verbatim, so `refresh_project_trackers` — an
    /// explicit, user-triggered "re-scan everything" — either fully succeeds
    /// or changes nothing: a stored tracker set silently missing whatever the
    /// failing detector would have produced is worse than a visible "refresh
    /// failed, try again".
    ///
    /// The alternative, once detectors are numerous and truly independent, is
    /// to persist the successes and surface per-detector errors separately.
    /// That's a real change with UI implications — make it on purpose.
    /// `into_result_discards_partial_trackers_on_any_error` guards this.
    pub fn into_result(self) -> Result<Vec<Tracker>, DetectorError> {
        let mut trackers = Vec::new();
        for outcome in self.outcomes {
            match outcome {
                DetectorOutcome::Detected { tracker, .. } => trackers.push(tracker),
                DetectorOutcome::NotDetected { .. } => {}
                DetectorOutcome::Failed { error, .. } => return Err(error),
            }
        }
        Ok(trackers)
    }
}

/// Runs the registered [`Detector`]s against a path. Detectors are held as
/// `Box<dyn Detector>` rather than concrete types, so registering
/// Unity/Godot/MATLAB support later is a matter of adding one to
/// [`default_detectors`](crate::detectors::registry::default_detectors).
///
/// The app builds one of these at startup into Tauri managed state
/// (`App::manage`); commands pull it out with `State<DetectorRunner>`.
pub struct DetectorRunner {
    detectors: Vec<Box<dyn Detector>>,
}

impl DetectorRunner {
    pub fn new(detectors: Vec<Box<dyn Detector>>) -> Self {
        Self { detectors }
    }

    /// The canonical detection operation: run `path` through every registered
    /// detector. Infallible by construction — see [`Detection`].
    pub fn detect_project(&self, path: &Path) -> Detection {
        self.inspect(path, None)
    }

    /// Like [`detect_project`](Self::detect_project), but when `only` is
    /// `Some(kind)` only the detector whose [`Detector::kind`] equals `kind`
    /// runs (for per-tracker re-detect). An unknown `kind` matches nothing
    /// and yields an empty [`Detection`].
    pub fn inspect(&self, path: &Path, only: Option<&str>) -> Detection {
        let mut outcomes = Vec::new();
        for detector in &self.detectors {
            let kind = detector.kind();
            if only.is_some_and(|k| k != kind) {
                continue;
            }
            outcomes.push(match detector.detect(path) {
                Ok(Some(tracker)) => DetectorOutcome::Detected { kind, tracker },
                Ok(None) => DetectorOutcome::NotDetected { kind },
                Err(error) => DetectorOutcome::Failed { kind, error },
            });
        }
        Detection { outcomes }
    }
}
```

- [ ] **Step 4: Update re-exports**

In `src-tauri/src/detectors/mod.rs`, change the `runner` re-export line to:

```rust
pub use runner::{Detection, DetectorOutcome, DetectorRunner};
```

- [ ] **Step 5: Update the three command call sites**

In `src-tauri/src/commands/projects.rs`:

`create_project` — replace the detection block:

```rust
    let detection = detectors.detect_project(Path::new(&project.directory));
    project.trackers = detection.trackers();
    for error in detection.errors() {
        eprintln!("Detector error for '{}': {}", project.directory, error);
    }
```

`refresh_project_trackers` — the `project.trackers = …` assignment becomes:

```rust
    project.trackers = detectors
        .detect_project(Path::new(&project.directory))
        .into_result()
        .map_err(|e| ProjectError::Detection(e.to_string()))?;
```

(unchanged from today except `detect_project` now returns the new `Detection` — `.into_result()` still applies)

`detect_project_trackers` — replace the body:

```rust
    let detection = detectors.detect_project(Path::new(&directory));
    for error in detection.errors() {
        eprintln!("Detector error previewing '{}': {}", directory, error);
    }
    detection.trackers()
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --lib`
Expected: PASS. (`web_url` in `sample_git_tracker` is NOT present yet — Task 3 adds it. If you accidentally left it in, remove it now.)

- [ ] **Step 7: Format, clippy, commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --lib 2>&1 | grep -c "^warning:"
```

Expected: `3` (the 2 pre-existing warnings + the summary line — i.e. no new warnings). If higher, fix.

```bash
git add src-tauri/src/detectors/ src-tauri/src/commands/projects.rs
git commit -m "$(cat <<'EOF'
refactor(detectors): Detection carries one outcome per detector

Detection { trackers, errors } becomes Detection { outcomes: Vec<DetectorOutcome> }
where each outcome is Detected / NotDetected / Failed, tagged with the
detector kind. trackers() / errors() / into_result() are accessors over it;
into_result semantics unchanged (the recorded all-or-nothing decision).

Adds DetectorRunner::inspect(path, only) to run a single detector by kind.
This is what lets the UI say "unity — not detected" and "blender — failed".

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: `GitInfo.web_url`

**Files:**
- Modify: `src-tauri/src/models/git.rs`
- Modify: `src-tauri/src/detectors/git/gitector.rs` (add `web_url` fn, populate the field, tests)
- Modify: `src-tauri/src/detectors/runner.rs` (`sample_git_tracker` — add `web_url: None`)
- Modify: `src/lib/api/types.ts` (`GitInfo.web_url`)

**Interfaces:**
- Consumes: nothing new.
- Produces: `GitInfo { …, pub web_url: Option<String> }`. `gitector::web_url(remote: &str) -> Option<String>` (private).

- [ ] **Step 1: Write the failing tests**

In `src-tauri/src/detectors/git/gitector.rs`, inside `mod tests`, add:

```rust
    #[test]
    fn web_url_normalizes_common_remote_forms() {
        assert_eq!(
            web_url("git@github.com:acme/repo.git").as_deref(),
            Some("https://github.com/acme/repo")
        );
        assert_eq!(
            web_url("ssh://git@gitlab.com/acme/repo.git").as_deref(),
            Some("https://gitlab.com/acme/repo")
        );
        assert_eq!(
            web_url("https://github.com/acme/repo.git").as_deref(),
            Some("https://github.com/acme/repo")
        );
        assert_eq!(
            web_url("https://github.com/acme/repo").as_deref(),
            Some("https://github.com/acme/repo")
        );
        assert_eq!(web_url("/srv/git/repo.git"), None);
        assert_eq!(web_url(""), None);
    }

    #[test]
    fn get_info_derives_web_url_from_an_ssh_remote() {
        let dir = temp_dir("web-url");
        let repo = init_repo(&dir);
        repo.remote("origin", "git@github.com:acme/friction-engine.git")
            .expect("should add remote");

        let tracker = Gitector
            .detect(&dir)
            .expect("should detect")
            .expect("should recognize the repo");
        let Tracker::Git(info) = tracker else {
            panic!("expected Tracker::Git");
        };

        assert_eq!(
            info.web_url.as_deref(),
            Some("https://github.com/acme/friction-engine")
        );
        std::fs::remove_dir_all(&dir).ok();
    }
```

Also add `web_url: None,` to `sample_git_tracker()` in `src-tauri/src/detectors/runner.rs` (the tests module).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib`
Expected: FAIL to compile — `web_url` fn missing, `GitInfo` has no `web_url` field.

- [ ] **Step 3: Add the field**

In `src-tauri/src/models/git.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub repo_root: String,
    pub dirty: bool,
    pub detached_head: bool,
    pub repo_url: Option<String>,
    /// Browser-openable form of `repo_url` (SSH → HTTPS, trailing `.git`
    /// stripped), or `None` if `repo_url` isn't a recognizable http/ssh
    /// git remote. `Option` so records written before this field load fine.
    pub web_url: Option<String>,
    pub contributors: Vec<String>,
    pub curr_branch: Option<String>,
    pub branches: Option<Vec<String>>,
    pub commit_hash: Option<String>,
}
```

- [ ] **Step 4: Add the `web_url` helper and populate the field**

In `src-tauri/src/detectors/git/gitector.rs`, add this free function near the other helpers (e.g. after `remote_url`):

```rust
/// Browser-openable form of a git remote URL, or `None` if it isn't a
/// recognizable http(s)/ssh git remote (a bare local path, say).
///
/// `git@host:owner/repo.git` and `ssh://git@host/owner/repo.git` and
/// `https://host/owner/repo.git` all normalize to `https://host/owner/repo`.
fn web_url(remote: &str) -> Option<String> {
    let remote = remote.trim();
    if remote.is_empty() {
        return None;
    }

    let (host, path) = if let Some(rest) = remote.strip_prefix("git@") {
        rest.split_once(':')?
    } else if let Some(rest) = remote
        .strip_prefix("ssh://git@")
        .or_else(|| remote.strip_prefix("ssh://"))
    {
        rest.split_once('/')?
    } else if let Some(rest) = remote
        .strip_prefix("https://")
        .or_else(|| remote.strip_prefix("http://"))
    {
        rest.split_once('/')?
    } else {
        return None;
    };

    let path = path.strip_suffix('/').unwrap_or(path);
    let path = path.strip_suffix(".git").unwrap_or(path);
    if host.is_empty() || path.is_empty() {
        return None;
    }
    Some(format!("https://{host}/{path}"))
}
```

In `Gitector::detect`, capture the remote once and derive both fields. Replace:

```rust
        Ok(Some(Tracker::Git(GitInfo {
            repo_root: root,
            dirty,
            detached_head: is_detached(&repo)?,
            repo_url: remote_url(&repo, "origin")?,
```

with:

```rust
        let repo_url = remote_url(&repo, "origin")?;
        let web_url = repo_url.as_deref().and_then(web_url);

        Ok(Some(Tracker::Git(GitInfo {
            repo_root: root,
            dirty,
            detached_head: is_detached(&repo)?,
            repo_url,
            web_url,
```

- [ ] **Step 5: Mirror in the frontend types**

In `src/lib/api/types.ts`, add to the `GitInfo` interface (after `repo_url`):

```typescript
  web_url: string | null;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --lib`
Expected: PASS.
Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 7: Format and commit**

```bash
cd src-tauri && cargo fmt
git add src-tauri/src/models/git.rs src-tauri/src/detectors/ src/lib/api/types.ts
git commit -m "$(cat <<'EOF'
feat(git): derive a browser-openable web_url from the remote

GitInfo.web_url normalizes the origin remote (git@…, ssh://…, https://…​.git)
to https://host/owner/repo, or None when it isn't a recognizable git remote.
Option-typed so pre-existing stored records load unchanged.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: `inspect_project` command

**Files:**
- Create: `src-tauri/src/commands/inspect.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs` (`invoke_handler`)

**Interfaces:**
- Consumes: `DetectorRunner::inspect` + `Detection` + `DetectorOutcome` (Task 2); `Project`, `ProjectStore`, `ProjectError`, `Project::check_directory_health`.
- Produces: Tauri command `inspect_project(id: String, only: Option<String>) -> Result<ProjectInspection, ProjectError>`.
  - `ProjectInspection { project: Project, directory_status: DirectoryStatus, results: Vec<DetectorResult> }`
  - `DirectoryStatus { ok: bool, message: Option<String> }`
  - `DetectorResult { kind: String, status: DetectorStatus, tracker: Option<Tracker>, error: Option<String> }`
  - `DetectorStatus` serializes as `"detected"` / `"not_detected"` / `"failed"`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/commands/inspect.rs` containing only the tests-first skeleton:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::detectors::{Detection, DetectorOutcome};
    use crate::errors::DetectorError;
    use crate::models::git::GitInfo;
    use crate::models::tracker::Tracker;

    fn sample_git_tracker() -> Tracker {
        Tracker::Git(GitInfo {
            repo_root: "/tmp/x".to_string(),
            dirty: false,
            detached_head: false,
            repo_url: None,
            web_url: None,
            contributors: Vec::new(),
            curr_branch: Some("main".to_string()),
            branches: None,
            commit_hash: None,
        })
    }

    #[test]
    fn results_from_maps_every_outcome_variant() {
        let detection = Detection {
            outcomes: vec![
                DetectorOutcome::Detected {
                    kind: "git",
                    tracker: sample_git_tracker(),
                },
                DetectorOutcome::NotDetected { kind: "unreal" },
                DetectorOutcome::Failed {
                    kind: "unity",
                    error: DetectorError::Other("boom".into()),
                },
            ],
        };

        let results = results_from(detection);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].kind, "git");
        assert!(matches!(results[0].status, DetectorStatus::Detected));
        assert!(results[0].tracker.is_some());
        assert!(matches!(results[1].status, DetectorStatus::NotDetected));
        assert!(results[1].tracker.is_none());
        assert!(matches!(results[2].status, DetectorStatus::Failed));
        assert_eq!(results[2].error.as_deref(), Some("boom"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Add `pub mod inspect;` to `src-tauri/src/commands/mod.rs` first (so the file compiles into the crate), then:

Run: `cd src-tauri && cargo test --lib commands::inspect`
Expected: FAIL to compile — `results_from`, `DetectorStatus` not defined.

- [ ] **Step 3: Implement the module**

Prepend to `src-tauri/src/commands/inspect.rs` (above the `#[cfg(test)] mod tests`):

```rust
use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::detectors::{Detection, DetectorOutcome, DetectorRunner};
use crate::errors::ProjectError;
use crate::models::{Project, Tracker};
use crate::store::ProjectStore;

/// Read-only snapshot of a project plus a live detection pass. Nothing is
/// persisted — `refresh_project_trackers` is the write path.
#[derive(Serialize)]
pub struct ProjectInspection {
    pub project: Project,
    pub directory_status: DirectoryStatus,
    pub results: Vec<DetectorResult>,
}

/// Whether the project's directory is currently usable. When `ok` is false
/// `results` is empty and `message` carries the reason.
#[derive(Serialize)]
pub struct DirectoryStatus {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// One registered detector's outcome for this project's directory.
#[derive(Serialize)]
pub struct DetectorResult {
    pub kind: String,
    pub status: DetectorStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracker: Option<Tracker>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum DetectorStatus {
    Detected,
    NotDetected,
    Failed,
}

fn results_from(detection: Detection) -> Vec<DetectorResult> {
    detection
        .outcomes
        .into_iter()
        .map(|outcome| match outcome {
            DetectorOutcome::Detected { kind, tracker } => DetectorResult {
                kind: kind.to_string(),
                status: DetectorStatus::Detected,
                tracker: Some(tracker),
                error: None,
            },
            DetectorOutcome::NotDetected { kind } => DetectorResult {
                kind: kind.to_string(),
                status: DetectorStatus::NotDetected,
                tracker: None,
                error: None,
            },
            DetectorOutcome::Failed { kind, error } => DetectorResult {
                kind: kind.to_string(),
                status: DetectorStatus::Failed,
                tracker: None,
                error: Some(error.to_string()),
            },
        })
        .collect()
}

/// Loads a project and runs detection against its directory **without
/// persisting**. A missing/inaccessible directory is reported via
/// `directory_status` (with empty `results`), not as a command error, so the
/// view can still render the project's identity. `only = Some(kind)` re-runs
/// just that one detector.
#[tauri::command]
pub fn inspect_project(
    app: AppHandle,
    detectors: State<'_, DetectorRunner>,
    id: String,
    only: Option<String>,
) -> Result<ProjectInspection, ProjectError> {
    let store = ProjectStore::new(&app)?;
    let project = store
        .get_project(&id)?
        .ok_or_else(|| ProjectError::NotFound(id.clone()))?;

    let (directory_status, results) = match Project::check_directory_health(&project.directory) {
        Ok(()) => {
            let detection =
                detectors.inspect(Path::new(&project.directory), only.as_deref());
            (
                DirectoryStatus {
                    ok: true,
                    message: None,
                },
                results_from(detection),
            )
        }
        Err(error) => (
            DirectoryStatus {
                ok: false,
                message: Some(error.to_string()),
            },
            Vec::new(),
        ),
    };

    Ok(ProjectInspection {
        project,
        directory_status,
        results,
    })
}
```

- [ ] **Step 4: Register the command**

In `src-tauri/src/lib.rs`, add to the `tauri::generate_handler![…]` list (after `commands::projects::detect_project_trackers`):

```rust
            commands::inspect::inspect_project,
```

- [ ] **Step 5: Run tests + build to verify**

Run: `cd src-tauri && cargo test --lib`
Expected: PASS.
Run: `cd src-tauri && cargo build`
Expected: builds clean (confirms the command signature is valid for Tauri).

- [ ] **Step 6: Format, clippy, commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --lib 2>&1 | grep "^warning:"
```

Expected: only the 2 pre-existing warnings.

```bash
git add src-tauri/src/commands/ src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(commands): add read-only inspect_project

Loads a project and runs a live detection pass without persisting, returning
per-detector results (detected / not_detected / failed) plus a directory
status. Backs the /project/[id] view; refresh stays the only write path.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Frontend API layer

**Files:**
- Modify: `src/lib/api/types.ts`
- Modify: `src/lib/api/projects.ts`
- Modify: `src/lib/api/opener.ts`

**Interfaces:**
- Consumes: the `inspect_project` command (Task 4); `@tauri-apps/plugin-opener`.
- Produces:
  - `types.ts`: `DetectorStatus`, `DetectorResult`, `DirectoryStatus`, `ProjectInspection`
  - `projects.ts`: `inspectProject(id: string, opts?: { only?: string }): Promise<ProjectInspection>`
  - `opener.ts`: `openExternalUrl(url: string): Promise<void>`, `revealPath(path: string): Promise<void>`

- [ ] **Step 1: Add the types**

In `src/lib/api/types.ts`, after the `Tracker` type definition, add:

```typescript
// Mirrors src-tauri/src/commands/inspect.rs
export type DetectorStatus = "detected" | "not_detected" | "failed";

export interface DetectorResult {
  kind: string;
  status: DetectorStatus;
  tracker?: Tracker;
  error?: string;
}

export interface DirectoryStatus {
  ok: boolean;
  message?: string;
}

export interface ProjectInspection {
  project: Project;
  directory_status: DirectoryStatus;
  results: DetectorResult[];
}
```

- [ ] **Step 2: Add the `inspectProject` wrapper**

In `src/lib/api/projects.ts`, add the import of `ProjectInspection` to the existing type import, then append:

```typescript
// Read-only: loads a project and runs a live detection pass without
// persisting. `only` re-runs a single detector by kind. Backs /project/[id].
export async function inspectProject(
  id: string,
  opts?: { only?: string },
): Promise<ProjectInspection> {
  try {
    return await invoke<ProjectInspection>("inspect_project", {
      id,
      only: opts?.only ?? null,
    });
  } catch (err) {
    throw toError(err);
  }
}
```

- [ ] **Step 3: Add the opener wrappers**

In `src/lib/api/opener.ts`, add at the top with the other imports:

```typescript
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
```

and append:

```typescript
// Opens a URL in the system browser.
export async function openExternalUrl(url: string): Promise<void> {
  try {
    await openUrl(url);
  } catch (err) {
    throw toError(err);
  }
}

// Reveals a file or directory in the system file explorer.
export async function revealPath(path: string): Promise<void> {
  try {
    await revealItemInDir(path);
  } catch (err) {
    throw toError(err);
  }
}
```

- [ ] **Step 4: Verify**

Run: `npm run check`
Expected: 0 errors. If `@tauri-apps/plugin-opener` doesn't export `openUrl` / `revealItemInDir`, check `node_modules/@tauri-apps/plugin-opener/dist-js/index.d.ts` for the real names and adjust (v2 uses `openUrl`, `openPath`, `revealItemInDir`).

- [ ] **Step 5: Commit**

```bash
git add src/lib/api/
git commit -m "$(cat <<'EOF'
feat(api): inspectProject wrapper + external-open / reveal-path helpers

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Typed `trackerFields` + `TrackerPanel`

**Files:**
- Modify: `vite.config.ts` (vitest `test` block), `package.json` (dev-dep + script)
- Modify: `src/lib/trackers.ts`
- Create: `src/lib/trackers.test.ts`
- Create: `src/lib/components/TrackerPanel.svelte`
- Modify: `src/lib/components/ProjectDetailModal.svelte` (adopt `TrackerPanel`)

**Interfaces:**
- Consumes: `openExternalUrl`, `revealPath` (Task 5); `Tracker` type.
- Produces:
  - `trackers.ts`: `type FieldType = "text" | "code" | "link" | "path" | "chips" | "flag"`; `interface TrackerField { label: string; type: FieldType; text: string; items: string[] }`; `trackerFields(tracker: Tracker): TrackerField[]` (new shape); `trackerKind` unchanged.
  - `TrackerPanel.svelte`: `<TrackerPanel tracker={tracker} />`.

**Note — refines spec rule 3:** a `link` field is produced only when the *value* is `http(s)://…`; an `ssh`/`git@` value becomes `code` (copyable, no broken "open" affordance). Same intent as the spec ("one clickable, one copyable"), cleaner.

- [ ] **Step 1: Add vitest**

```bash
npm install -D vitest
```

In `package.json` `"scripts"`, add:

```json
    "test": "vitest run",
```

In `vite.config.ts`, add a `test` property to the config object:

```typescript
  test: {
    include: ["src/**/*.test.ts"],
    environment: "node",
  },
```

(If `vite.config.ts` lacks the `/// <reference types="vitest/config" />` triple-slash directive and TS complains about the `test` key, add `/// <reference types="vitest/config" />` as the file's first line.)

- [ ] **Step 2: Write the failing tests**

Create `src/lib/trackers.test.ts`:

```typescript
import { describe, expect, it } from "vitest";
import { trackerFields, trackerKind } from "./trackers";
import type { Tracker } from "./api/types";

const gitTracker = (over: Record<string, unknown> = {}): Tracker =>
  ({
    Git: {
      repo_root: "D:\\Games\\friction-engine",
      dirty: true,
      detached_head: false,
      repo_url: "git@github.com:acme/friction-engine.git",
      web_url: "https://github.com/acme/friction-engine",
      contributors: [],
      curr_branch: "main",
      branches: ["main", "develop"],
      commit_hash: "a1b2c3d4",
      ...over,
    },
  }) as unknown as Tracker;

describe("trackerKind", () => {
  it("reads the variant key", () => {
    expect(trackerKind(gitTracker())).toBe("Git");
  });
  it("reads a bare string variant", () => {
    expect(trackerKind("Unity" as unknown as Tracker)).toBe("Unity");
  });
});

describe("trackerFields typing", () => {
  const byLabel = (t: Tracker) =>
    Object.fromEntries(trackerFields(t).map((f) => [f.label, f]));

  it("types an http url as a link", () => {
    expect(byLabel(gitTracker())["Web url"]).toMatchObject({
      type: "link",
      text: "https://github.com/acme/friction-engine",
    });
  });

  it("types an ssh remote as code, not a broken link", () => {
    expect(byLabel(gitTracker())["Repo url"].type).toBe("code");
  });

  it("types *_root / *_path keys as path", () => {
    expect(byLabel(gitTracker())["Repo root"].type).toBe("path");
  });

  it("types commit-hash keys as code", () => {
    expect(byLabel(gitTracker())["Commit hash"].type).toBe("code");
  });

  it("types arrays as chips and drops empty ones", () => {
    const f = byLabel(gitTracker());
    expect(f["Branches"]).toMatchObject({ type: "chips", items: ["main", "develop"] });
    expect(f["Contributors"]).toBeUndefined();
  });

  it("shows a true bool as a flag and hides a false one", () => {
    const f = byLabel(gitTracker());
    expect(f["Dirty"].type).toBe("flag");
    expect(f["Detached head"]).toBeUndefined();
  });

  it("omits null / empty-string values", () => {
    expect(byLabel(gitTracker({ commit_hash: null }))["Commit hash"]).toBeUndefined();
  });

  it("falls back to text", () => {
    expect(byLabel(gitTracker())["Curr branch"].type).toBe("text");
  });
});
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `npm test`
Expected: FAIL — `trackerFields` returns the old `{ label, value, isLink }` shape; `f["Web url"].type` is undefined etc.

- [ ] **Step 4: Rewrite `trackers.ts`**

Replace the contents of `src/lib/trackers.ts` with:

```typescript
// Generic helpers over `Tracker` that work for any variant, including ones
// added after this file was written. Field *semantics* (is this a link? a
// path? a copyable id?) are inferred from the key name and value shape — see
// `inferType` — so no per-tracker-kind code lives here or in the UI. Naming a
// detector's `*Info` fields per the convention below is how a field gets an
// affordance:
//   *_url / *_root / *_path / *_dir  → link|path      *hash* / *commit* → code
//   arrays → chips                    booleans → flag (shown only when true)
import type { Tracker } from "./api/types";

export function trackerKind(tracker: Tracker): string {
  return typeof tracker === "string" ? tracker : Object.keys(tracker)[0];
}

function trackerPayload(tracker: Tracker): Record<string, unknown> | null {
  if (typeof tracker === "string") return null;
  const kind = trackerKind(tracker);
  return (tracker as Record<string, unknown>)[kind] as Record<string, unknown>;
}

export type FieldType = "text" | "code" | "link" | "path" | "chips" | "flag";

export interface TrackerField {
  label: string;
  type: FieldType;
  /** Display/copy text for text|code|link|path. Empty for chips|flag. */
  text: string;
  /** Chip values; empty otherwise. */
  items: string[];
}

function humanizeKey(key: string): string {
  const spaced = key.replace(/_/g, " ");
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

function inferType(key: string, value: unknown): FieldType | null {
  if (typeof value === "boolean") return "flag";
  if (Array.isArray(value)) return value.length > 0 ? "chips" : null;
  if (value === null || value === undefined || value === "") return null;
  const s = String(value);
  if (/^https?:\/\//i.test(s)) return "link";
  if (/^(git@|ssh:\/\/)/i.test(s)) return "code";
  if (/(^|_)(path|root|dir)$|directory/i.test(key)) return "path";
  if (/hash|commit/i.test(key)) return "code";
  return "text";
}

export function trackerFields(tracker: Tracker): TrackerField[] {
  const payload = trackerPayload(tracker);
  if (!payload) return [];

  const fields: TrackerField[] = [];
  for (const [key, value] of Object.entries(payload)) {
    const type = inferType(key, value);
    if (type === null) continue;

    if (type === "flag") {
      if (value === true) fields.push({ label: humanizeKey(key), type, text: "", items: [] });
      continue;
    }
    if (type === "chips") {
      fields.push({
        label: humanizeKey(key),
        type,
        text: "",
        items: (value as unknown[]).map(String),
      });
      continue;
    }
    fields.push({ label: humanizeKey(key), type, text: String(value), items: [] });
  }
  return fields;
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `npm test`
Expected: PASS.

- [ ] **Step 6: Create `TrackerPanel.svelte`**

Create `src/lib/components/TrackerPanel.svelte`:

```svelte
<script lang="ts">
  import { openExternalUrl, revealPath } from "$lib/api/opener";
  import type { Tracker } from "$lib/api/types";
  import { trackerFields } from "$lib/trackers";

  let { tracker, onerror }: { tracker: Tracker; onerror?: (m: string) => void } = $props();

  let fields = $derived(trackerFields(tracker));

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
    } catch (err) {
      console.warn("clipboard write failed", err);
    }
  }

  async function open(url: string) {
    try {
      await openExternalUrl(url);
    } catch (err) {
      onerror?.((err as Error).message);
    }
  }

  async function reveal(path: string) {
    try {
      await revealPath(path);
    } catch (err) {
      onerror?.((err as Error).message);
    }
  }

  const iconBtn =
    "text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 text-xs";
</script>

{#if fields.length === 0}
  <p class="text-sm text-gray-500 dark:text-gray-400">No details available.</p>
{:else}
  <dl class="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-2 text-sm">
    {#each fields as field}
      <dt class="text-gray-500 dark:text-gray-400">{field.label}</dt>
      <dd class="min-w-0 break-all text-gray-900 dark:text-gray-100">
        {#if field.type === "flag"}
          <span
            class="rounded-full bg-amber-100 px-2 py-0.5 text-xs text-amber-800 dark:bg-amber-950 dark:text-amber-300"
          >
            {field.label}
          </span>
        {:else if field.type === "chips"}
          <span class="flex flex-wrap gap-1">
            {#each field.items as item}
              <span
                class="rounded bg-gray-100 px-1.5 py-0.5 text-xs dark:bg-gray-700"
              >{item}</span>
            {/each}
          </span>
        {:else if field.type === "link"}
          <a
            href={field.text}
            target="_blank"
            rel="noreferrer"
            class="text-blue-600 hover:underline dark:text-blue-400"
          >{field.text}</a>
          <button type="button" class={iconBtn} onclick={() => open(field.text)}>↗ open</button>
          <button type="button" class={iconBtn} onclick={() => copy(field.text)}>⧉ copy</button>
        {:else if field.type === "path"}
          <span class="font-mono text-xs">{field.text}</span>
          <button type="button" class={iconBtn} onclick={() => reveal(field.text)}>📂 reveal</button>
          <button type="button" class={iconBtn} onclick={() => copy(field.text)}>⧉ copy</button>
        {:else if field.type === "code"}
          <span class="font-mono text-xs">{field.text}</span>
          <button type="button" class={iconBtn} onclick={() => copy(field.text)}>⧉ copy</button>
        {:else}
          {field.text}
        {/if}
      </dd>
    {/each}
  </dl>
{/if}
```

- [ ] **Step 7: Adopt `TrackerPanel` in `ProjectDetailModal`**

In `src/lib/components/ProjectDetailModal.svelte`: remove the `trackerFields` import (keep `trackerKind`), import `TrackerPanel`, and replace the `{#each project.trackers as tracker, i}` tab-panel block (the `{@const fields = trackerFields(tracker)}` … `</div>`) with:

```svelte
      {#each project.trackers as tracker, i}
        {#if activeIndex === i}
          <div role="tabpanel" class="p-3">
            <TrackerPanel {tracker} />
          </div>
        {/if}
      {/each}
```

Add the import: `import TrackerPanel from "./TrackerPanel.svelte";`

- [ ] **Step 8: Verify**

Run: `npm test` → PASS
Run: `npm run check` → 0 errors

- [ ] **Step 9: Commit**

```bash
git add package.json package-lock.json vite.config.ts src/lib/trackers.ts src/lib/trackers.test.ts src/lib/components/TrackerPanel.svelte src/lib/components/ProjectDetailModal.svelte
git commit -m "$(cat <<'EOF'
feat(trackers): type-inferred fields + generic TrackerPanel

trackerFields now returns typed fields (text/code/link/path/chips/flag)
inferred from key names and value shape — no per-tracker-kind code. New
TrackerPanel renders them with open/reveal/copy affordances; ProjectDetailModal
adopts it. Adds vitest with coverage for the inference rules.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: `ProjectIdentity` + the `/project/[id]` route

**Files:**
- Create: `src/lib/components/ProjectIdentity.svelte`
- Create: `src/routes/project/[id]/+page.ts`
- Create: `src/routes/project/[id]/+page.svelte`

**Interfaces:**
- Consumes: `inspectProject` (Task 5); `TrackerPanel` (Task 6); `EditProjectForm`, `ErrorBanner` (existing); `refreshProjectTrackers`, `openProjectDirectory`, `isOpenWithAppMissing` (existing); `page` from `$app/stores`.
- Produces: `ProjectIdentity.svelte` (`<ProjectIdentity {project} />`); the route at `/project/<id>`.

- [ ] **Step 1: Create `ProjectIdentity.svelte`**

Lift the identity `<dl>` out of `ProjectDetailModal`. Create `src/lib/components/ProjectIdentity.svelte`:

```svelte
<script lang="ts">
  import type { Project } from "$lib/api/types";

  let { project }: { project: Project } = $props();

  function formatDate(iso: string | null): string {
    return iso ? new Date(iso).toLocaleString() : "—";
  }
</script>

<dl class="grid grid-cols-[max-content_1fr] gap-x-3 gap-y-1 text-sm">
  <dt class="text-gray-500 dark:text-gray-400">Directory</dt>
  <dd class="break-all text-gray-900 dark:text-gray-100">{project.directory}</dd>

  {#if project.description}
    <dt class="text-gray-500 dark:text-gray-400">Description</dt>
    <dd class="text-gray-900 dark:text-gray-100">{project.description}</dd>
  {/if}
  {#if project.client}
    <dt class="text-gray-500 dark:text-gray-400">Client</dt>
    <dd class="text-gray-900 dark:text-gray-100">{project.client}</dd>
  {/if}
  {#if project.tags.length > 0}
    <dt class="text-gray-500 dark:text-gray-400">Tags</dt>
    <dd class="text-gray-900 dark:text-gray-100">{project.tags.join(", ")}</dd>
  {/if}
  {#if project.notes}
    <dt class="text-gray-500 dark:text-gray-400">Notes</dt>
    <dd class="text-gray-900 dark:text-gray-100">{project.notes}</dd>
  {/if}

  <dt class="text-gray-500 dark:text-gray-400">Created</dt>
  <dd class="text-gray-900 dark:text-gray-100">{formatDate(project.created_at)}</dd>
  <dt class="text-gray-500 dark:text-gray-400">Last opened</dt>
  <dd class="text-gray-900 dark:text-gray-100">{formatDate(project.last_opened_at)}</dd>
</dl>
```

- [ ] **Step 2: Create `+page.ts`**

Create `src/routes/project/[id]/+page.ts`:

```typescript
// Dynamic route under adapter-static + SPA fallback: not prerendered.
export const prerender = false;
```

- [ ] **Step 3: Create the route component**

Create `src/routes/project/[id]/+page.svelte`:

```svelte
<script lang="ts">
  import { page } from "$app/stores";
  import { isOpenWithAppMissing, openProjectDirectory } from "$lib/api/opener";
  import { inspectProject, refreshProjectTrackers } from "$lib/api/projects";
  import type { ProjectInspection } from "$lib/api/types";
  import EditProjectForm from "$lib/components/EditProjectForm.svelte";
  import ErrorBanner from "$lib/components/ErrorBanner.svelte";
  import ProjectIdentity from "$lib/components/ProjectIdentity.svelte";
  import TrackerPanel from "$lib/components/TrackerPanel.svelte";
  import { buttonClass } from "$lib/components/styles";
  import { trackerKind } from "$lib/trackers";

  let id = $derived($page.params.id);

  let inspection = $state<ProjectInspection | null>(null);
  let loadError = $state("");
  let banner = $state("");
  let loading = $state(false);
  let editing = $state(false);
  let activeKind = $state<string | null>(null);

  async function load(only?: string) {
    loading = true;
    banner = "";
    try {
      const next = await inspectProject(id, only ? { only } : undefined);
      if (only && inspection) {
        // merge a single re-detected result
        const merged = inspection.results.map((r) =>
          r.kind === only ? next.results.find((n) => n.kind === only) ?? r : r,
        );
        inspection = { ...next, results: merged };
      } else {
        inspection = next;
      }
      loadError = "";
    } catch (err) {
      loadError = (err as Error).message;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    // re-runs when `id` changes
    void id;
    load();
  });

  let detected = $derived(inspection?.results.filter((r) => r.status === "detected") ?? []);

  $effect(() => {
    if (detected.length > 0 && !detected.some((r) => r.kind === activeKind)) {
      activeKind = detected[0].kind;
    }
  });

  async function handleOpen() {
    try {
      await openProjectDirectory(id);
      await load();
    } catch (err) {
      banner = isOpenWithAppMissing(err)
        ? "The app configured for this project can't be found."
        : (err as Error).message;
    }
  }

  async function handleRefresh() {
    banner = "";
    try {
      await refreshProjectTrackers(id);
    } catch (err) {
      banner = (err as Error).message;
    }
    await load();
  }

  async function handleSaved() {
    editing = false;
    await load();
  }
</script>

<main class="mx-auto max-w-3xl px-4 py-8">
  <a href="/" class="text-sm text-blue-600 hover:underline dark:text-blue-400">← All projects</a>

  {#if loadError}
    <div class="mt-4">
      <ErrorBanner message={loadError} />
    </div>
  {:else if inspection}
    <div class="mt-3 flex items-start justify-between gap-4">
      <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">
        {inspection.project.name}
      </h1>
      <div class="flex shrink-0 gap-2">
        <button type="button" class={buttonClass} onclick={handleOpen}>Open</button>
        <button type="button" class={buttonClass} onclick={() => (editing = true)}>Edit</button>
        <button type="button" class={buttonClass} onclick={handleRefresh} disabled={loading}>
          {loading ? "…" : "Refresh"}
        </button>
      </div>
    </div>

    <div class="mt-3"><ErrorBanner message={banner} /></div>

    <div class="mt-2 rounded-lg bg-white p-4 shadow-sm dark:bg-gray-800">
      <ProjectIdentity project={inspection.project} />
    </div>

    {#if !inspection.directory_status.ok}
      <p class="mt-4 rounded-md bg-amber-100 p-3 text-sm text-amber-900 dark:bg-amber-950 dark:text-amber-200">
        {inspection.directory_status.message ?? "This project's directory is unavailable."}
      </p>
    {:else}
      <div class="mt-4 flex flex-wrap gap-x-4 gap-y-1 text-xs">
        {#each inspection.results as r}
          <span>
            {#if r.status === "detected"}
              <span class="text-green-600 dark:text-green-400">●</span> {r.kind}
            {:else if r.status === "not_detected"}
              <span class="text-gray-400">○</span>
              <span class="text-gray-500 dark:text-gray-400">{r.kind} — not detected</span>
            {:else}
              <span class="text-red-600 dark:text-red-400">▲</span>
              <span class="text-red-600 dark:text-red-400">{r.kind} — {r.error}</span>
            {/if}
          </span>
        {/each}
      </div>

      {#if detected.length > 0}
        <div class="mt-3 flex flex-wrap gap-1 border-b border-gray-200 dark:border-gray-700">
          {#each detected as r}
            <button
              type="button"
              onclick={() => (activeKind = r.kind)}
              class={`rounded-t-md px-3 py-1.5 text-sm font-medium ${
                activeKind === r.kind
                  ? "bg-gray-100 text-gray-900 dark:bg-gray-700 dark:text-gray-100"
                  : "text-gray-500 hover:text-gray-700 dark:text-gray-400"
              }`}
            >
              {trackerKind(r.tracker!)}
            </button>
          {/each}
        </div>

        {#each detected as r}
          {#if activeKind === r.kind}
            <div class="p-3">
              <div class="mb-2 flex justify-end">
                <button
                  type="button"
                  class="text-xs text-gray-400 hover:text-gray-700 dark:hover:text-gray-200"
                  onclick={() => load(r.kind)}
                >
                  re-detect
                </button>
              </div>
              <TrackerPanel tracker={r.tracker!} onerror={(m) => (banner = m)} />
            </div>
          {/if}
        {/each}
      {/if}
    {/if}
  {/if}
</main>

{#if editing && inspection}
  <div
    class="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 p-4"
    role="presentation"
    onclick={() => (editing = false)}
    onkeydown={(e) => e.key === "Escape" && (editing = false)}
  >
    <div
      class="w-11/12 max-w-lg"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <EditProjectForm
        project={inspection.project}
        onSaved={handleSaved}
        onCancel={() => (editing = false)}
        onerror={(m) => (banner = m)}
      />
    </div>
  </div>
{/if}
```

**Note:** confirm `EditProjectForm`'s actual prop names (`onSaved` / `onCancel` / `onerror`) by reading `src/lib/components/EditProjectForm.svelte` and adjust the invocation to match. Same for `ErrorBanner` (`message` prop).

- [ ] **Step 4: Verify**

Run: `npm run check` → 0 errors
Run: `npm run build` → succeeds (this is the real test that adapter-static accepts the dynamic route; if it fails with a prerender error, confirm `+page.ts` has `prerender = false` and `+layout.ts` has `ssr = false`).

- [ ] **Step 5: Manual smoke**

```bash
npm run tauri dev
```

Navigate to `http://localhost:1420/project/<paste a real project id>` (get one from the running app's list, or from `projects.json`). Verify: identity block renders, status strip shows git/unreal, a tracker tab renders fields with open/copy/reveal buttons, Refresh works, Edit opens and saves.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/ProjectIdentity.svelte "src/routes/project/[id]"
git commit -m "$(cat <<'EOF'
feat(ui): /project/[id] route with per-tracker tabs and live status

New client-side route: identity block, a detection-status strip (detected /
not-detected / failed), one tab per detected tracker rendered via TrackerPanel,
plus Open / Edit / Refresh and per-tab re-detect.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Route the "Details" button; delete the modal

**Files:**
- Modify: `src/lib/components/ProjectCard.svelte` (Details button → link)
- Modify: `src/lib/components/ProjectList.svelte` (drop `onShowDetails`)
- Modify: `src/routes/+page.svelte` (drop detail-modal state)
- Modify: `src/lib/components/FavoritesModal.svelte`, `src/lib/components/BinModal.svelte` (only if they pass `onShowDetails`)
- Delete: `src/lib/components/ProjectDetailModal.svelte`
- Modify: `src-tauri/capabilities/default.json` (if `revealItemInDir` needs a permission)

**Interfaces:**
- Consumes: the route from Task 7.
- Produces: nothing new; removes `onShowDetails` from the `ProjectCard` / `ProjectList` prop contracts.

- [ ] **Step 1: `ProjectCard` — Details becomes a link**

In `src/lib/components/ProjectCard.svelte`: remove `onShowDetails` from the `$props()` destructure and its type; replace the Details `<button>` with:

```svelte
    <a
      href={`/project/${project.id}`}
      class={buttonClass}
    >
      Details
    </a>
```

- [ ] **Step 2: `ProjectList` — drop the pass-through**

In `src/lib/components/ProjectList.svelte`: remove `onShowDetails` from `$props()`, its type, and the `<ProjectCard … onShowDetails={onShowDetails} />` (or `{onShowDetails}`) attribute.

- [ ] **Step 3: `+page.svelte` — remove detail-modal state**

In `src/routes/+page.svelte`: delete `detailTargetId`, `detailTarget`, `handleShowDetails`, `handleCloseDetails`, the `ProjectDetailModal` import, the `onShowDetails={handleShowDetails}` prop on `<ProjectList>`, and the `{#if detailTarget}<ProjectDetailModal … />{/if}` block.

- [ ] **Step 4: Check `FavoritesModal` / `BinModal`**

Run: `npx grep -rn "onShowDetails\|ProjectDetailModal" src/` (or use the editor search). For any hit in `FavoritesModal.svelte` / `BinModal.svelte`, remove the `onShowDetails` prop from their `<ProjectCard>` usage and their own `$props()`. If those modals render their own card markup (not `ProjectCard`), add a Details `<a href={`/project/${p.id}`}>` there too, matching Step 1.

- [ ] **Step 5: Delete the modal**

```bash
git rm src/lib/components/ProjectDetailModal.svelte
```

- [ ] **Step 6: Reveal-path permission**

Run: `npm run tauri dev`, open a project view, click a path's `📂 reveal`. If the console shows a permission error like `opener.reveal_item_in_dir not allowed`, add `"opener:allow-reveal-item-in-dir"` to the `permissions` array in `src-tauri/capabilities/default.json` and restart. If it works, no change.

- [ ] **Step 7: Verify**

Run: `npm run check` → 0 errors
Run: `npm run build` → succeeds
Manual: from the list, click **Details** on a project → lands on `/project/<id>`. Click the browser/app back → returns to the list. Repeat from the Favorites modal.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(ui): Details navigates to /project/[id]; remove ProjectDetailModal

The dedicated route replaces the modal. ProjectCard's Details button is now a
link; +page.svelte drops the detail-modal state.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Docs + full verification

**Files:**
- Modify: `docs/architecture.md`, `docs/knowledgebase.md`, `docs/checklist.md`, `docs/accomplishments.md`

- [ ] **Step 1: Full verification pass**

```bash
cd src-tauri && cargo test --lib && cargo fmt --check && cargo clippy --lib 2>&1 | grep "^warning:"
cd .. && npm test && npm run check && npm run build
```

Expected: all green; clippy shows only the 2 pre-existing warnings.

- [ ] **Step 2: Manual verification (via the `run` skill or `npm run tauri dev`)**

Walk the spec's checklist against a real project that is a git repo (add a throwaway `.uproject` file to it to also exercise the Unreal tab):
- open `/project/<id>` → identity block + status strip (`● git`, `● unreal`)
- Git tab: `web_url` row has `↗ open` (opens browser) + `⧉ copy`; `repo_url` (ssh) row is `⧉ copy` only; `Repo root` has `📂 reveal`; `Commit hash` copy; `Dirty` flag shows only when the tree is dirty; `Branches` chips
- Unreal tab: `.uproject` path reveal/copy; `Modules` / `Enabled plugins` chips
- per-tab **re-detect** → tab repaints
- **Refresh** → persists; force a failure (temporarily rename `.git` to break the git detector) → status strip shows `▲ git — …`, `ErrorBanner` on the page, other tabs still there
- **Edit** → change the name → save → header + identity update
- delete the project's directory on disk → reload the route → identity block + "directory unavailable" banner, no tabs
- visit `/project/not-a-real-id` → "project not found" error, back link works
- from the list and from the Favorites modal, **Details** navigates correctly; back returns

Fix anything that fails before continuing.

- [ ] **Step 3: Update `docs/architecture.md`**

- Invariant #1: reword so it reads (keep the surrounding list intact):

  > 1. **A new detector is "implement + register" — nothing else.** It touches
  >    its own module, one line in `detectors/registry.rs`, a `Tracker`
  >    variant, its `*Info` model, and `Detector::kind()`. **Zero frontend
  >    code:** the generic `TrackerPanel` renders any tracker, inferring each
  >    field's affordance from its name/shape — `*_url` / `https://…` → link,
  >    `*_root` / `*_path` / `*_dir` → path, `*hash*` / `*commit*` → code,
  >    arrays → chips, booleans → flags. A field named off-convention just
  >    renders as plain text.

- In the "Quality backlog", check off `[x]` **Explicit tracker/detector
  identity** and (add if missing, checked) **Per-detector status to the UI**.
- In the "Recorded decisions" → "Refresh is all-or-nothing" section, change the
  `Detection { trackers, errors }` phrasing to `Detection`'s `outcomes` and
  keep the guard-test name.

- [ ] **Step 4: Update `docs/knowledgebase.md`**

- Backend `detectors/` bullet: `Detection` now carries `outcomes` (one
  `DetectorOutcome` per detector); `Detector::kind()`; note the new
  `commands/inspect.rs` / `inspect_project`.
- Backend `models/` bullet: `GitInfo` gains `web_url`.
- Frontend section: `ProjectDetailModal` replaced by the `/project/[id]` route;
  `TrackerPanel` + the `trackers.ts` field-type convention; `vitest` is now a
  second frontend check alongside `svelte-check`.
- "Tracker detection, end to end": add that the project view calls
  `inspect_project` (read-only, live) on open and that `refresh` is still the
  only write path.
- Test-coverage line: bump the count; mention `web_url` normalization,
  `Detection` outcomes / `inspect`, `results_from` mapping, and the
  `trackers.ts` vitest suite.

- [ ] **Step 5: Update `docs/checklist.md`**

Add a section:

```markdown
## Project view

- [x] `/project/[id]` route replaces `ProjectDetailModal`
- [x] Live read-only detection on open (`inspect_project`) + per-detector status strip
- [x] Generic `TrackerPanel` — typed fields, open/reveal/copy affordances, no per-kind UI code
- [x] `GitInfo.web_url` (SSH→HTTPS) — "open remote" for any project in git
- [x] Per-tab re-detect, jump-to-Edit, Refresh
- [x] `vitest` covering the `trackers.ts` inference rules
```

Under "Detection plumbing", check off any identity/status items now done.

- [ ] **Step 6: Update `docs/accomplishments.md`**

Append a dated entry (`## 2026-08-31 — Project view`) summarizing: the
`Detection` outcomes refactor, `Detector::kind()`, `GitInfo.web_url`,
`inspect_project`, the `/project/[id]` route + generic `TrackerPanel` +
`trackers.ts` typing + vitest, and `ProjectDetailModal` removed. Note final
test counts and that `cargo`/`npm` checks are green.

- [ ] **Step 7: Commit**

```bash
git add docs/
git commit -m "$(cat <<'EOF'
docs: record the /project/[id] view and its backend changes

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Self-Review

**1. Spec coverage**

| Spec item | Task |
|---|---|
| `Detector::kind()` | 1 |
| `Detection` → outcomes, accessors, `into_result` unchanged | 2 |
| `DetectorRunner::inspect(path, only)` | 2 |
| 3 command call sites migrated | 2 |
| `GitInfo.web_url` + normalization table | 3 |
| `inspect_project` command + DTOs, directory-health handling | 4 |
| `ProjectInspection` / `DetectorResult` / `DirectoryStatus` / status strings | 4, 5 |
| `types.ts` mirrors, `inspectProject`, opener helpers | 5 |
| `trackers.ts` typed inference (+ rule-3 refinement noted) | 6 |
| `TrackerPanel.svelte` render-by-type table | 6 |
| `ProjectIdentity.svelte` lifted from the modal | 7 |
| `/project/[id]` route + `+page.ts` (`prerender = false`) | 7 |
| header / identity / status strip / tabs / edit overlay / refresh / re-detect | 7 |
| Not-found + directory-unavailable states | 7 (rendered), 4 (backend) |
| Details → route link; `ProjectDetailModal` deleted; `+page.svelte` cleanup; Favorites/Bin check | 8 |
| Tauri capability for reveal | 8 |
| doc + invariant updates | 9 |
| manual verification checklist | 7 (smoke), 9 (full) |
| clipboard failure swallowed; open/reveal failure → banner | 6 (TrackerPanel) |

No gaps.

**2. Placeholder scan**

Every code step has literal code. The two "confirm the real prop/export names"
notes (Task 5 Step 4, Task 7 Step 3) are verification instructions against
existing files, not deferred work. Task 8 Step 4/6 and Task 7 Step 5 are
conditional-on-observed-behavior, with the exact change spelled out for each
branch.

**3. Type consistency**

- `Detection { outcomes: Vec<DetectorOutcome> }` — same in Tasks 2, 3, 4.
- `DetectorOutcome::{Detected,NotDetected,Failed}` with `kind: &'static str` —
  Tasks 1 (Boom), 2, 4.
- `Detection::trackers()` / `errors()` (methods, not fields) — Tasks 2, 3.
- `inspect(&self, path: &Path, only: Option<&str>)` — Tasks 2, 4 (`only.as_deref()`).
- `web_url(remote: &str) -> Option<String>` (free fn) — Task 3 helper + tests.
- `GitInfo.web_url: Option<String>` / `web_url: string | null` — Tasks 2 (sample), 3, 6 (test fixture uses a string).
- `DetectorStatus` Rust `snake_case` ↔ TS `"detected" | "not_detected" | "failed"` — Tasks 4, 5.
- `TrackerField { label, type, text, items }` — Task 6 (`trackers.ts` + test + `TrackerPanel`).
- `inspectProject(id, opts?: { only?: string })` — Tasks 5, 7.
- `trackerKind` unchanged, still `(tracker) => string` — Tasks 6, 7.

Consistent.
