# Folder Scanning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Point the app at a folder, tick the detectors you care about, choose quick or deep, and import every project found in one reviewed pass.

**Architecture:** A pure walker in `core::domain::scan` produces candidates; `core::application::scan_service` filters out already-tracked directories, assigns collision-free names, and commits them through `ProjectService`. Three Tauri commands wrap it. The frontend is one modal with three steps, remembering its settings in `localStorage`. No new database table, no schema migration.

**Tech Stack:** Rust (`indexer-core`, no new crate dependencies — the walk is `std::fs`), Tauri 2 command layer, Svelte 5 + Tailwind 4 frontend, `cargo test` and Vitest.

**Spec:** [`docs/superpowers/specs/2026-09-07-folder-scanning-design.md`](../specs/2026-09-07-folder-scanning-design.md)

## Global Constraints

- **`indexer-core` must never `use tauri`.** Invariant 9. All logic lives in core; the Tauri layer is a thin adapter.
- **No new third-party crate.** The walk uses `std::fs` only. Do not add `walkdir`, `ignore`, or `rayon`.
- **`user_version` stays at 3.** This feature ships no migration and no new table. If you find yourself editing `run_migrations`, stop — you have gone outside the plan.
- **Never call `refresh_trackers` or `Detection::into_result()` in the bulk path.** `into_result()` is all-or-nothing: one corrupt repository would discard an entire 200-directory sweep. Use `trackers()` plus logged errors.
- **`.git`, `.svn`, `.hg` are pruned unconditionally**, outside the include-ignored option. Submodules live under `.git/modules`; a walk reaching them would offer to import every submodule as its own project.
- **`MAX_DIRECTORIES = 50_000`.** The walk stops there and reports `stopped_early`.
- **Zero per-detector frontend code.** Invariant 1. The detector tick-list is built from a backend call, never a hardcoded array.
- **Comment style:** this codebase writes *why*, not *what*. Match the density of the file you are editing.
- Run Rust tests with `cargo test -p indexer-core`, frontend tests with `pnpm test`.

---

### Task 1: `DetectorRunner::inspect_kinds`

The set filter that lets the walk run only the ticked detectors. The existing single-kind `inspect` stays — the re-detect sweep needs it — and is reimplemented over the new method so there is one traversal of the detector list.

**Files:**
- Modify: `crates/core/src/detectors/runner.rs` (the `impl DetectorRunner` block, and its `mod tests`)

**Interfaces:**
- Consumes: nothing.
- Produces: `DetectorRunner::inspect_kinds(&self, path: &Path, only: Option<&[&str]>) -> Detection`. `None` runs every detector; `Some(&[])` runs none; unknown kinds match nothing rather than erroring.

- [ ] **Step 1: Write the failing tests**

Add to the `mod tests` block at the bottom of `crates/core/src/detectors/runner.rs`:

```rust
#[test]
fn inspect_kinds_runs_only_the_named_detectors() {
    let dir = temp_dir("kinds");
    git2::Repository::init(&dir).expect("should init a git repo");

    let runner = DetectorRunner::new(vec![Box::new(Gitector), Box::new(Boom)]);

    assert_eq!(runner.inspect_kinds(&dir, Some(&["git"])).outcomes.len(), 1);
    assert_eq!(
        runner.inspect_kinds(&dir, Some(&["git", "boom"])).outcomes.len(),
        2
    );
    assert_eq!(runner.inspect_kinds(&dir, None).outcomes.len(), 2);

    std::fs::remove_dir_all(&dir).ok();
}

/// An empty selection is "no detectors", not "all detectors" — the walk
/// relies on this to treat a scan with nothing ticked as finding nothing,
/// rather than importing the entire disk.
#[test]
fn inspect_kinds_with_an_empty_selection_runs_nothing() {
    let dir = temp_dir("kinds-empty");
    git2::Repository::init(&dir).expect("should init a git repo");

    let runner = DetectorRunner::new(vec![Box::new(Gitector)]);
    assert_eq!(runner.inspect_kinds(&dir, Some(&[])).outcomes.len(), 0);

    std::fs::remove_dir_all(&dir).ok();
}

/// Remembered settings can name a detector that no longer exists. That must
/// degrade to finding less, never to an error.
#[test]
fn inspect_kinds_ignores_unknown_kinds() {
    let dir = temp_dir("kinds-unknown");
    git2::Repository::init(&dir).expect("should init a git repo");

    let runner = DetectorRunner::new(vec![Box::new(Gitector)]);

    assert_eq!(runner.inspect_kinds(&dir, Some(&["nonsense"])).outcomes.len(), 0);
    assert_eq!(
        runner.inspect_kinds(&dir, Some(&["nonsense", "git"])).outcomes.len(),
        1
    );

    std::fs::remove_dir_all(&dir).ok();
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p indexer-core inspect_kinds`
Expected: FAIL to compile — `no method named 'inspect_kinds' found for struct 'DetectorRunner'`.

- [ ] **Step 3: Write the implementation**

In `crates/core/src/detectors/runner.rs`, replace the body of `inspect` and add `inspect_kinds` beside it:

```rust
    /// Like [`detect_project`](Self::detect_project), but when `only` is
    /// `Some(kind)` only the detector whose [`Detector::kind`] equals `kind`
    /// runs (for per-tracker re-detect). An unknown `kind` matches nothing
    /// and yields an empty [`Detection`].
    ///
    /// Kept alongside [`inspect_kinds`](Self::inspect_kinds) because the
    /// re-detect sweep genuinely wants exactly one detector, and expressing
    /// that as a one-element slice at every call site reads worse.
    pub fn inspect(&self, path: &Path, only: Option<&str>) -> Detection {
        match only {
            Some(kind) => self.inspect_kinds(path, Some(&[kind])),
            None => self.inspect_kinds(path, None),
        }
    }

    /// Runs the detectors whose [`Detector::kind`] appears in `only`, or every
    /// registered detector when `only` is `None`.
    ///
    /// This is the folder scanner's entry point. The distinction it encodes:
    /// the selection decides **whether a directory is worth registering**, not
    /// what gets recorded about it — a directory that survives the scan is
    /// registered through `create`, which runs every installed detector and
    /// stores every tracker that matched.
    ///
    /// `Some(&[])` runs nothing, which is deliberate: a scan with no detectors
    /// ticked finds nothing rather than everything. An unrecognized kind
    /// matches nothing rather than erroring, so remembered settings naming a
    /// detector that has since been removed degrade to finding less.
    pub fn inspect_kinds(&self, path: &Path, only: Option<&[&str]>) -> Detection {
        let mut outcomes = Vec::new();
        for detector in &self.detectors {
            let kind = detector.kind();
            if only.is_some_and(|kinds| !kinds.contains(&kind)) {
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
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p indexer-core --lib detectors::runner`
Expected: PASS — including the pre-existing `inspect_runs_only_the_named_detector`, which proves the delegation kept the old behaviour.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/detectors/runner.rs
git commit -m "feat(core): add DetectorRunner::inspect_kinds set filter"
```

---

### Task 2: `disambiguate`

The one function that answers the name-collision question for both the scanner and `ensure_project`.

**Files:**
- Modify: `crates/core/src/domain/naming.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `domain::naming::disambiguate(preferred: &str, parent_dir: &str, taken: &HashSet<String>) -> String`. `taken` holds **lowercased** names. Callers build it with `taken_names_from`.
- Produces: `domain::naming::taken_names_from(names: impl IntoIterator<Item = String>) -> HashSet<String>` — lowercases and collects, so no caller has to remember the case rule.

- [ ] **Step 1: Write the failing tests**

Add to the `mod tests` block in `crates/core/src/domain/naming.rs`:

```rust
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p indexer-core disambiguate`
Expected: FAIL to compile — `cannot find function 'disambiguate' in this scope`.

- [ ] **Step 3: Write the implementation**

At the top of `crates/core/src/domain/naming.rs` add the import, then append the functions:

```rust
use std::collections::HashSet;
```

```rust
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
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p indexer-core --lib domain::naming`
Expected: PASS, all tests in the module including the pre-existing ones.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/domain/naming.rs
git commit -m "feat(core): add disambiguate for colliding project names"
```

---

### Task 3: `ensure_project` honours names and collisions

Fixes a real bug — `ensure_project` currently hands an underived name to `create` and fails with `DuplicateName` the second time it meets an `api` folder — and gives the scanner the get-or-create entry point it commits through.

**Files:**
- Modify: `crates/core/src/application/service.rs` (the `ensure_project` method near line 302, and `mod tests`)

**Interfaces:**
- Consumes: `disambiguate`, `taken_names_from` (Task 2).
- Produces:
  - `ProjectService::ensure_project(&self, directory: &str) -> Result<Project, ProjectError>` — unchanged signature, now collision-safe.
  - `ProjectService::ensure_project_named(&self, directory: &str, name: &str) -> Result<Project, ProjectError>` — get-or-create with a caller-supplied name, disambiguated if it collides. The scanner's commit path.
  - `ProjectService::active_project_names(&self) -> Result<Vec<String>, ProjectError>` — names of non-deleted projects, for building `taken`.

- [ ] **Step 1: Write the failing tests**

Add to the `mod tests` block in `crates/core/src/application/service.rs`. The existing `tmpdir` helper is already in that module.

```rust
    /// The bug this fixes: two `api` directories under different parents both
    /// have to register. Before `disambiguate`, the second returned
    /// `DuplicateName` and the observer CLI simply failed on it.
    /// Note the expectation is derived, not hardcoded: `tmpdir` prefixes its
    /// argument (`pi-svc-collide-work`), and that prefix is the parent folder
    /// name `disambiguate` qualifies with. Spelling the prefix into the
    /// assertion would couple this test to the helper's naming.
    #[test]
    fn ensure_project_disambiguates_a_colliding_name() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let code = tmpdir("collide-code");
        let work = tmpdir("collide-work");
        let a = std::path::Path::new(&code).join("api");
        let b = std::path::Path::new(&work).join("api");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();

        let first = svc.ensure_project(a.to_str().unwrap()).unwrap();
        let second = svc.ensure_project(b.to_str().unwrap()).unwrap();

        let work_folder = std::path::Path::new(&work)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert_eq!(first.name, "api");
        assert_eq!(second.name, format!("{work_folder}/api"));
        assert_ne!(first.id, second.id);
    }

    #[test]
    fn ensure_project_named_uses_the_given_name() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let dir = tmpdir("named");

        let project = svc.ensure_project_named(&dir, "Chosen Name").unwrap();

        assert_eq!(project.name, "Chosen Name");
    }

    /// Get-or-create: a directory already tracked comes back as-is, and the
    /// supplied name does not rename it. Re-scanning a folder must be a no-op,
    /// not an edit.
    #[test]
    fn ensure_project_named_is_idempotent_and_does_not_rename() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let dir = tmpdir("named-idempotent");

        let first = svc.ensure_project_named(&dir, "First").unwrap();
        let second = svc.ensure_project_named(&dir, "Second").unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(second.name, "First");
        assert_eq!(svc.list(Default::default()).unwrap().len(), 1);
    }

    #[test]
    fn ensure_project_named_disambiguates_a_colliding_name() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let first_dir = tmpdir("named-collide-a");
        let parent = tmpdir("clients");
        let second_dir = std::path::Path::new(&parent).join("api");
        std::fs::create_dir_all(&second_dir).unwrap();

        svc.ensure_project_named(&first_dir, "api").unwrap();
        let second = svc
            .ensure_project_named(second_dir.to_str().unwrap(), "api")
            .unwrap();

        // Derived, not hardcoded — `tmpdir` prefixes its argument, and that
        // prefixed folder name is what the qualifier uses.
        let parent_folder = std::path::Path::new(&parent)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert_eq!(second.name, format!("{parent_folder}/api"));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p indexer-core ensure_project`
Expected: FAIL to compile — `no method named 'ensure_project_named'`.

- [ ] **Step 3: Write the implementation**

In `crates/core/src/application/service.rs`, add the import at the top:

```rust
use crate::domain::naming::{disambiguate, suggest_project_name, taken_names_from};
```

(That replaces the existing `use crate::domain::naming::suggest_project_name;` line.)

Then replace the whole `ensure_project` method with these three:

```rust
    /// Names of every non-deleted project. The input to
    /// [`taken_names_from`](crate::domain::naming::taken_names_from) wherever a
    /// caller needs to pick a name that will survive `create`'s duplicate check.
    pub fn active_project_names(&self) -> Result<Vec<String>, ProjectError> {
        Ok(self
            .repo
            .list()?
            .into_iter()
            .filter(|p| !p.is_deleted)
            .map(|p| p.name)
            .collect())
    }

    /// Returns the project registered for `directory`, creating one if there
    /// isn't one yet. The name is inferred exactly the way the GUI's
    /// `suggest_project_name` command infers it — the git remote's repo name
    /// when the directory is a repo with a remote, otherwise the folder name —
    /// and then disambiguated against the names already in use.
    pub fn ensure_project(&self, directory: &str) -> Result<Project, ProjectError> {
        let trackers = self.preview_detection(directory);
        let name =
            suggest_project_name(&trackers, directory).unwrap_or_else(|| "project".to_string());
        self.ensure_project_named(directory, &name)
    }

    /// Get-or-create for `directory`, using `name` when it has to create.
    ///
    /// The scanner's commit path: the user reviewed and possibly edited that
    /// name, so it is used rather than re-derived. If it collides with a name
    /// already in use it is disambiguated — `create` would otherwise reject it
    /// with `DuplicateName`, which across a two-hundred-directory import means
    /// losing a row for a reason the user cannot act on.
    ///
    /// **Idempotent, and never a rename.** A directory already tracked comes
    /// back untouched, whatever `name` says: re-scanning a folder is a no-op,
    /// not an edit to projects the user has since renamed by hand.
    pub fn ensure_project_named(
        &self,
        directory: &str,
        name: &str,
    ) -> Result<Project, ProjectError> {
        if let Some(existing) = self.find_by_directory(directory)? {
            return Ok(existing);
        }
        let taken = taken_names_from(self.active_project_names()?);
        let parent = Path::new(directory)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");
        let name = disambiguate(name, parent, &taken);
        self.create(name, directory.to_string(), None, None)
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p indexer-core --lib application::service`
Expected: PASS, including the pre-existing `ensure_project_is_idempotent`.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/application/service.rs
git commit -m "fix(core): disambiguate names in ensure_project, add ensure_project_named"
```

---

### Task 4: The walk

The pure traversal. No database, no store, no `ProjectService` — given a request and a detector runner, it produces candidates.

**Files:**
- Create: `crates/core/src/domain/scan.rs`
- Modify: `crates/core/src/domain/mod.rs` (register the module and re-export)

**Interfaces:**
- Consumes: `DetectorRunner::inspect_kinds` (Task 1), `naming::suggest_project_name`.
- Produces:
  - `domain::scan::ScanMode` — `Quick` | `Deep { depth: u32 }`, serde-tagged as `{"mode":"quick"}` / `{"mode":"deep","depth":3}`.
  - `domain::scan::ScanRequest { scan_root: String, mode: ScanMode, detectors: Vec<String>, include_ignored: bool }`
  - `domain::scan::Candidate { directory: String, suggested_name: String, matched_kinds: Vec<String>, already_tracked: bool }`
  - `domain::scan::ScanReport { candidates: Vec<Candidate>, visited: usize, stopped_early: bool }`
  - `domain::scan::walk(request: &ScanRequest, detectors: &DetectorRunner) -> ScanReport`
  - `domain::scan::MAX_DIRECTORIES: usize`

- [ ] **Step 1: Write the failing tests**

Create `crates/core/src/domain/scan.rs` containing **only** this test module for now (the implementation lands in step 3):

```rust
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
        repo_at(&root, "one/two/target");

        let shallow = walk(&request(&root, ScanMode::Deep { depth: 2 }), &runner());
        assert!(shallow.candidates.is_empty());

        let deep = walk(&request(&root, ScanMode::Deep { depth: 3 }), &runner());
        assert_eq!(directories(&deep), vec!["target"]);

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
```

- [ ] **Step 2: Run the tests to verify they fail**

First register the module — add `pub mod scan;` to `crates/core/src/domain/mod.rs` in alphabetical position (after `pub mod project;`).

Run: `cargo test -p indexer-core --lib domain::scan`
Expected: FAIL to compile — `cannot find function 'walk' in this scope`, plus unresolved `ScanMode`, `ScanRequest`, `ScanReport`, `DetectorRunner`.

- [ ] **Step 3: Write the implementation**

Insert **above** the `#[cfg(test)] mod tests` block in `crates/core/src/domain/scan.rs`:

```rust
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
const CONVENTIONALLY_IGNORED: &[&str] = &["node_modules", "target", ".venv", "venv", "build", "dist"];

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
            eprintln!("Scan: skipping unreadable directory '{}': {e}", path.display());
            return Vec::new();
        }
    };

    let mut children: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|entry| {
            !is_pruned(&entry.file_name().to_string_lossy(), include_ignored)
        })
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
```

Then add the re-export to `crates/core/src/domain/mod.rs`, below the existing `pub use` lines:

```rust
pub use scan::{Candidate, ScanMode, ScanReport, ScanRequest};
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p indexer-core --lib domain::scan`
Expected: PASS — all ten tests.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/domain/scan.rs crates/core/src/domain/mod.rs
git commit -m "feat(core): add the folder-scan walk"
```

---

### Task 5: `ScanService`

Orchestration: run the walk, flag what is already tracked, assign collision-free names, and commit selected rows best-effort.

**Files:**
- Create: `crates/core/src/application/scan_service.rs`
- Modify: `crates/core/src/detectors/runner.rs` (add `kinds()`)
- Modify: `crates/core/src/application/mod.rs`, `crates/core/src/lib.rs` (re-exports)

**Interfaces:**
- Consumes: `domain::scan::{walk, ScanRequest, ScanReport}` (Task 4); `ProjectService::{find_by_directory, active_project_names, ensure_project_named}` (Task 3); `naming::{disambiguate, taken_names_from}` (Task 2).
- Produces:
  - `application::ScanService::new(projects: Arc<ProjectService>, detectors: Arc<DetectorRunner>) -> ScanService`
  - `ScanService::scan(&self, request: &ScanRequest) -> Result<ScanReport, ProjectError>`
  - `ScanService::import(&self, selections: &[ImportSelection]) -> Result<ImportReport, ProjectError>`
  - `application::scan_service::ImportSelection { directory: String, name: String }`
  - `application::scan_service::ImportReport { imported: Vec<Project>, skipped: usize, failures: Vec<ImportFailure> }`
  - `application::scan_service::ImportFailure { directory: String, message: String }`

- [ ] **Step 1: Write the failing tests**

Create `crates/core/src/application/scan_service.rs` with **only** this test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::detectors::DetectorRunner;
    use crate::domain::scan::ScanMode;
    use crate::error::LauncherError;
    use crate::infra::SqliteRepository;
    use crate::ports::AppLauncher;
    use std::path::{Path, PathBuf};

    struct NoLauncher;
    impl AppLauncher for NoLauncher {
        fn open(&self, _: &str, _: Option<&str>) -> Result<(), LauncherError> {
            Ok(())
        }
        fn is_available(&self, _: &str) -> bool {
            false
        }
    }

    fn services() -> (Arc<ProjectService>, ScanService) {
        let repo = Arc::new(SqliteRepository::in_memory().unwrap());
        let detectors = Arc::new(DetectorRunner::default());
        let projects = Arc::new(ProjectService::new(
            repo.clone(),
            Arc::new(NoLauncher),
            detectors.clone(),
            repo,
        ));
        let scan = ScanService::new(projects.clone(), detectors);
        (projects, scan)
    }

    fn temp_tree(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("project-indexer-tests-scansvc-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("should create temp dir");
        dir
    }

    fn repo_at(root: &Path, relative: &str) -> String {
        let path = root.join(relative);
        std::fs::create_dir_all(&path).expect("should create dir");
        git2::Repository::init(&path).expect("should init a git repo");
        path.to_string_lossy().to_string()
    }

    fn request(root: &Path) -> ScanRequest {
        ScanRequest {
            scan_root: root.to_string_lossy().to_string(),
            mode: ScanMode::Quick,
            detectors: vec!["git".to_string()],
            include_ignored: false,
        }
    }

    #[test]
    fn scan_assigns_collision_free_names_within_one_report() {
        let root = temp_tree("collide");
        repo_at(&root, "work/api");
        repo_at(&root, "code/api");
        let (_, scan) = services();

        let report = scan
            .scan(&ScanRequest {
                mode: ScanMode::Deep { depth: 2 },
                ..request(&root)
            })
            .unwrap();

        let mut names: Vec<String> = report
            .candidates
            .iter()
            .map(|c| c.suggested_name.clone())
            .collect();
        names.sort();
        assert_eq!(names, vec!["api".to_string(), "work/api".to_string()]);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn scan_flags_directories_that_are_already_tracked() {
        let root = temp_tree("tracked");
        let api = repo_at(&root, "api");
        let (projects, scan) = services();
        projects.ensure_project(&api).unwrap();

        let report = scan.scan(&request(&root)).unwrap();

        assert_eq!(report.candidates.len(), 1);
        assert!(report.candidates[0].already_tracked);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn import_registers_the_selected_directories() {
        let root = temp_tree("import");
        let api = repo_at(&root, "api");
        let site = repo_at(&root, "site");
        let (projects, scan) = services();

        let report = scan
            .import(&[
                ImportSelection { directory: api, name: "api".into() },
                ImportSelection { directory: site, name: "site".into() },
            ])
            .unwrap();

        assert_eq!(report.imported.len(), 2);
        assert!(report.failures.is_empty());
        assert_eq!(report.skipped, 0);
        assert_eq!(projects.list(Default::default()).unwrap().len(), 2);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn import_counts_an_already_tracked_directory_as_skipped() {
        let root = temp_tree("import-skip");
        let api = repo_at(&root, "api");
        let (projects, scan) = services();
        projects.ensure_project(&api).unwrap();

        let report = scan
            .import(&[ImportSelection { directory: api, name: "api".into() }])
            .unwrap();

        assert_eq!(report.skipped, 1);
        assert!(report.imported.is_empty());
        assert_eq!(projects.list(Default::default()).unwrap().len(), 1);

        std::fs::remove_dir_all(&root).ok();
    }

    /// The rule that makes a bulk import usable: a row that cannot be
    /// registered costs that row and nothing else. Across two hundred
    /// directories, one bad path must not discard the other 199.
    #[test]
    fn a_failing_row_does_not_stop_the_rest() {
        let root = temp_tree("import-partial");
        let api = repo_at(&root, "api");
        let (projects, scan) = services();

        let report = scan
            .import(&[
                ImportSelection { directory: String::new(), name: "broken".into() },
                ImportSelection { directory: api, name: "api".into() },
            ])
            .unwrap();

        assert_eq!(report.imported.len(), 1);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(projects.list(Default::default()).unwrap().len(), 1);

        std::fs::remove_dir_all(&root).ok();
    }

    /// A rescan after committing is the "what is new since last time" case.
    #[test]
    fn rescanning_after_an_import_reports_everything_as_tracked() {
        let root = temp_tree("rescan");
        let api = repo_at(&root, "api");
        let (_, scan) = services();

        scan.import(&[ImportSelection { directory: api, name: "api".into() }])
            .unwrap();
        let report = scan.scan(&request(&root)).unwrap();

        assert!(report.candidates.iter().all(|c| c.already_tracked));

        std::fs::remove_dir_all(&root).ok();
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Register the module first — add `pub mod scan_service;` to `crates/core/src/application/mod.rs`.

Run: `cargo test -p indexer-core --lib application::scan_service`
Expected: FAIL to compile — `cannot find type 'ScanService' in this scope`.

- [ ] **Step 3: Write the implementation**

Insert **above** the test module in `crates/core/src/application/scan_service.rs`:

```rust
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::application::ProjectService;
use crate::detectors::DetectorRunner;
use crate::domain::naming::{disambiguate, taken_names_from};
// `ScanMode` is deliberately not imported here — the service never inspects
// the mode, it just hands the request to the walk. The test module imports it
// itself.
use crate::domain::scan::{walk, ScanReport, ScanRequest};
use crate::domain::Project;
use crate::error::ProjectError;
use std::path::Path;

/// One reviewed row on its way into the store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSelection {
    pub directory: String,
    /// The name as the user left it in the review list — used rather than
    /// re-derived, because they may well have edited it.
    pub name: String,
}

/// Why one row could not be imported. Carried rather than returned as an
/// error, so the other rows still land.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportFailure {
    pub directory: String,
    pub message: String,
}

/// What one bulk import did.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReport {
    pub imported: Vec<Project>,
    /// Rows whose directory was already tracked. Not a failure — re-importing
    /// a folder is a no-op by design.
    pub skipped: usize,
    pub failures: Vec<ImportFailure>,
}

/// Bulk import: walk a folder, review what it found, register the selection.
///
/// Separate from [`ProjectService`] rather than more methods on it — that file
/// is already the largest in `core`, and bulk-import orchestration is a
/// different concern from the per-project surface. It also keeps the walk
/// testable without a database.
///
/// **User-triggered, bounded and finite.** Nothing here runs unprompted; there
/// is no timer, no watcher and no background task. "Autorunner" names *this*,
/// and the word "background" is reserved for a feature that does not exist.
pub struct ScanService {
    projects: Arc<ProjectService>,
    detectors: Arc<DetectorRunner>,
}

impl std::fmt::Debug for ScanService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScanService").finish_non_exhaustive()
    }
}

impl ScanService {
    pub fn new(projects: Arc<ProjectService>, detectors: Arc<DetectorRunner>) -> Self {
        Self {
            projects,
            detectors,
        }
    }

    /// Walks `request` and returns what it found, with names already resolved
    /// against the projects that exist and against each other, and
    /// already-tracked directories flagged.
    ///
    /// Names are assigned here rather than at import so the review list shows
    /// exactly what will be created — a rename appearing only after the user
    /// pressed Import would be a surprise.
    pub fn scan(&self, request: &ScanRequest) -> Result<ScanReport, ProjectError> {
        let mut report = walk(request, &self.detectors);

        let mut taken = taken_names_from(self.projects.active_project_names()?);
        for candidate in &mut report.candidates {
            candidate.already_tracked = self
                .projects
                .find_by_directory(&candidate.directory)?
                .is_some();

            let parent = Path::new(&candidate.directory)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("");
            let name = disambiguate(&candidate.suggested_name, parent, &taken);
            // Reserve it, so two candidates in one report cannot collide with
            // each other — the case `~/code/api` and `~/work/api` hits on the
            // very first scan.
            taken.insert(name.to_lowercase());
            candidate.suggested_name = name;
        }

        Ok(report)
    }

    /// Registers `selections`, best-effort.
    ///
    /// **Per-row failures are collected, never fatal.** A corrupt repository at
    /// row 47 must not cost rows 48 through 200 — which is also why this goes
    /// through `ensure_project_named` (get-or-create, best-effort detection)
    /// and never `refresh_trackers`, whose all-or-nothing `into_result()`
    /// would discard an entire sweep over one bad directory.
    pub fn import(&self, selections: &[ImportSelection]) -> Result<ImportReport, ProjectError> {
        let mut imported = Vec::new();
        let mut skipped = 0usize;
        let mut failures = Vec::new();

        for selection in selections {
            let already_tracked = match self.projects.find_by_directory(&selection.directory) {
                Ok(existing) => existing.is_some(),
                Err(e) => {
                    failures.push(ImportFailure {
                        directory: selection.directory.clone(),
                        message: e.to_string(),
                    });
                    continue;
                }
            };
            if already_tracked {
                skipped += 1;
                continue;
            }

            match self
                .projects
                .ensure_project_named(&selection.directory, &selection.name)
            {
                Ok(project) => imported.push(project),
                Err(e) => failures.push(ImportFailure {
                    directory: selection.directory.clone(),
                    message: e.to_string(),
                }),
            }
        }

        Ok(ImportReport {
            imported,
            skipped,
            failures,
        })
    }

    /// The detector kinds this build registers, for the scan form's tick-list.
    ///
    /// Exists so shipping a new detector needs no frontend edit — invariant 1,
    /// "implement + register, zero frontend code". A hardcoded array of names
    /// in a Svelte file would break it the day Unity lands.
    pub fn detector_kinds(&self) -> Vec<String> {
        self.detectors.kinds()
    }
}
```

Add the supporting method to `crates/core/src/detectors/runner.rs`, inside `impl DetectorRunner`:

```rust
    /// The [`Detector::kind`] of every registered detector, in registration
    /// order. Lets a caller present the detector set without knowing which
    /// concrete detectors were compiled in.
    pub fn kinds(&self) -> Vec<String> {
        self.detectors
            .iter()
            .map(|d| d.kind().to_string())
            .collect()
    }
```

Then the re-exports. In `crates/core/src/application/mod.rs`:

```rust
pub use scan_service::{ImportFailure, ImportReport, ImportSelection, ScanService};
```

And in `crates/core/src/lib.rs`, extend the application re-export line:

```rust
pub use application::{GroupService, ProjectInspection, ProjectService, ScanService};
```

Note the `ScanMode` import in this file is used by the tests only; if the compiler warns it is unused in the non-test build, move it into the `mod tests` `use super::*` by importing it there instead.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p indexer-core`
Expected: PASS — the whole core suite, so this also confirms Tasks 1–4 still hold.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/application/scan_service.rs crates/core/src/application/mod.rs crates/core/src/detectors/runner.rs crates/core/src/lib.rs
git commit -m "feat(core): add ScanService for reviewed bulk import"
```

---

### Task 6: Tauri commands

Three commands and the managed state behind them. The adapter stays thin — no logic here.

**Files:**
- Create: `src-tauri/src/commands/scan.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` (managed state in `setup`, handler registration)

**Interfaces:**
- Consumes: `ScanService::{scan, import, detector_kinds}`, `ScanRequest`, `ScanReport`, `ImportSelection`, `ImportReport` (Task 5).
- Produces: the Tauri commands `scan_folder`, `import_scanned`, `list_detector_kinds`.

- [ ] **Step 1: Write the command module**

There is no Rust test harness for the Tauri layer in this codebase — it is a thin adapter and the logic it calls is covered by Task 5. Verification for this task is `cargo build` plus the manual check in step 4.

Create `src-tauri/src/commands/scan.rs`:

```rust
use std::sync::Arc;

use tauri::State;

use indexer_core::application::{ImportReport, ImportSelection, ScanService};
use indexer_core::domain::scan::{ScanReport, ScanRequest};
use indexer_core::error::ProjectError;

/// Walks a folder looking for projects the ticked detectors recognize, and
/// returns what it found **without registering anything**. The review step is
/// the point: registering a project is a durable act, and a bulk one should be
/// a deliberate one.
///
/// `async` so the walk runs on Tauri's pool and the window stays responsive.
/// It is blocking from the frontend's point of view — there are no progress
/// events and no cancellation, which is acceptable only because the walk is
/// bounded by `MAX_DIRECTORIES`; `stopped_early` on the report says when that
/// bound was hit.
#[tauri::command]
pub async fn scan_folder(
    scan: State<'_, Arc<ScanService>>,
    request: ScanRequest,
) -> Result<ScanReport, ProjectError> {
    scan.scan(&request)
}

/// Registers the rows the user kept, using the names they left in the review
/// list. Best-effort per row: a directory that fails is reported in
/// `failures` and the rest still land.
#[tauri::command]
pub async fn import_scanned(
    scan: State<'_, Arc<ScanService>>,
    selections: Vec<ImportSelection>,
) -> Result<ImportReport, ProjectError> {
    scan.import(&selections)
}

/// The detector kinds this build registers, so the scan form's tick-list is
/// built from what the binary actually has rather than a hardcoded array —
/// invariant 1, "a new detector is implement + register, zero frontend code".
#[tauri::command]
pub fn list_detector_kinds(scan: State<'_, Arc<ScanService>>) -> Vec<String> {
    scan.detector_kinds()
}
```

- [ ] **Step 2: Register the module, the state and the handlers**

In `src-tauri/src/commands/mod.rs`, add in alphabetical position:

```rust
pub mod scan;
```

In `src-tauri/src/lib.rs`, extend the `use` of core's application types to include `ScanService` (match the existing import style in that file), then inside `.setup(...)` — immediately after `app.manage(Arc::new(GroupService::new(repo)));` — replace that line and add the scan service. Because `service` is moved into `manage`, capture an `Arc` of it first:

```rust
            let service = Arc::new(service);
            let detectors = Arc::new(DetectorRunner::default());
            app.manage(Arc::new(ScanService::new(
                service.clone(),
                detectors,
            )));
            app.manage(service);
            app.manage(Arc::new(GroupService::new(repo)));
```

For this to compile, the `ProjectService::new(...)` call just above must use that same detector runner rather than constructing a second one. Change it to build the runner first:

```rust
            let repo = Arc::new(repo);
            let detectors = Arc::new(DetectorRunner::default());
            let service = Arc::new(ProjectService::new(
                repo.clone(),
                Arc::new(OpenerLauncher),
                detectors.clone(),
                repo.clone(),
            ));
            app.manage(Arc::new(ScanService::new(service.clone(), detectors)));
            app.manage(service);
            app.manage(Arc::new(GroupService::new(repo)));
```

(One runner, shared — a second `DetectorRunner::default()` would work but means two registrations of the same stateless detectors for no reason.)

Then add the three commands to `tauri::generate_handler![...]`, after the `inspect_project` line:

```rust
            commands::scan::scan_folder,
            commands::scan::import_scanned,
            commands::scan::list_detector_kinds,
```

- [ ] **Step 3: Build to verify it compiles**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: SUCCESS, no warnings about unused imports.

- [ ] **Step 4: Verify the whole workspace still builds and tests**

Run: `cargo test -p indexer-core && cargo build --manifest-path src-tauri/Cargo.toml`
Expected: PASS then SUCCESS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/scan.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(tauri): expose scan_folder, import_scanned and list_detector_kinds"
```

---

### Task 7: Frontend API client and types

Mirrors the Rust models and wraps the three commands, following `src/lib/api/projects.ts` exactly.

**Files:**
- Modify: `src/lib/api/types.ts`
- Create: `src/lib/api/scan.ts`

**Interfaces:**
- Consumes: the commands from Task 6.
- Produces: `ScanMode`, `ScanRequest`, `Candidate`, `ScanReport`, `ImportSelection`, `ImportFailure`, `ImportReport` types; `scanFolder`, `importScanned`, `listDetectorKinds` functions.

- [ ] **Step 1: Add the types**

Append to `src/lib/api/types.ts`:

```ts
// Mirrors crates/core/src/domain/scan.rs and
// crates/core/src/application/scan_service.rs.
//
// ScanMode is #[serde(tag = "mode", rename_all = "lowercase")] and is
// #[serde(flatten)]ed into ScanRequest, so the wire shape is a flat object:
// { scan_root, mode: "quick", detectors, include_ignored } or
// { scan_root, mode: "deep", depth: 3, detectors, include_ignored }.
export type ScanRequest =
  | {
      scan_root: string;
      mode: "quick";
      detectors: string[];
      include_ignored: boolean;
    }
  | {
      scan_root: string;
      mode: "deep";
      depth: number;
      detectors: string[];
      include_ignored: boolean;
    };

export interface Candidate {
  directory: string;
  suggested_name: string;
  matched_kinds: string[];
  already_tracked: boolean;
}

export interface ScanReport {
  candidates: Candidate[];
  visited: number;
  // True when MAX_DIRECTORIES (50,000) was reached, so the results are
  // incomplete rather than the disk holding no more.
  stopped_early: boolean;
}

export interface ImportSelection {
  directory: string;
  name: string;
}

export interface ImportFailure {
  directory: string;
  message: string;
}

export interface ImportReport {
  imported: Project[];
  skipped: number;
  failures: ImportFailure[];
}
```

- [ ] **Step 2: Write the client**

Create `src/lib/api/scan.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
import { toError } from "./errors";
import type { ImportReport, ImportSelection, ScanReport, ScanRequest } from "./types";

// Walks a folder and reports what it found. Registers nothing — the review
// step between this and importScanned is the whole point of the feature.
export async function scanFolder(request: ScanRequest): Promise<ScanReport> {
  try {
    return await invoke<ScanReport>("scan_folder", { request });
  } catch (err) {
    throw toError(err);
  }
}

// Registers the reviewed rows. Best-effort per row: check `failures` on the
// result rather than expecting this to throw for one bad directory.
export async function importScanned(selections: ImportSelection[]): Promise<ImportReport> {
  try {
    return await invoke<ImportReport>("import_scanned", { selections });
  } catch (err) {
    throw toError(err);
  }
}

// The detector kinds this build registers. The scan form's tick-list is built
// from this, never a hardcoded array — a new detector must cost zero frontend
// code.
export async function listDetectorKinds(): Promise<string[]> {
  try {
    return await invoke<string[]>("list_detector_kinds");
  } catch (err) {
    throw toError(err);
  }
}
```

- [ ] **Step 3: Verify types check**

Run: `pnpm check`
Expected: no errors introduced by these files.

- [ ] **Step 4: Commit**

```bash
git add src/lib/api/scan.ts src/lib/api/types.ts
git commit -m "feat(ui): add the scan API client and types"
```

---

### Task 8: Remembered scan settings

The whole of "remembering": last-used settings in `localStorage`, pre-filling the form. Follows `src/lib/viewState.ts` — pure restore rules split from storage access so the rules are testable in Vitest's node environment, where there is no `localStorage`.

**Files:**
- Create: `src/lib/scanSettings.ts`
- Create: `src/lib/scanSettings.test.ts`

**Interfaces:**
- Consumes: `ScanRequest` (Task 7).
- Produces:
  - `ScanSettings { scanRoot: string; mode: "quick" | "deep"; depth: number; detectors: string[]; includeIgnored: boolean }`
  - `DEFAULT_SCAN_SETTINGS: ScanSettings`
  - `restoreScanSettings(raw: string | null, availableKinds: string[]): ScanSettings` — pure
  - `toScanRequest(settings: ScanSettings): ScanRequest` — pure
  - `loadScanSettings(availableKinds: string[]): ScanSettings`, `saveScanSettings(settings: ScanSettings): void`

- [ ] **Step 1: Write the failing tests**

Create `src/lib/scanSettings.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import {
  DEFAULT_SCAN_SETTINGS,
  restoreScanSettings,
  toScanRequest,
  type ScanSettings,
} from "./scanSettings";

const KINDS = ["git", "unreal"];

const stored: ScanSettings = {
  scanRoot: "/home/user/projects",
  mode: "deep",
  depth: 3,
  detectors: ["git"],
  includeIgnored: true,
};

describe("restoreScanSettings", () => {
  it("returns defaults when nothing is stored", () => {
    expect(restoreScanSettings(null, KINDS)).toEqual({
      ...DEFAULT_SCAN_SETTINGS,
      detectors: KINDS,
    });
  });

  it("round-trips stored settings", () => {
    expect(restoreScanSettings(JSON.stringify(stored), KINDS)).toEqual(stored);
  });

  // Hand-edited, written by an older build, or truncated. A broken preference
  // is not worth an error — it falls back like an absent one.
  it("falls back to defaults on unparseable json", () => {
    expect(restoreScanSettings("{not json", KINDS)).toEqual({
      ...DEFAULT_SCAN_SETTINGS,
      detectors: KINDS,
    });
  });

  it("falls back on a value of the wrong shape", () => {
    expect(restoreScanSettings(JSON.stringify({ scanRoot: 42 }), KINDS)).toEqual({
      ...DEFAULT_SCAN_SETTINGS,
      detectors: KINDS,
    });
  });

  // A detector removed from the build must not stay ticked in a form that
  // cannot show it — the tick-list is built from what the binary registers.
  it("drops stored detectors the build no longer has", () => {
    const raw = JSON.stringify({ ...stored, detectors: ["git", "gone"] });
    expect(restoreScanSettings(raw, KINDS).detectors).toEqual(["git"]);
  });

  // Every ticked detector having been removed would mean a scan that silently
  // finds nothing, so fall back to all of them rather than none.
  it("falls back to every kind when none of the stored ones survive", () => {
    const raw = JSON.stringify({ ...stored, detectors: ["gone"] });
    expect(restoreScanSettings(raw, KINDS).detectors).toEqual(KINDS);
  });

  it("clamps a nonsensical depth", () => {
    const shallow = JSON.stringify({ ...stored, depth: 0 });
    const absurd = JSON.stringify({ ...stored, depth: 999 });
    expect(restoreScanSettings(shallow, KINDS).depth).toBe(1);
    expect(restoreScanSettings(absurd, KINDS).depth).toBe(10);
  });
});

describe("toScanRequest", () => {
  // ScanMode is an internally-tagged enum flattened into ScanRequest, so quick
  // must not carry a depth field at all — serde would reject the unknown key.
  it("omits depth for a quick scan", () => {
    const request = toScanRequest({ ...stored, mode: "quick" });
    expect(request).toEqual({
      scan_root: stored.scanRoot,
      mode: "quick",
      detectors: ["git"],
      include_ignored: true,
    });
    expect("depth" in request).toBe(false);
  });

  it("carries depth for a deep scan", () => {
    expect(toScanRequest(stored)).toEqual({
      scan_root: stored.scanRoot,
      mode: "deep",
      depth: 3,
      detectors: ["git"],
      include_ignored: true,
    });
  });
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm test scanSettings`
Expected: FAIL — `Failed to resolve import "./scanSettings"`.

- [ ] **Step 3: Write the implementation**

Create `src/lib/scanSettings.ts`:

```ts
import type { ScanRequest } from "./api/types";

// The last scan's settings, so the form comes up as you left it.
//
// This is the whole of "remembering a scan". A `scan_roots` table with its own
// ports, CRUD and a schema migration was designed and cut: the path was never
// the friction — anybody scanning ~/projects knows where ~/projects is — and a
// rescan cannot skip the disk regardless, since a project that appeared
// yesterday is discoverable only by looking. What is actually worth keeping is
// the settings, because a rescan that quietly ran depth 2 instead of depth 4
// gives a different answer with nothing on screen saying why.
//
// localStorage rather than projects.db for the same reason viewState.ts uses
// it: the database is a cross-app contract devmon attaches read-only, and one
// user's scan folder is none of its business.
export interface ScanSettings {
  scanRoot: string;
  mode: "quick" | "deep";
  depth: number;
  detectors: string[];
  includeIgnored: boolean;
}

const KEY = "pi.scanSettings";

// Depth is a user-chosen number, so it needs a floor and a ceiling. Ten is
// well past any sane project layout and keeps a typo from turning a scan into
// a full-disk walk that only MAX_DIRECTORIES ends.
const MIN_DEPTH = 1;
const MAX_DEPTH = 10;

// `detectors` is empty here and filled from the build's registered kinds by
// restoreScanSettings — this module does not know what detectors exist.
export const DEFAULT_SCAN_SETTINGS: ScanSettings = {
  scanRoot: "",
  mode: "quick",
  depth: 2,
  detectors: [],
  includeIgnored: false,
};

function clampDepth(value: unknown): number {
  if (typeof value !== "number" || !Number.isFinite(value)) return DEFAULT_SCAN_SETTINGS.depth;
  return Math.min(MAX_DEPTH, Math.max(MIN_DEPTH, Math.trunc(value)));
}

// Pure. `raw` is whatever was in storage — possibly written by an older build,
// possibly hand-edited, possibly absent. `availableKinds` is what this build
// registers, which is authoritative: a stored detector the binary no longer
// has cannot be shown as a tick, so it is dropped.
export function restoreScanSettings(raw: string | null, availableKinds: string[]): ScanSettings {
  const fallback: ScanSettings = { ...DEFAULT_SCAN_SETTINGS, detectors: [...availableKinds] };
  if (!raw) return fallback;

  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return fallback;
  }
  if (typeof parsed !== "object" || parsed === null) return fallback;

  const value = parsed as Partial<Record<keyof ScanSettings, unknown>>;
  if (typeof value.scanRoot !== "string") return fallback;

  const detectors = Array.isArray(value.detectors)
    ? value.detectors.filter((k): k is string => typeof k === "string" && availableKinds.includes(k))
    : [];

  return {
    scanRoot: value.scanRoot,
    mode: value.mode === "deep" ? "deep" : "quick",
    depth: clampDepth(value.depth),
    // Every stored detector having been removed would mean a scan that
    // silently finds nothing, so fall back to all of them rather than none.
    detectors: detectors.length > 0 ? detectors : [...availableKinds],
    includeIgnored: value.includeIgnored === true,
  };
}

// Pure. ScanMode is an internally-tagged serde enum flattened into
// ScanRequest, so a quick scan must carry no `depth` key at all — serde
// rejects unknown fields on the deny-unknown side of a flattened enum.
export function toScanRequest(settings: ScanSettings): ScanRequest {
  const base = {
    scan_root: settings.scanRoot,
    detectors: settings.detectors,
    include_ignored: settings.includeIgnored,
  };
  return settings.mode === "deep"
    ? { ...base, mode: "deep", depth: settings.depth }
    : { ...base, mode: "quick" };
}

// Storage access, split from the rules above so the rules are testable in
// vitest's node environment. Guarded because a browser with site data blocked
// throws on the accessor itself.
export function loadScanSettings(availableKinds: string[]): ScanSettings {
  try {
    const raw = typeof localStorage === "undefined" ? null : localStorage.getItem(KEY);
    return restoreScanSettings(raw, availableKinds);
  } catch {
    return restoreScanSettings(null, availableKinds);
  }
}

export function saveScanSettings(settings: ScanSettings): void {
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(KEY, JSON.stringify(settings));
  } catch {
    // A preference that cannot be saved is not worth an error banner.
  }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `pnpm test scanSettings`
Expected: PASS — all eleven tests.

- [ ] **Step 5: Commit**

```bash
git add src/lib/scanSettings.ts src/lib/scanSettings.test.ts
git commit -m "feat(ui): remember the last scan's settings in localStorage"
```

---

### Task 9: The scan modal

One modal, three steps: configure, review, result. Wired into the page beside "New project".

**Files:**
- Create: `src/lib/components/ScanFolderModal.svelte`
- Modify: `src/routes/+page.svelte`

**Interfaces:**
- Consumes: `scanFolder`, `importScanned`, `listDetectorKinds` (Task 7); `loadScanSettings`, `saveScanSettings`, `toScanRequest`, `ScanSettings` (Task 8); the existing `DirectoryField`, `ErrorBanner` components and `styles.ts` classes.
- Produces: `ScanFolderModal` with props `{ onImported: (report: ImportReport) => void; onClose: () => void; onerror: (message: string) => void }`.

- [ ] **Step 1: Read the components this one has to match**

Read these first — this task follows their conventions rather than inventing new ones:
- `src/lib/components/CreateProjectModal.svelte` — modal shell, backdrop, close behaviour
- `src/lib/components/DirectoryField.svelte` — the Browse dialog, already written
- `src/lib/components/styles.ts` — `buttonClass`, `primaryButtonClass`, `inputClass`, `labelClass`
- `src/lib/components/TrackerBadges.svelte` — how matched kinds render

- [ ] **Step 2: Write the modal**

Create `src/lib/components/ScanFolderModal.svelte`:

```svelte
<script lang="ts">
  import { importScanned, listDetectorKinds, scanFolder } from "$lib/api/scan";
  import type { Candidate, ImportReport } from "$lib/api/types";
  import {
    loadScanSettings,
    saveScanSettings,
    toScanRequest,
    type ScanSettings,
  } from "$lib/scanSettings";
  import DirectoryField from "./DirectoryField.svelte";
  import { buttonClass, inputClass, labelClass, primaryButtonClass } from "./styles";

  let {
    onImported,
    onClose,
    onerror,
  }: {
    onImported: (report: ImportReport) => void;
    onClose: () => void;
    onerror: (message: string) => void;
  } = $props();

  // configure → review → result. One modal rather than three, because the
  // review step only makes sense as the middle of this flow.
  let step = $state<"configure" | "review" | "result">("configure");
  let kinds = $state<string[]>([]);
  let settings = $state<ScanSettings>(loadScanSettings([]));
  let scanning = $state(false);
  let importing = $state(false);
  let report = $state<{ candidates: Candidate[]; visited: number; stopped_early: boolean } | null>(
    null,
  );
  let result = $state<ImportReport | null>(null);
  let selected = $state<Set<string>>(new Set());
  let names = $state<Record<string, string>>({});
  let showTracked = $state(false);

  // The tick-list is built from what the binary registers, never a hardcoded
  // array — invariant 1, "a new detector is implement + register, zero
  // frontend code".
  $effect(() => {
    listDetectorKinds()
      .then((available) => {
        kinds = available;
        settings = loadScanSettings(available);
      })
      .catch((err: Error) => onerror(err.message));
  });

  const untracked = $derived(report?.candidates.filter((c) => !c.already_tracked) ?? []);
  const tracked = $derived(report?.candidates.filter((c) => c.already_tracked) ?? []);
  const visible = $derived(showTracked ? (report?.candidates ?? []) : untracked);

  function toggleDetector(kind: string) {
    settings.detectors = settings.detectors.includes(kind)
      ? settings.detectors.filter((k) => k !== kind)
      : [...settings.detectors, kind];
  }

  async function runScan() {
    scanning = true;
    try {
      const found = await scanFolder(toScanRequest(settings));
      report = found;
      // Already-tracked rows start unticked: importing them is a no-op, and
      // pre-selecting them would make the count lie about what will happen.
      selected = new Set(found.candidates.filter((c) => !c.already_tracked).map((c) => c.directory));
      names = Object.fromEntries(found.candidates.map((c) => [c.directory, c.suggested_name]));
      step = "review";
    } catch (err) {
      onerror((err as Error).message);
    } finally {
      scanning = false;
    }
  }

  async function runImport() {
    importing = true;
    try {
      const selections = [...selected].map((directory) => ({
        directory,
        name: names[directory] ?? "",
      }));
      const imported = await importScanned(selections);
      result = imported;
      // Written on commit, not on scan: an abandoned review leaves nothing
      // behind, so an exploratory scan cannot overwrite settings that worked.
      saveScanSettings(settings);
      step = "result";
      onImported(imported);
    } catch (err) {
      onerror((err as Error).message);
    } finally {
      importing = false;
    }
  }

  function toggleRow(directory: string) {
    const next = new Set(selected);
    if (next.has(directory)) next.delete(directory);
    else next.add(directory);
    selected = next;
  }
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4">
  <div class="max-h-[85vh] w-full max-w-3xl overflow-y-auto border border-line bg-base p-6">
    <h2 class="mb-4 text-xl text-phos">Scan a folder for projects</h2>

    {#if step === "configure"}
      <div class="flex flex-col gap-4">
        <DirectoryField
          bind:value={settings.scanRoot}
          label="Folder to scan"
          required
          {onerror}
        />

        <fieldset class="flex flex-col gap-2">
          <legend class={labelClass}>How deep</legend>
          <label class="flex items-center gap-2">
            <input type="radio" bind:group={settings.mode} value="quick" />
            Quick — only the folders directly inside
          </label>
          <label class="flex items-center gap-2">
            <input type="radio" bind:group={settings.mode} value="deep" />
            Deep — descend
            <input
              type="number"
              min="1"
              max="10"
              bind:value={settings.depth}
              disabled={settings.mode !== "deep"}
              class="{inputClass} w-16"
            />
            levels
          </label>
        </fieldset>

        <fieldset class="flex flex-col gap-2">
          <legend class={labelClass}>Scan for</legend>
          {#each kinds as kind (kind)}
            <label class="flex items-center gap-2">
              <input
                type="checkbox"
                checked={settings.detectors.includes(kind)}
                onchange={() => toggleDetector(kind)}
              />
              {kind}
            </label>
          {/each}
          <p class="text-sm text-dim">
            This decides which folders are worth importing. Whatever you import is
            checked against every detector, so a folder that is both lands once
            carrying both.
          </p>
        </fieldset>

        <label class="flex items-center gap-2">
          <input type="checkbox" bind:checked={settings.includeIgnored} />
          Include node_modules, target, build and other usually-ignored folders
        </label>

        <div class="flex justify-end gap-2">
          <button type="button" class={buttonClass} onclick={onClose}>Cancel</button>
          <button
            type="button"
            class={primaryButtonClass}
            disabled={scanning || !settings.scanRoot || settings.detectors.length === 0}
            onclick={runScan}
          >
            {scanning ? "Scanning…" : "Scan"}
          </button>
        </div>
      </div>
    {:else if step === "review"}
      <div class="flex flex-col gap-3">
        <p class="text-sm text-dim">
          Found {untracked.length} new
          {untracked.length === 1 ? "project" : "projects"} in {report?.visited ?? 0} folders.
        </p>

        {#if report?.stopped_early}
          <p class="border border-warn p-2 text-sm">
            Stopped after 50,000 folders — these results are incomplete. Narrow the
            folder or reduce the depth.
          </p>
        {/if}

        {#if tracked.length > 0}
          <button
            type="button"
            class="self-start text-sm underline"
            onclick={() => (showTracked = !showTracked)}
          >
            {showTracked ? "Hide" : "Show"} {tracked.length} already tracked
          </button>
        {/if}

        <div class="flex gap-2">
          <button
            type="button"
            class={buttonClass}
            onclick={() => (selected = new Set(untracked.map((c) => c.directory)))}
          >
            Select all
          </button>
          <button type="button" class={buttonClass} onclick={() => (selected = new Set())}>
            Select none
          </button>
        </div>

        <ul class="flex flex-col gap-2">
          {#each visible as candidate (candidate.directory)}
            <li class="flex items-center gap-2 border border-line p-2">
              <input
                type="checkbox"
                checked={selected.has(candidate.directory)}
                disabled={candidate.already_tracked}
                onchange={() => toggleRow(candidate.directory)}
              />
              <input
                class="{inputClass} w-56"
                bind:value={names[candidate.directory]}
                disabled={candidate.already_tracked}
              />
              <span class="flex-1 truncate text-sm text-dim" title={candidate.directory}>
                {candidate.directory}
              </span>
              <span class="text-xs text-dim">{candidate.matched_kinds.join(", ")}</span>
              {#if candidate.already_tracked}
                <span class="text-xs text-dim">already tracked</span>
              {:else if names[candidate.directory] !== candidate.suggested_name}
                <span class="text-xs text-dim">renamed</span>
              {/if}
            </li>
          {/each}
        </ul>

        <div class="flex justify-end gap-2">
          <button type="button" class={buttonClass} onclick={() => (step = "configure")}>
            Back
          </button>
          <button
            type="button"
            class={primaryButtonClass}
            disabled={importing || selected.size === 0}
            onclick={runImport}
          >
            {importing ? "Importing…" : `Import ${selected.size}`}
          </button>
        </div>
      </div>
    {:else}
      <div class="flex flex-col gap-3">
        <p>
          Imported {result?.imported.length ?? 0}
          {(result?.imported.length ?? 0) === 1 ? "project" : "projects"}.
          {#if (result?.skipped ?? 0) > 0}
            Skipped {result?.skipped} already tracked.
          {/if}
        </p>

        {#if (result?.failures.length ?? 0) > 0}
          <div>
            <p class="mb-1">{result?.failures.length} could not be imported:</p>
            <ul class="flex flex-col gap-1 text-sm text-dim">
              {#each result?.failures ?? [] as failure (failure.directory)}
                <li>{failure.directory} — {failure.message}</li>
              {/each}
            </ul>
          </div>
        {/if}

        <div class="flex justify-end">
          <button type="button" class={primaryButtonClass} onclick={onClose}>Done</button>
        </div>
      </div>
    {/if}
  </div>
</div>
```

Note on classes: `text-dim`, `border-warn`, `bg-base`, `text-phos` and `border-line` come from the project's `@theme` tokens. Check `src/app.css` and swap any token that does not exist for the nearest one that does — do not invent new tokens in this task.

- [ ] **Step 3: Wire it into the page**

In `src/routes/+page.svelte`:

Add the import beside the existing `CreateProjectModal` import (around line 13):

```ts
  import ScanFolderModal from "$lib/components/ScanFolderModal.svelte";
```

Add state beside the existing `createOpen`:

```ts
  let scanOpen = $state(false);
```

Add a button beside "+ New project" (around line 258), inside the same flex container:

```svelte
    <button
      type="button"
      onclick={() => {
        scanOpen = true;
        error = "";
      }}
      class={buttonClass}
    >
      Scan folder
    </button>
```

(`buttonClass` is already imported in this file if `primaryButtonClass` is; check the import line and add it if not.)

Add the modal beside the `{#if createOpen}` block (around line 323):

```svelte
{#if scanOpen}
  <ScanFolderModal
    onImported={handleCreated}
    onClose={() => (scanOpen = false)}
    onerror={handleError}
  />
{/if}
```

`handleCreated` reloads the project list; check its signature in this file and adapt the call if it expects a single `Project` — if so, use `onImported={() => handleCreated()}` or whatever refresh function the file already uses after a mutation, rather than changing `handleCreated`.

- [ ] **Step 4: Verify**

Run: `pnpm check && pnpm test`
Expected: no type errors, all tests pass.

Then run the app and check the flow by hand — this is the first time the feature is visible:

Run: `pnpm tauri dev`

Verify:
1. "Scan folder" opens the modal with git ticked.
2. Pointing it at a folder with two or more git repositories inside, Quick, finds them.
3. Deselecting one and importing registers only the other.
4. Re-opening the modal shows the same folder and settings pre-filled.
5. Re-scanning the same folder shows the imported projects as "already tracked" behind the toggle, and unticked.
6. Two folders both named `api` under different parents import as `api` and `<parent>/api`.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/ScanFolderModal.svelte src/routes/+page.svelte
git commit -m "feat(ui): add the folder scan modal"
```

---

### Task 10: Documentation

The feature is not done until the docs say what shipped.

**Files:**
- Modify: `docs/checklist.md` (the NEXT item, around line 133)
- Modify: `ROADMAP.md` (the *Scanning a folder for projects* section)
- Modify: `CHANGELOG.md` (Unreleased)
- Modify: `docs/architecture.md` (*Detection semantics* — note `inspect_kinds` beside `inspect`)
- Modify: `docs/USAGE.md` (how to use the scan)

- [ ] **Step 1: Mark the checklist item shipped**

In `docs/checklist.md`, move the whole scanning entry from *Open (features)* into the shipped list and replace it with:

```markdown
- [x] **Scan a folder for projects.** Point the app at `~/projects`, tick the
  detectors to scan for, choose a **quick scan** (the folders directly inside)
  or a **deep scan** to a depth you pick, then review what was found and import
  it in one pass. The walk is a pure iterative BFS in `core::domain::scan` —
  `std::fs` only, no new dependency — and `ScanService` orchestrates it over
  `ProjectService`. **The tick decides whether to register, not what gets
  recorded:** a kept directory runs every installed detector and lands once
  carrying every tracker it matched, via the new set filter
  `DetectorRunner::inspect_kinds(path, Option<&[&str]>)`; the single-kind
  `inspect` stays for the re-detect sweep and is now implemented over it.
  A matched directory ends that branch, since a repo inside a repo is vendored
  or a submodule — and `.git`/`.svn`/`.hg` are pruned **unconditionally**,
  outside the include-ignored checkbox, because submodules live under
  `.git/modules` and a walk reaching them would offer to import every one.
  Name collisions resolve through `domain::naming::disambiguate` — `api`
  becomes `work/api`, then `work/api (2)` — shared with `ensure_project`,
  which fixes a real bug there: it previously returned `DuplicateName` on the
  second `api` it met. Import is best-effort per row, so one corrupt
  repository costs that row and not the other 199; `refresh_trackers` and
  `into_result()` are never used in the bulk path. **No schema change** —
  the last scan's settings live in `localStorage` (`scanSettings.ts`), which
  is the whole of "remembering": a `scan_roots` table was designed and cut,
  because the path was never the friction and a rescan cannot skip the disk
  anyway. The walk is bounded at 50,000 directories and reports
  `stopped_early` rather than growing progress events and a cancel button.
  Spec: `docs/superpowers/specs/2026-09-07-folder-scanning-design.md`; plan:
  `docs/superpowers/plans/2026-09-07-folder-scanning.md`.
```

Then move the `**NEXT —**` marker onto the re-detect sweep item, which becomes the next thing built.

- [ ] **Step 2: Update the roadmap**

In `ROADMAP.md`, change *Scanning a folder for projects* from "This is the next thing built" to shipped, keeping the settled-decision record intact — the "considered and ruled out" material is the point of that document. Update *Where things stand* to mention bulk import. Leave the schema version at 3.

- [ ] **Step 3: Add a changelog entry**

Under `## [Unreleased]` → `### Added` in `CHANGELOG.md`:

```markdown
- Scan a folder for projects: point the app at a directory, tick the detectors
  to scan for, choose a quick scan of the folders directly inside or a deep
  scan to a depth you pick, then review what was found and import it in one
  pass. Names that collide are qualified by their parent folder (`work/api`)
  and can be edited before importing. The last scan's settings are remembered,
  so rescanning to pick up what is new is a matter of reopening the dialog and
  pressing Scan.
```

- [ ] **Step 4: Note the runner's two shapes**

In `docs/architecture.md` → *Detection semantics*, add a short paragraph after the outcome table recording that `inspect_kinds(path, Option<&[&str]>)` is the folder scanner's selection filter and `inspect(path, Option<&str>)` is the re-detect sweep's single-kind form, that both are real, and that the selection decides whether to register rather than what gets recorded.

- [ ] **Step 5: Document the feature for users**

In `docs/USAGE.md`, add a section covering: where the button is, quick versus deep, what the detector ticks mean (which folders get imported, not what gets recorded), that usually-ignored folders are skipped by default, that a folder which is itself a project is not descended into, the review step, and that settings are remembered for the next scan.

- [ ] **Step 6: Commit**

```bash
git add docs/checklist.md ROADMAP.md CHANGELOG.md docs/architecture.md docs/USAGE.md
git commit -m "docs: record folder scanning as shipped"
```

---

## Verification

After Task 10, confirm the whole thing from a clean state:

```bash
cargo test -p indexer-core
cargo build --manifest-path src-tauri/Cargo.toml
pnpm check
pnpm test
```

All four must pass. Then re-run the six manual checks from Task 9 step 4 — automated tests do not cover the modal, so that walkthrough is the only thing standing between this and a broken window.
