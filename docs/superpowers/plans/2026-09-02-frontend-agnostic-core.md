# Frontend-agnostic core — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract all domain logic, orchestration, and persistence into a Tauri-free `indexer-core` library crate; the GUI becomes a thin adapter over it. Swap `tauri-plugin-store` for a SQLite repository. No user-visible change.

**Architecture:** Cargo workspace: `crates/core` (`indexer-core`, no `tauri`), `src-tauri` (GUI adapter), `crates/cli` (stub). Two ports (`ProjectRepository`, `AppLauncher`); one `ProjectService` holding `Arc<dyn ProjectRepository>` + `Arc<dyn AppLauncher>` + `Arc<DetectorRunner>`. `#[tauri::command]` functions shrink to ~3-line pass-throughs over `State<Arc<ProjectService>>`.

**Tech Stack:** Rust, Tauri v2, `rusqlite` (bundled), SvelteKit frontend (unchanged bar one command call).

**Spec:** `docs/superpowers/specs/2026-09-02-frontend-agnostic-core-design.md` — read it alongside this plan.

## Global Constraints

- **`indexer-core` never depends on `tauri`, `tauri-plugin-*`, or `clap`.** A `use tauri::` in `core` is a bug. Verify with `cargo tree -p indexer-core | grep -i tauri` → no output.
- **One public application error: `ProjectError`.** Its `impl serde::Serialize` (Display → plain string) is preserved verbatim — the JS side keeps receiving string errors. Ports get their own small errors (`RepositoryError`, `LauncherError`), mapped into `ProjectError` in the service via `From` impls.
- **Zero user-visible GUI change.** Every `#[tauri::command]` keeps its exact name and argument/return JSON shape. The one addition is a new command `suggest_project_name`. The frontend changes in exactly one file (`CreateProjectForm.svelte`).
- **All existing tests move with their code and pass unchanged.** `src-tauri`'s current `cargo test --lib` count is the floor; after the move the same tests run under `cargo test -p indexer-core`.
- **No `serde_json::Value` migration layer.** `src-tauri/src/migrations/` is deleted, not moved. Schema evolution is the SQLite `user_version` runner only. The `#[serde(default)]` / `Option<T>` rule for `Project` fields stays.
- **SQLite is a document store:** `Project` serialized whole into a `data` TEXT column; `is_deleted` / `directory_normalized` / `updated_at` promoted for querying; `tags` also written to a `project_tags` table (derived — blob stays source of truth). `trackers` is blob-only.
- **`rusqlite` uses `features = ["bundled"]`** — no system SQLite, needs a C compiler (present on all CI runners).
- Every commit message ends with:
  `Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>`
- **Do not commit `pnpm-lock.yaml`.** `npm run tauri dev` dirties it via `beforeDevCommand: "pnpm dev"`; run `git checkout -- pnpm-lock.yaml` before each commit.
- **Kill the running dev app before any `cargo build`** — the Windows linker cannot overwrite `project-indexer.exe` while it runs.
- `cargo fmt` clean, `cargo clippy` clean bar the 2 known warnings (`sort_by_key`, module-inception), `npm run check` clean bar the 8 known `EditProjectForm` warnings.

---

## File Structure

### New — `crates/core/` (`indexer-core`)

| File | Responsibility |
|---|---|
| `Cargo.toml` | `serde`, `serde_json`, `chrono`, `uuid`, `thiserror`, `git2`, `rusqlite` (bundled); Windows-only `winreg`, `parselnk`. No `tauri`. |
| `src/lib.rs` | `pub mod domain / ports / application / detectors / platform / infra / error;` + curated re-exports. |
| `src/domain/mod.rs` | re-exports `Project`, `Tracker`, `GitInfo`, `UnrealInfo`, `InstalledApp`, `UpdateProject`, the `normalize`/`sorting`/`naming` functions. |
| `src/domain/project.rs` | `Project` + methods (moved verbatim from `models/project.rs`). |
| `src/domain/tracker.rs`, `git.rs`, `unreal.rs`, `installed_app.rs`, `update_project.rs` | moved verbatim from `models/`. |
| `src/domain/normalize.rs`, `src/domain/sorting.rs` | moved verbatim from `utils/`. |
| `src/domain/naming.rs` | **new** — `repo_name_from_url`, `folder_name_from_directory`, `suggest_project_name` (ported from `CreateProjectForm.svelte`). |
| `src/error/mod.rs` | re-exports all error types. |
| `src/error/project_error.rs`, `detector_error.rs`, `git.rs`, `unreal.rs` | moved verbatim from `errors/`. |
| `src/error/repository.rs` | **new** — `RepositoryError`. |
| `src/error/launcher.rs` | **new** — `LauncherError`. |
| `src/ports/mod.rs` | re-exports `ProjectReader`, `ProjectRepository`, `AppLauncher`. |
| `src/ports/repository.rs` | **new** — `ProjectReader`, `ProjectRepository: ProjectReader`. |
| `src/ports/launcher.rs` | **new** — `AppLauncher`. |
| `src/application/mod.rs` | re-exports `ProjectService`, `ProjectInspection`, `DetectorResult`, `DetectorStatus`, `DirectoryState`. |
| `src/application/service.rs` | **new** — `ProjectService`, one method per current command. |
| `src/application/inspection.rs` | **new** — inspection DTOs (moved out of `commands/inspect.rs`), `results_from`. |
| `src/detectors/**` | moved verbatim from `detectors/` (`detector.rs`, `runner.rs`, `registry.rs`, `git/`, `unreal/`). |
| `src/platform/mod.rs` | re-exports `check_directory_status`, `DirectoryStatus`, `remove_directory`, `list_installed_apps`, `open_with_app_available`. |
| `src/platform/filesystem.rs` | `check_directory_status` + `DirectoryStatus` (from `utils/filesystem.rs`) + `remove_directory` (from `commands/system.rs`). |
| `src/platform/app_discovery.rs` | `list_installed_apps`, `open_with_app_available`, `command_exists`, `windows_path_extensions`, `windows_impl`, `linux_impl` + their tests (from `commands/system.rs`). |
| `src/infra/mod.rs` | re-exports `SqliteRepository`, `CURRENT_SCHEMA_VERSION`. |
| `src/infra/sqlite_repository.rs` | **new** — `SqliteRepository`, schema, `user_version` runner + skew guard. |

### New — `crates/cli/`

| File | Responsibility |
|---|---|
| `Cargo.toml` | package `indexer-cli`; depends on nothing (stub). |
| `src/main.rs` | `fn main() { eprintln!("project-indexer CLI: not implemented (spec 2)"); std::process::exit(1); }` |

### New — repo root

| File | Responsibility |
|---|---|
| `Cargo.toml` | `[workspace]` with `members = ["src-tauri", "crates/core", "crates/cli"]`, `resolver = "2"`. |

### Modified — `src-tauri/src/`

| File | Change |
|---|---|
| `Cargo.toml` | drop `tauri-plugin-store`, `git2`, `winreg`, `parselnk`; add `indexer-core = { path = "../crates/core" }`. |
| `lib.rs` | `setup` builds `SqliteRepository` + `OpenerLauncher` + `DetectorRunner` → `ProjectService` → `app.manage`. Remove the store plugin, the `on_window_event` flush, the separate `.manage(DetectorRunner)`. Register `suggest_project_name`. |
| `adapters/mod.rs`, `adapters/opener_launcher.rs` | **new** — `OpenerLauncher` impl `AppLauncher` (the current `open_in_app` Windows/Linux logic). |
| `commands/projects.rs` | every fn → `State<Arc<ProjectService>>` pass-through; add `suggest_project_name`. |
| `commands/inspect.rs` | → `service.inspect(...)` pass-through; DTOs re-exported from `indexer_core`. |
| `commands/system.rs` | → `indexer_core::platform::list_installed_apps()` pass-through; delete everything else. |
| `models/`, `errors/`, `utils/`, `detectors/`, `migrations/`, `store/` | **deleted** (contents moved to `core`, or — `migrations/`, `store/` — deleted outright). |

### Modified — frontend

| File | Change |
|---|---|
| `src/lib/components/CreateProjectForm.svelte` | replace inline `repoNameFromUrl` / `folderNameFromDirectory` / `suggestProjectName` with one `invoke("suggest_project_name", { directory })` call. |
| `src/lib/api/projects.ts` | add `suggestProjectName(directory)`. |

---

## Task 1: Workspace scaffold

**Files:**
- Create: `Cargo.toml` (repo root)
- Create: `crates/core/Cargo.toml`, `crates/core/src/lib.rs`
- Create: `crates/cli/Cargo.toml`, `crates/cli/src/main.rs`
- Modify: `src-tauri/Cargo.toml`

**Interfaces:**
- Produces: a buildable 3-member workspace; `indexer-core` exposes nothing yet (`pub fn placeholder() {}`).

- [ ] **Step 1: Create the workspace manifest**

`Cargo.toml` (repo root):

```toml
[workspace]
members = ["src-tauri", "crates/core", "crates/cli"]
resolver = "2"
```

- [ ] **Step 2: Create the `indexer-core` crate**

`crates/core/Cargo.toml`:

```toml
[package]
name = "indexer-core"
version = "0.1.0"
edition = "2021"

[lib]
name = "indexer_core"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4.45", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
thiserror = "2"
git2 = "0.21.0"
rusqlite = { version = "0.32", features = ["bundled"] }

[target.'cfg(windows)'.dependencies]
winreg = "0.56"
parselnk = "0.1"
```

`crates/core/src/lib.rs`:

```rust
pub fn placeholder() {}
```

- [ ] **Step 3: Create the `indexer-cli` stub**

`crates/cli/Cargo.toml`:

```toml
[package]
name = "indexer-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "project-indexer"
path = "src/main.rs"

[dependencies]
```

`crates/cli/src/main.rs`:

```rust
fn main() {
    eprintln!("project-indexer CLI: not implemented (spec 2)");
    std::process::exit(1);
}
```

- [ ] **Step 4: Add the path dependency to `src-tauri`**

In `src-tauri/Cargo.toml` `[dependencies]`, add (do not remove anything yet):

```toml
indexer-core = { path = "../crates/core" }
```

- [ ] **Step 5: Verify the workspace builds**

Run: `cargo build --workspace`
Expected: three crates compile; a warning that `indexer_core::placeholder` is unused is fine.

- [ ] **Step 6: Verify the app still runs**

Run: `npm run tauri dev`, confirm the window opens and the project list loads, close it.
Run: `git checkout -- pnpm-lock.yaml`

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/ src-tauri/Cargo.toml
git commit -m "$(cat <<'EOF'
build: scaffold cargo workspace with indexer-core and indexer-cli crates

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Move domain, errors, and naming into `core`

Moves the pure-data layer. No logic changes. Adds `naming` and the two new port-error types. `migrations/` is deleted here.

**Files:**
- Move: `src-tauri/src/models/{project,tracker,git,unreal,installed_app,update_project}.rs` → `crates/core/src/domain/`
- Move: `src-tauri/src/utils/{normalize,sorting}.rs` → `crates/core/src/domain/`
- Move: `src-tauri/src/errors/{project_error,detector_error,git,unreal}.rs` → `crates/core/src/error/`
- Create: `crates/core/src/error/repository.rs`, `crates/core/src/error/launcher.rs`
- Create: `crates/core/src/domain/naming.rs`, `crates/core/src/domain/mod.rs`, `crates/core/src/error/mod.rs`
- Delete: `src-tauri/src/migrations/mod.rs`, `src-tauri/src/models/mod.rs`, `src-tauri/src/errors/mod.rs`
- Modify: `crates/core/src/lib.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/store/project_store.rs`, `src-tauri/src/commands/*.rs`, all moved files' `use` lines, `src-tauri/src/detectors/**`, `src-tauri/src/utils/mod.rs`

**Interfaces:**
- Produces:
  - `indexer_core::domain::{Project, Tracker, GitInfo, UnrealInfo, InstalledApp, UpdateProject}`
  - `indexer_core::domain::normalize::{normalize_directory, normalize_tag, normalize_tags, remove_spaces}`
  - `indexer_core::domain::sorting::{SortBy, SortDirection, SortOptions, sort_projects, sort_alphabetically, sort_projects_by_recents, filter_favorites, filter_deleted}`
  - `indexer_core::domain::naming::{repo_name_from_url, folder_name_from_directory, suggest_project_name}`
  - `indexer_core::error::{ProjectError, DetectorError, GitError, UnrealError, RepositoryError, LauncherError}`
  - Also re-exported flat: `indexer_core::{Project, Tracker, ProjectError, ...}` for convenience.

- [ ] **Step 1: Write `naming.rs` failing tests**

`crates/core/src/domain/naming.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Tracker, GitInfo};

    fn git_tracker(repo_url: Option<&str>) -> Tracker {
        Tracker::Git(GitInfo {
            repo_root: "/tmp/x".into(), dirty: false, detached_head: false,
            repo_url: repo_url.map(str::to_string), web_url: None,
            contributors: vec![], curr_branch: None, branches: None, commit_hash: None,
        })
    }

    #[test]
    fn repo_name_from_https_url() {
        assert_eq!(repo_name_from_url("https://github.com/user/my-repo.git").as_deref(), Some("my-repo"));
        assert_eq!(repo_name_from_url("https://github.com/user/my-repo").as_deref(), Some("my-repo"));
    }

    #[test]
    fn repo_name_from_ssh_url() {
        assert_eq!(repo_name_from_url("git@github.com:user/my-repo.git").as_deref(), Some("my-repo"));
    }

    #[test]
    fn repo_name_ignores_trailing_slash() {
        assert_eq!(repo_name_from_url("https://github.com/user/my-repo/").as_deref(), Some("my-repo"));
    }

    #[test]
    fn folder_name_from_windows_path() {
        assert_eq!(folder_name_from_directory("D:\\Projects\\Friction\\").as_deref(), Some("Friction"));
    }

    #[test]
    fn folder_name_from_unix_path() {
        assert_eq!(folder_name_from_directory("/home/user/friction").as_deref(), Some("friction"));
    }

    #[test]
    fn suggest_prefers_git_remote_name() {
        let t = [git_tracker(Some("https://github.com/user/cool-thing.git"))];
        assert_eq!(suggest_project_name(&t, "/home/user/local-dir").as_deref(), Some("cool-thing"));
    }

    #[test]
    fn suggest_falls_back_to_folder_name() {
        let t = [git_tracker(None)];
        assert_eq!(suggest_project_name(&t, "/home/user/local-dir").as_deref(), Some("local-dir"));
        assert_eq!(suggest_project_name(&[], "/home/user/local-dir").as_deref(), Some("local-dir"));
    }

    #[test]
    fn suggest_returns_none_for_empty_directory_and_no_trackers() {
        assert_eq!(suggest_project_name(&[], ""), None);
    }
}
```

- [ ] **Step 2: Implement `naming.rs`**

Above the tests:

```rust
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
```

- [ ] **Step 3: Move the model files**

```bash
git mv src-tauri/src/models/project.rs        crates/core/src/domain/project.rs
git mv src-tauri/src/models/tracker.rs        crates/core/src/domain/tracker.rs
git mv src-tauri/src/models/git.rs            crates/core/src/domain/git.rs
git mv src-tauri/src/models/unreal.rs         crates/core/src/domain/unreal.rs
git mv src-tauri/src/models/installed_app.rs  crates/core/src/domain/installed_app.rs
git mv src-tauri/src/models/update_project.rs crates/core/src/domain/update_project.rs
git mv src-tauri/src/utils/normalize.rs       crates/core/src/domain/normalize.rs
git mv src-tauri/src/utils/sorting.rs         crates/core/src/domain/sorting.rs
rm src-tauri/src/models/mod.rs
```

- [ ] **Step 4: Move the error files**

```bash
git mv src-tauri/src/errors/project_error.rs  crates/core/src/error/project_error.rs
git mv src-tauri/src/errors/detector_error.rs crates/core/src/error/detector_error.rs
git mv src-tauri/src/errors/git.rs            crates/core/src/error/git.rs
git mv src-tauri/src/errors/unreal.rs         crates/core/src/error/unreal.rs
rm src-tauri/src/errors/mod.rs
rm src-tauri/src/migrations/mod.rs
```

- [ ] **Step 5: Rewrite `use` paths in the moved files**

In every moved file, apply these substitutions:

| Old | New |
|---|---|
| `crate::models::tracker::Tracker` | `crate::domain::tracker::Tracker` |
| `crate::models::git::GitInfo` | `crate::domain::git::GitInfo` |
| `crate::models::unreal::UnrealInfo` | `crate::domain::unreal::UnrealInfo` |
| `crate::models::update_project::UpdateProject` | `crate::domain::update_project::UpdateProject` |
| `crate::models::{Project, ...}` / `crate::models::Project` | `crate::domain::{Project, ...}` |
| `crate::utils::normalize::{...}` | `crate::domain::normalize::{...}` |
| `crate::utils::filesystem::{check_directory_status, DirectoryStatus}` | `crate::platform::filesystem::{check_directory_status, DirectoryStatus}` |
| `crate::errors::{ProjectError, ...}` / `crate::errors::GitError` etc. | `crate::error::{...}` |
| `crate::migrations::migrate` (in `project.rs` tests) | *remove* — delete `loads_a_legacy_record_through_the_migration_path` (Step 7) |

`project.rs` currently imports `crate::utils::filesystem` — that module doesn't exist in `core` yet (Task 3). For this task, temporarily add a stub `crates/core/src/platform/filesystem.rs` containing only `check_directory_status` + `DirectoryStatus` copied from `src-tauri/src/utils/filesystem.rs` (its tests too), and `crates/core/src/platform/mod.rs` with `pub mod filesystem;`. Task 3 fills in the rest and removes the `src-tauri` original.

- [ ] **Step 6: Write the new error types**

`crates/core/src/error/repository.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("project store is unavailable: {0}")]
    Backend(String),
    #[error("project store holds a record that can't be read: {0}")]
    Corrupt(String),
}
```

`crates/core/src/error/launcher.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub struct LauncherError(pub String);
```

- [ ] **Step 7: Adjust `project.rs` tests for the deleted migration layer**

In `crates/core/src/domain/project.rs` `mod tests`:
- Delete `loads_a_legacy_record_through_the_migration_path` entirely.
- Keep `loads_a_record_missing_every_absorbable_field` and `rejects_a_record_missing_its_identity` unchanged.
- `LEGACY_RECORD` const stays (still used by the kept test).

- [ ] **Step 8: Write `crates/core/src/domain/mod.rs`**

```rust
pub mod git;
pub mod installed_app;
pub mod naming;
pub mod normalize;
pub mod project;
pub mod sorting;
pub mod tracker;
pub mod update_project;

pub use git::GitInfo;
pub use installed_app::InstalledApp;
pub use project::Project;
pub use tracker::Tracker;
pub use unreal::UnrealInfo;
pub use update_project::UpdateProject;
pub use unreal::UnrealInfo as _UnrealInfo; // (remove if unused; keep the real one above)
```

(Only include `pub use` lines that compile — mirror `src-tauri/src/models/mod.rs`'s original exports: `InstalledApp`, `Project`, `Tracker`, `UnrealInfo`, `UpdateProject`. Add `GitInfo`.)

Also `pub mod unreal;` — the file is `unreal.rs`.

- [ ] **Step 9: Write `crates/core/src/error/mod.rs`**

```rust
pub mod detector_error;
pub mod git;
pub mod launcher;
pub mod project_error;
pub mod repository;
pub mod unreal;

pub use detector_error::DetectorError;
pub use git::GitError;
pub use launcher::LauncherError;
pub use project_error::ProjectError;
pub use repository::RepositoryError;
pub use unreal::UnrealError;
```

- [ ] **Step 10: Write `crates/core/src/lib.rs`**

```rust
pub mod domain;
pub mod error;
pub mod platform; // filesystem stub for now; filled in Task 3

pub use domain::{Project, Tracker, GitInfo, UnrealInfo, InstalledApp, UpdateProject};
pub use error::{
    DetectorError, GitError, LauncherError, ProjectError, RepositoryError, UnrealError,
};
```

- [ ] **Step 11: Point `src-tauri` at the moved code**

Delete `src-tauri/src/models/`, `src-tauri/src/errors/`, `src-tauri/src/migrations/` module declarations from `src-tauri/src/lib.rs` (`pub mod models;` etc.). Add nothing — `indexer-core` is already a dep.

In every remaining `src-tauri` file (`commands/*.rs`, `detectors/**` — still in `src-tauri` until Task 3 — `store/project_store.rs`, `utils/mod.rs`, `utils/filesystem.rs`, `lib.rs`), rewrite:

| Old | New |
|---|---|
| `crate::models::` | `indexer_core::domain::` (or `indexer_core::` for the flat re-exports) |
| `crate::errors::` | `indexer_core::error::` |
| `use crate::utils::{filter_deleted, filter_favorites, sort_projects, SortOptions};` | `use indexer_core::domain::sorting::{filter_deleted, filter_favorites, sort_projects, SortOptions};` |
| `crate::utils::sort_projects_by_recents` | `indexer_core::domain::sorting::sort_projects_by_recents` |
| `crate::migrations::migrate(value)` (in `store/project_store.rs`) | *remove the call* — `set`/`get` the value directly; delete the `use crate::migrations;` line. (`store/` is deleted in Task 7 anyway; this just keeps it compiling now.) |

`src-tauri/src/utils/mod.rs` now only needs `pub mod filesystem;` (until Task 3). Delete the `pub use sorting::...` / `pub use normalize::...` lines.

- [ ] **Step 12: Run core tests**

Run: `cargo test -p indexer-core`
Expected: all moved model/normalize/sorting/error tests pass, plus the 8 new `naming` tests. Count ≈ (moved) + 8.

- [ ] **Step 13: Build and smoke-test the app**

Kill any running dev app. Run: `cargo build -p project-indexer`
Expected: compiles (warnings about the soon-to-be-moved `detectors`/`utils` are fine).
Run: `npm run tauri dev`, create a project, edit it, favorite it, delete it, restore it — all work. Close. `git checkout -- pnpm-lock.yaml`.

- [ ] **Step 14: `cargo fmt` and commit**

Run: `cargo fmt --all`

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor(core): move domain models, errors, sorting, normalize into indexer-core

Adds domain::naming (ported from CreateProjectForm.svelte) and the
RepositoryError / LauncherError port errors. Deletes the serde_json::Value
migration layer — no production data to migrate.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Move detectors and platform code into `core`

**Files:**
- Move: `src-tauri/src/detectors/**` → `crates/core/src/detectors/**`
- Move: `src-tauri/src/utils/filesystem.rs` → `crates/core/src/platform/filesystem.rs` (replace the Task-2 stub; add `remove_directory`)
- Move (partial): non-Tauri parts of `src-tauri/src/commands/system.rs` → `crates/core/src/platform/app_discovery.rs`
- Modify: `src-tauri/Cargo.toml` (drop `git2`, `winreg`, `parselnk`), `crates/core/src/lib.rs`, `crates/core/src/platform/mod.rs`, `src-tauri/src/commands/system.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/utils/mod.rs` (delete)

**Interfaces:**
- Consumes: `indexer_core::domain::{Tracker}`, `indexer_core::error::{DetectorError, GitError, UnrealError}`.
- Produces:
  - `indexer_core::detectors::{Detector, DetectorRunner, Detection, DetectorOutcome, default_detectors}`
  - `indexer_core::detectors::git::Gitector`, `indexer_core::detectors::unreal::UnrealDetector`
  - `indexer_core::platform::filesystem::{check_directory_status, DirectoryStatus, remove_directory}`
  - `indexer_core::platform::app_discovery::{list_installed_apps, open_with_app_available}`
  - flat: `indexer_core::platform::{check_directory_status, DirectoryStatus, remove_directory, list_installed_apps, open_with_app_available}`

- [ ] **Step 1: Move the detectors tree**

```bash
git mv src-tauri/src/detectors crates/core/src/detectors
```

- [ ] **Step 2: Rewrite `use` paths in the detectors tree**

In `crates/core/src/detectors/**`:

| Old | New |
|---|---|
| `crate::detectors::detector::Detector` | `crate::detectors::detector::Detector` *(unchanged)* |
| `crate::detectors::registry::default_detectors` | *(unchanged)* |
| `crate::errors::{DetectorError, GitError}` | `crate::error::{DetectorError, GitError}` |
| `crate::errors::{DetectorError, UnrealError}` | `crate::error::{DetectorError, UnrealError}` |
| `crate::models::git::GitInfo` | `crate::domain::git::GitInfo` |
| `crate::models::tracker::Tracker` | `crate::domain::tracker::Tracker` |
| `crate::models::unreal::UnrealInfo` | `crate::domain::unreal::UnrealInfo` |

- [ ] **Step 3: Fill in `platform/filesystem.rs`**

Replace the Task-2 stub `crates/core/src/platform/filesystem.rs` with the full content of `src-tauri/src/utils/filesystem.rs` (identical — `check_directory_status`, `DirectoryStatus`, all tests), then append `remove_directory` moved from `src-tauri/src/commands/system.rs`:

```rust
/// Recursively deletes a directory, treating already-missing as success.
pub fn remove_directory(path: &str) -> Result<(), String> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Failed to delete directory: {}", e)),
    }
}
```

```bash
rm src-tauri/src/utils/filesystem.rs
rm src-tauri/src/utils/mod.rs   # utils/ is now empty
```

- [ ] **Step 4: Create `platform/app_discovery.rs`**

Move from `src-tauri/src/commands/system.rs` into `crates/core/src/platform/app_discovery.rs`, verbatim: `open_with_app_available`, `program_from_open_with`, `command_exists`, `windows_path_extensions`, the entire `#[cfg(windows)] mod windows_impl`, the entire `#[cfg(target_os = "linux")] mod linux_impl`, and the file-level `#[cfg(test)] mod tests` (the `open_with_app_available` tests). Also move `list_installed_apps`'s body into a plain `pub fn list_installed_apps() -> Vec<InstalledApp>` (drop the `#[tauri::command]` and the `Result` wrapper — it never errors):

```rust
use crate::domain::InstalledApp;

pub fn list_installed_apps() -> Vec<InstalledApp> {
    #[cfg(windows)]
    { windows_impl::list_installed_apps() }
    #[cfg(target_os = "linux")]
    { linux_impl::list_installed_apps() }
    #[cfg(not(any(windows, target_os = "linux")))]
    { Vec::new() }
}
```

Rewrite `use crate::models::InstalledApp;` → `use crate::domain::InstalledApp;` in the moved `windows_impl` / `linux_impl`.

Leave `open_in_app` in `src-tauri/src/commands/system.rs` for now (Task 6 moves it into the adapter).

- [ ] **Step 5: Write `crates/core/src/platform/mod.rs`**

```rust
pub mod app_discovery;
pub mod filesystem;

pub use app_discovery::{list_installed_apps, open_with_app_available};
pub use filesystem::{check_directory_status, remove_directory, DirectoryStatus};
```

- [ ] **Step 6: Update `crates/core/src/lib.rs`**

```rust
pub mod application; // added in Task 5, harmless to omit until then
pub mod detectors;
pub mod domain;
pub mod error;
pub mod infra;       // added in Task 4
pub mod platform;
pub mod ports;       // added in Task 4

pub use detectors::{Detection, DetectorOutcome, DetectorRunner};
pub use domain::{GitInfo, InstalledApp, Project, Tracker, UnrealInfo, UpdateProject};
pub use error::{
    DetectorError, GitError, LauncherError, ProjectError, RepositoryError, UnrealError,
};
```

(Only declare modules that exist — add `application` / `infra` / `ports` lines in their own tasks.)

- [ ] **Step 7: Move deps in Cargo.toml**

`src-tauri/Cargo.toml`: remove `git2 = "0.21.0"` from `[dependencies]` and the entire `[target.'cfg(windows)'.dependencies]` block (`winreg`, `parselnk`).
(These are already in `crates/core/Cargo.toml` from Task 1.)

- [ ] **Step 8: Point `src-tauri` at the moved detectors/platform**

`src-tauri/src/lib.rs`: remove `pub mod detectors;` and `pub mod utils;`.
In `src-tauri/src/commands/*.rs` and `src-tauri/src/lib.rs`, rewrite:

| Old | New |
|---|---|
| `crate::detectors::{DetectorRunner, Detection, ...}` | `indexer_core::detectors::{...}` (or `indexer_core::{DetectorRunner, ...}`) |
| `crate::detectors::DetectorRunner` | `indexer_core::detectors::DetectorRunner` |
| `use crate::utils::filesystem::{check_directory_status, DirectoryStatus};` | `use indexer_core::platform::{check_directory_status, DirectoryStatus};` |
| `crate::commands::system::{open_with_app_available, remove_directory}` | `indexer_core::platform::{open_with_app_available, remove_directory}` |
| `crate::commands::system::open_in_app` | *(unchanged — still in `system.rs`)* |

`commands/system.rs`: keep `open_in_app` and a thin `list_installed_apps` command:

```rust
use indexer_core::domain::InstalledApp;

#[tauri::command]
pub fn list_installed_apps() -> Result<Vec<InstalledApp>, String> {
    Ok(indexer_core::platform::list_installed_apps())
}
```

- [ ] **Step 9: Run tests**

Run: `cargo test -p indexer-core`
Expected: detector tests (`runner.rs`, `gitector.rs`, `unreal.rs`), filesystem tests, `app_discovery` tests all pass, plus Task-2's. No test lost.

Run: `cargo tree -p indexer-core | grep -i "tauri"`
Expected: **no output.**

- [ ] **Step 10: Build and smoke-test**

Kill dev app. `cargo build -p project-indexer`. `npm run tauri dev` → create a project in a real git repo dir, confirm the git badge appears; open "add project" and Browse, confirm name suggestion still works; check the "open with" app picker lists apps. Close. `git checkout -- pnpm-lock.yaml`.

- [ ] **Step 11: `cargo fmt` and commit**

```bash
cargo fmt --all
git add -A
git commit -m "$(cat <<'EOF'
refactor(core): move detectors and platform code into indexer-core

detectors/ wholesale; filesystem + installed-app discovery + launch-target
checks into platform/. git2/winreg/parselnk deps move with them. core has
no tauri in its dependency tree.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Ports and `SqliteRepository`

**Files:**
- Create: `crates/core/src/ports/mod.rs`, `ports/repository.rs`, `ports/launcher.rs`
- Create: `crates/core/src/infra/mod.rs`, `infra/sqlite_repository.rs`
- Modify: `crates/core/src/lib.rs`, `crates/core/Cargo.toml` (add `[dev-dependencies] tempfile = "3"` if a file-path test wants it — optional; `in_memory()` covers most)

**Interfaces:**
- Consumes: `indexer_core::domain::{Project}`, `indexer_core::domain::normalize::normalize_directory`, `indexer_core::error::RepositoryError`.
- Produces:
  - `indexer_core::ports::{ProjectReader, ProjectRepository, AppLauncher}`
  - `indexer_core::infra::{SqliteRepository, CURRENT_SCHEMA_VERSION}`

- [ ] **Step 1: Write the port traits**

`crates/core/src/ports/repository.rs`:

```rust
use crate::domain::Project;
use crate::error::RepositoryError;

/// Read access to stored projects. Split from [`ProjectRepository`] so an
/// external consumer (devmon) can depend on reads without the write surface.
pub trait ProjectReader: Send + Sync {
    fn get(&self, id: &str) -> Result<Option<Project>, RepositoryError>;
    /// Every project, deleted included, no ordering guarantee.
    fn list(&self) -> Result<Vec<Project>, RepositoryError>;
    /// `normalized_directory` must already be `normalize_directory`'d.
    fn find_by_directory(
        &self,
        normalized_directory: &str,
    ) -> Result<Option<Project>, RepositoryError>;
}

pub trait ProjectRepository: ProjectReader {
    /// Insert or replace by `project.id`.
    fn save(&self, project: &Project) -> Result<(), RepositoryError>;
    /// Idempotent — a missing id is `Ok(())`.
    fn delete(&self, id: &str) -> Result<(), RepositoryError>;
}
```

`crates/core/src/ports/launcher.rs`:

```rust
use crate::error::LauncherError;

pub trait AppLauncher: Send + Sync {
    /// Open `directory`, with `open_with` if given, else the OS default.
    fn open(&self, directory: &str, open_with: Option<&str>) -> Result<(), LauncherError>;
    /// Whether `open_with` names an app that can currently be launched.
    fn is_available(&self, open_with: &str) -> bool;
}
```

`crates/core/src/ports/mod.rs`:

```rust
pub mod launcher;
pub mod repository;

pub use launcher::AppLauncher;
pub use repository::{ProjectReader, ProjectRepository};
```

Add `pub mod ports;` + re-exports to `crates/core/src/lib.rs`.

- [ ] **Step 2: Write `SqliteRepository` failing tests**

`crates/core/src/infra/sqlite_repository.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Project;

    fn sample(id: &str, dir: &str) -> Project {
        let mut p = Project::new("Name".into(), dir.into(), None, Some(vec!["Rust".into()]))
            .expect("dir must exist for Project::new");
        p.id = id.to_string();
        p
    }

    // Project::new validates the directory exists, so tests point at a real temp dir.
    fn tmp() -> String {
        std::env::temp_dir().to_string_lossy().into_owned()
    }

    #[test]
    fn round_trips_a_project() {
        let repo = SqliteRepository::in_memory().unwrap();
        let p = sample("id-1", &tmp());
        repo.save(&p).unwrap();
        let got = repo.get("id-1").unwrap().unwrap();
        assert_eq!(got.id, "id-1");
        assert_eq!(got.tags, vec!["Rust".to_string()]);
    }

    #[test]
    fn save_replaces_on_conflict() {
        let repo = SqliteRepository::in_memory().unwrap();
        let mut p = sample("id-1", &tmp());
        repo.save(&p).unwrap();
        p.name = "Renamed".into();
        repo.save(&p).unwrap();
        assert_eq!(repo.list().unwrap().len(), 1);
        assert_eq!(repo.get("id-1").unwrap().unwrap().name, "Renamed");
    }

    #[test]
    fn delete_is_idempotent_and_cascades_tags() {
        let repo = SqliteRepository::in_memory().unwrap();
        repo.save(&sample("id-1", &tmp())).unwrap();
        repo.delete("id-1").unwrap();
        repo.delete("id-1").unwrap(); // no error
        assert!(repo.get("id-1").unwrap().is_none());
        let conn = repo.conn.lock().unwrap();
        let tag_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM project_tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tag_count, 0);
    }

    #[test]
    fn list_includes_deleted() {
        let repo = SqliteRepository::in_memory().unwrap();
        let mut p = sample("id-1", &tmp());
        p.is_deleted = true;
        repo.save(&p).unwrap();
        assert_eq!(repo.list().unwrap().len(), 1);
    }

    #[test]
    fn find_by_directory_matches_normalized() {
        let repo = SqliteRepository::in_memory().unwrap();
        let dir = tmp();
        repo.save(&sample("id-1", &dir)).unwrap();
        let normalized = crate::domain::normalize::normalize_directory(&dir);
        assert_eq!(repo.find_by_directory(&normalized).unwrap().unwrap().id, "id-1");
        assert!(repo.find_by_directory("/nope").unwrap().is_none());
    }

    #[test]
    fn corrupt_blob_is_reported() {
        let repo = SqliteRepository::in_memory().unwrap();
        {
            let conn = repo.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO projects (id, data, is_deleted, directory_normalized, updated_at)
                 VALUES ('bad', '{not json', 0, '/x', '2024-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        }
        assert!(matches!(repo.get("bad"), Err(RepositoryError::Corrupt(_))));
    }

    #[test]
    fn fresh_db_is_at_current_schema_version() {
        let repo = SqliteRepository::in_memory().unwrap();
        let conn = repo.conn.lock().unwrap();
        let v: i64 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, CURRENT_SCHEMA_VERSION);
        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "memory"); // in-memory DBs report "memory"; a file DB reports "wal"
        let app: String = conn
            .query_row("SELECT value FROM meta WHERE key = 'app'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(app, "project-indexer");
    }

    #[test]
    fn refuses_a_newer_database() {
        let repo = SqliteRepository::in_memory().unwrap();
        {
            let conn = repo.conn.lock().unwrap();
            conn.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION + 1)
                .unwrap();
        }
        // Re-open the same connection path is awkward for :memory:; instead test the
        // guard via a helper that takes an existing Connection:
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION + 1).unwrap();
        assert!(matches!(
            SqliteRepository::from_connection(conn),
            Err(RepositoryError::Backend(_))
        ));
    }
}
```

- [ ] **Step 3: Implement `SqliteRepository`**

Above the tests:

```rust
use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension};

use crate::domain::normalize::normalize_directory;
use crate::domain::Project;
use crate::error::RepositoryError;
use crate::ports::{ProjectReader, ProjectRepository};

/// The schema version this binary understands. `open` migrates up to this and
/// refuses any database already past it.
pub const CURRENT_SCHEMA_VERSION: i64 = 1;

pub struct SqliteRepository {
    conn: Mutex<Connection>,
}

impl SqliteRepository {
    pub fn open(path: &Path) -> Result<Self, RepositoryError> {
        let conn = Connection::open(path).map_err(be)?;
        Self::from_connection(conn)
    }

    pub fn in_memory() -> Result<Self, RepositoryError> {
        Self::from_connection(Connection::open_in_memory().map_err(be)?)
    }

    /// Shared setup: pragmas, version-skew guard, migrations.
    pub fn from_connection(conn: Connection) -> Result<Self, RepositoryError> {
        conn.pragma_update(None, "journal_mode", "WAL").map_err(be)?;
        conn.pragma_update(None, "busy_timeout", 5000).map_err(be)?;
        conn.pragma_update(None, "foreign_keys", "ON").map_err(be)?;

        let version: i64 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(be)?;
        if version > CURRENT_SCHEMA_VERSION {
            return Err(RepositoryError::Backend(
                "database is from a newer version of Project Indexer".into(),
            ));
        }
        run_migrations(&conn, version)?;

        Ok(Self { conn: Mutex::new(conn) })
    }
}

fn run_migrations(conn: &Connection, from: i64) -> Result<(), RepositoryError> {
    if from < 1 {
        conn.execute_batch(
            "BEGIN;
             CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO meta (key, value) VALUES
               ('app', 'project-indexer'), ('schema_version', '1');
             CREATE TABLE projects (
               id                   TEXT PRIMARY KEY,
               data                 TEXT NOT NULL,
               is_deleted           INTEGER NOT NULL,
               directory_normalized TEXT NOT NULL,
               updated_at           TEXT NOT NULL
             );
             CREATE INDEX idx_projects_is_deleted ON projects(is_deleted);
             CREATE INDEX idx_projects_directory_normalized ON projects(directory_normalized);
             CREATE TABLE project_tags (
               project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
               tag        TEXT NOT NULL,
               PRIMARY KEY (project_id, tag)
             );
             CREATE INDEX idx_project_tags_tag ON project_tags(tag);
             COMMIT;",
        )
        .map_err(be)?;
        conn.pragma_update(None, "user_version", 1).map_err(be)?;
    }
    Ok(())
}

impl ProjectReader for SqliteRepository {
    fn get(&self, id: &str) -> Result<Option<Project>, RepositoryError> {
        let conn = self.conn.lock().unwrap();
        let data: Option<String> = conn
            .query_row("SELECT data FROM projects WHERE id = ?1", [id], |r| r.get(0))
            .optional()
            .map_err(be)?;
        data.map(|d| parse(&d)).transpose()
    }

    fn list(&self) -> Result<Vec<Project>, RepositoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT data FROM projects").map_err(be)?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(be)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(parse(&row.map_err(be)?)?);
        }
        Ok(out)
    }

    fn find_by_directory(
        &self,
        normalized_directory: &str,
    ) -> Result<Option<Project>, RepositoryError> {
        let conn = self.conn.lock().unwrap();
        let data: Option<String> = conn
            .query_row(
                "SELECT data FROM projects WHERE directory_normalized = ?1 LIMIT 1",
                [normalized_directory],
                |r| r.get(0),
            )
            .optional()
            .map_err(be)?;
        data.map(|d| parse(&d)).transpose()
    }
}

impl ProjectRepository for SqliteRepository {
    fn save(&self, project: &Project) -> Result<(), RepositoryError> {
        let data = serde_json::to_string(project)
            .map_err(|e| RepositoryError::Backend(format!("serialize: {e}")))?;
        let dir_norm = normalize_directory(&project.directory);

        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(be)?;
        tx.execute(
            "INSERT INTO projects (id, data, is_deleted, directory_normalized, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
               data = excluded.data,
               is_deleted = excluded.is_deleted,
               directory_normalized = excluded.directory_normalized,
               updated_at = excluded.updated_at",
            rusqlite::params![
                project.id,
                data,
                project.is_deleted as i64,
                dir_norm,
                project.updated_at.to_rfc3339(),
            ],
        )
        .map_err(be)?;
        tx.execute("DELETE FROM project_tags WHERE project_id = ?1", [&project.id])
            .map_err(be)?;
        {
            let mut ins = tx
                .prepare("INSERT INTO project_tags (project_id, tag) VALUES (?1, ?2)")
                .map_err(be)?;
            for tag in &project.tags {
                ins.execute(rusqlite::params![project.id, tag]).map_err(be)?;
            }
        }
        tx.commit().map_err(be)?;
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<(), RepositoryError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM projects WHERE id = ?1", [id])
            .map_err(be)?;
        Ok(())
    }
}

fn be(e: rusqlite::Error) -> RepositoryError {
    RepositoryError::Backend(e.to_string())
}

fn parse(data: &str) -> Result<Project, RepositoryError> {
    serde_json::from_str(data).map_err(|e| RepositoryError::Corrupt(e.to_string()))
}
```

`crates/core/src/infra/mod.rs`:

```rust
pub mod sqlite_repository;
pub use sqlite_repository::{SqliteRepository, CURRENT_SCHEMA_VERSION};
```

Add `pub mod infra;` + `pub use infra::{SqliteRepository, CURRENT_SCHEMA_VERSION};` to `lib.rs`.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p indexer-core sqlite`
Expected: all 8 pass. Fix the `fresh_db_is_at_current_schema_version` `journal_mode` assertion if `:memory:` reports something other than `"memory"` on the runner — the point is that `open` succeeds and `user_version`/`meta` are set.

- [ ] **Step 5: Full core test run + fmt + commit**

Run: `cargo test -p indexer-core` (all green), `cargo clippy -p indexer-core` (clean), `cargo fmt --all`.

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(core): ProjectRepository / AppLauncher ports + SqliteRepository

rusqlite (bundled, WAL). Project stored as a JSON blob with promoted
columns; tags mirrored into project_tags. user_version migration runner
with a version-skew guard that refuses a database from a newer binary.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: `ProjectService`

**Files:**
- Create: `crates/core/src/application/mod.rs`, `application/service.rs`, `application/inspection.rs`
- Modify: `crates/core/src/lib.rs`, `crates/core/src/error/project_error.rs` (add `From` impls)

**Interfaces:**
- Consumes: all ports, `DetectorRunner`, domain types, `platform::{check_directory_status, remove_directory}`.
- Produces:
  - `indexer_core::application::ProjectService` with:
    - `new(repo: Arc<dyn ProjectRepository>, launcher: Arc<dyn AppLauncher>, detectors: Arc<DetectorRunner>) -> Self`
    - `create(&self, name: String, directory: String, description: Option<String>, tags: Option<Vec<String>>) -> Result<Project, ProjectError>`
    - `update(&self, id: &str, update: UpdateProject) -> Result<Project, ProjectError>`
    - `get(&self, id: &str) -> Result<Project, ProjectError>`
    - `list(&self, options: SortOptions) -> Result<Vec<Project>, ProjectError>`
    - `list_deleted(&self, options: SortOptions) -> Result<Vec<Project>, ProjectError>`
    - `list_favorites(&self, options: SortOptions) -> Result<Vec<Project>, ProjectError>`
    - `list_missing_directories(&self) -> Result<Vec<String>, ProjectError>`
    - `refresh_trackers(&self, id: &str) -> Result<Project, ProjectError>`
    - `preview_detection(&self, directory: &str) -> Vec<Tracker>`
    - `inspect(&self, id: &str, only: Option<&str>) -> Result<ProjectInspection, ProjectError>`
    - `delete(&self, id: &str) -> Result<(), ProjectError>`
    - `restore(&self, id: &str) -> Result<Project, ProjectError>`
    - `untrack(&self, id: &str) -> Result<(), ProjectError>`
    - `delete_directory(&self, id: &str, delete_metadata: bool) -> Result<(), ProjectError>`
    - `open(&self, id: &str) -> Result<Project, ProjectError>`
    - `open_in_explorer(&self, id: &str) -> Result<Project, ProjectError>`
    - `find_by_directory(&self, directory: &str) -> Result<Option<Project>, ProjectError>`
    - `ensure_project(&self, directory: &str) -> Result<Project, ProjectError>`
  - `indexer_core::application::{ProjectInspection, DetectorResult, DetectorStatus, DirectoryState}`

- [ ] **Step 1: Add error `From` impls**

In `crates/core/src/error/project_error.rs`, after the enum:

```rust
impl From<crate::error::RepositoryError> for ProjectError {
    fn from(e: crate::error::RepositoryError) -> Self {
        ProjectError::Store(e.to_string())
    }
}

impl From<crate::error::LauncherError> for ProjectError {
    fn from(e: crate::error::LauncherError) -> Self {
        ProjectError::OpenFailed(e.0)
    }
}
```

- [ ] **Step 2: Move the inspection DTOs**

Create `crates/core/src/application/inspection.rs` from the current `src-tauri/src/commands/inspect.rs` — everything except `inspect_project` itself:
- `ProjectInspection` (field `directory_status: DirectoryState`, keep the JSON key `directory_status`)
- rename the struct `DirectoryStatusDto` → `DirectoryState` (JSON unaffected — it's the field name that's serialized)
- `DetectorResult`, `DetectorStatus` (unchanged, keep `#[serde(rename_all = "snake_case")]`)
- `results_from(detection: Detection) -> Vec<DetectorResult>` — make it `pub(crate)`
- the `#[cfg(test)] mod tests` (`results_from_maps_every_outcome_variant`) — move it too, fix imports.

Rewrite imports: `crate::detectors::{Detection, DetectorOutcome}`, `crate::domain::{Project, Tracker}`.

- [ ] **Step 3: Write `ProjectService` failing tests**

`crates/core/src/application/service.rs` `#[cfg(test)] mod tests` — a `FakeLauncher` plus flow tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::detectors::DetectorRunner;
    use crate::infra::SqliteRepository;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FakeLauncher {
        available: bool,
        opened: Mutex<Vec<(String, Option<String>)>>,
    }
    impl AppLauncher for FakeLauncher {
        fn open(&self, dir: &str, with: Option<&str>) -> Result<(), crate::error::LauncherError> {
            self.opened.lock().unwrap().push((dir.into(), with.map(str::to_string)));
            Ok(())
        }
        fn is_available(&self, _: &str) -> bool { self.available }
    }

    fn service(launcher: Arc<FakeLauncher>) -> ProjectService {
        ProjectService::new(
            Arc::new(SqliteRepository::in_memory().unwrap()),
            launcher,
            Arc::new(DetectorRunner::default()),
        )
    }

    fn tmpdir(name: &str) -> String {
        let d = std::env::temp_dir().join(format!("pi-svc-{name}"));
        std::fs::create_dir_all(&d).unwrap();
        d.to_string_lossy().into_owned()
    }

    #[test]
    fn create_then_get_and_list() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let p = svc.create("Alpha".into(), tmpdir("a"), None, None).unwrap();
        assert_eq!(svc.get(&p.id).unwrap().name, "Alpha");
        assert_eq!(svc.list(Default::default()).unwrap().len(), 1);
    }

    #[test]
    fn create_rejects_duplicate_name() {
        let svc = service(Arc::new(FakeLauncher::default()));
        svc.create("Dup".into(), tmpdir("dup1"), None, None).unwrap();
        let err = svc.create("dup".into(), tmpdir("dup2"), None, None).unwrap_err();
        assert!(matches!(err, ProjectError::DuplicateName(_)));
    }

    #[test]
    fn create_rejects_duplicate_directory() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let dir = tmpdir("samedir");
        svc.create("One".into(), dir.clone(), None, None).unwrap();
        let err = svc.create("Two".into(), dir, None, None).unwrap_err();
        assert!(matches!(err, ProjectError::DuplicateDirectory(_)));
    }

    #[test]
    fn get_unknown_is_not_found() {
        let svc = service(Arc::new(FakeLauncher::default()));
        assert!(matches!(svc.get("nope"), Err(ProjectError::NotFound(_))));
    }

    #[test]
    fn open_missing_directory_is_deleted_or_moved() {
        let svc = service(Arc::new(FakeLauncher { available: true, ..Default::default() }));
        let dir = tmpdir("open-gone");
        let p = svc.create("Gone".into(), dir.clone(), None, None).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(matches!(svc.open(&p.id), Err(ProjectError::DirectoryDeletedOrMoved(_))));
    }

    #[test]
    fn open_with_missing_app_is_reported() {
        let launcher = Arc::new(FakeLauncher { available: false, ..Default::default() });
        let svc = service(launcher);
        let mut p = svc.create("App".into(), tmpdir("open-app"), None, None).unwrap();
        p.open_with = Some("/nonexistent/editor".into());
        svc.update(&p.id, mk_update_open_with("/nonexistent/editor")).unwrap();
        assert!(matches!(svc.open(&p.id), Err(ProjectError::OpenWithAppMissing(_))));
    }

    #[test]
    fn open_success_marks_opened_and_calls_launcher() {
        let launcher = Arc::new(FakeLauncher { available: true, ..Default::default() });
        let svc = service(launcher.clone());
        let dir = tmpdir("open-ok");
        let p = svc.create("OK".into(), dir.clone(), None, None).unwrap();
        let opened = svc.open(&p.id).unwrap();
        assert!(opened.last_opened_at.is_some());
        assert_eq!(launcher.opened.lock().unwrap().len(), 1);
        assert!(svc.get(&p.id).unwrap().last_opened_at.is_some());
    }

    #[test]
    fn delete_requires_bin() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let p = svc.create("Live".into(), tmpdir("del-live"), None, None).unwrap();
        assert!(matches!(svc.delete(&p.id), Err(ProjectError::ProjectNotInBin(_))));
    }

    #[test]
    fn untrack_then_recreate() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let dir = tmpdir("untrack");
        let p = svc.create("U".into(), dir.clone(), None, None).unwrap();
        svc.untrack(&p.id).unwrap();
        assert!(svc.get(&p.id).is_err());
        svc.create("U again".into(), dir, None, None).unwrap(); // dir is free again
    }

    #[test]
    fn delete_directory_soft_keeps_record_in_bin() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let dir = tmpdir("deldir-soft");
        let p = svc.create("S".into(), dir.clone(), None, None).unwrap();
        svc.delete_directory(&p.id, false).unwrap();
        assert!(svc.get(&p.id).unwrap().is_deleted);
        assert!(!std::path::Path::new(&dir).exists());
    }

    #[test]
    fn delete_directory_hard_purges() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let dir = tmpdir("deldir-hard");
        let p = svc.create("H".into(), dir, None, None).unwrap();
        svc.delete_directory(&p.id, true).unwrap();
        assert!(svc.get(&p.id).unwrap_err().to_string().contains("not found") || svc.get(&p.id).unwrap().id != p.id);
    }

    #[test]
    fn refresh_all_or_nothing_leaves_stored_trackers_on_detector_failure() {
        // Uses a runner with one always-failing detector alongside the defaults.
        // Build the service with a custom DetectorRunner.
        use crate::detectors::{Detector, DetectorRunner};
        use crate::error::DetectorError;
        struct Boom;
        impl Detector for Boom {
            fn kind(&self) -> &'static str { "boom" }
            fn detect(&self, _: &std::path::Path) -> Result<Option<Tracker>, DetectorError> {
                Err(DetectorError::Other("boom".into()))
            }
        }
        let repo = Arc::new(SqliteRepository::in_memory().unwrap());
        let svc = ProjectService::new(
            repo,
            Arc::new(FakeLauncher::default()),
            Arc::new(DetectorRunner::new(vec![Box::new(Boom)])),
        );
        let p = svc.create("R".into(), tmpdir("refresh"), None, None).unwrap();
        let before = svc.get(&p.id).unwrap().trackers.len();
        assert!(svc.refresh_trackers(&p.id).is_err());
        assert_eq!(svc.get(&p.id).unwrap().trackers.len(), before);
    }

    #[test]
    fn inspect_reports_bad_directory_without_erroring() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let dir = tmpdir("inspect-gone");
        let p = svc.create("I".into(), dir.clone(), None, None).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let ins = svc.inspect(&p.id, None).unwrap();
        assert!(!ins.directory_status.ok);
        assert!(ins.results.is_empty());
    }

    #[test]
    fn restore_clears_deleted() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let dir = tmpdir("restore");
        let p = svc.create("Rst".into(), dir.clone(), None, None).unwrap();
        svc.delete_directory(&p.id, false).unwrap();
        assert!(svc.get(&p.id).unwrap().is_deleted);
        let restored = svc.restore(&p.id).unwrap();
        assert!(!restored.is_deleted);
    }

    #[test]
    fn ensure_project_is_idempotent() {
        let svc = service(Arc::new(FakeLauncher::default()));
        let dir = tmpdir("ensure");
        let a = svc.ensure_project(&dir).unwrap();
        let b = svc.ensure_project(&dir).unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(svc.list(Default::default()).unwrap().len(), 1);
    }

    fn mk_update_open_with(path: &str) -> UpdateProject {
        serde_json::from_value(serde_json::json!({ "open_with": path })).unwrap()
    }
}
```

- [ ] **Step 4: Implement `ProjectService`**

Port each body from `src-tauri/src/commands/projects.rs` and `inspect.rs`, replacing:
- `let store = ProjectStore::new(&app)?;` → use `self.repo`
- `store.get_project(&id)?.ok_or_else(|| ProjectError::NotFound(id.clone()))?` → `self.repo.get(id)?.ok_or_else(|| ProjectError::NotFound(id.to_string()))?`
- `store.get_all_projects()?` → `self.repo.list()?.into_iter().filter(|p| !p.is_deleted).collect::<Vec<_>>()`
- `store.all_projects()?` → `self.repo.list()?`
- `store.save_project(&p)?` → `self.repo.save(&p)?`
- `store.delete_project(&id)?` → `self.repo.delete(id)?`
- `detectors.detect_project(...)` → `self.detectors.detect_project(...)`
- `crate::commands::system::open_in_app(...)` → `self.launcher.open(...)?`
- `open_with_app_available(cmd)` → `self.launcher.is_available(cmd)`
- `check_directory_health` is `Project::check_directory_health` (moved to `core::domain::project` in Task 2) — call it directly.
- `remove_directory` → `crate::platform::remove_directory`
- `check_directory_status` / `DirectoryStatus` → `crate::platform::{check_directory_status, DirectoryStatus}`

```rust
use std::path::Path;
use std::sync::Arc;

use crate::application::inspection::{results_from, DirectoryState, ProjectInspection};
use crate::detectors::DetectorRunner;
use crate::domain::naming::suggest_project_name;
use crate::domain::sorting::{filter_deleted, filter_favorites, sort_projects, SortOptions};
use crate::domain::{Project, Tracker, UpdateProject};
use crate::error::ProjectError;
use crate::platform::{check_directory_status, remove_directory, DirectoryStatus};
use crate::ports::{AppLauncher, ProjectRepository};

pub struct ProjectService {
    repo: Arc<dyn ProjectRepository>,
    launcher: Arc<dyn AppLauncher>,
    detectors: Arc<DetectorRunner>,
}

impl ProjectService {
    pub fn new(
        repo: Arc<dyn ProjectRepository>,
        launcher: Arc<dyn AppLauncher>,
        detectors: Arc<DetectorRunner>,
    ) -> Self {
        Self { repo, launcher, detectors }
    }

    fn load(&self, id: &str) -> Result<Project, ProjectError> {
        self.repo.get(id)?.ok_or_else(|| ProjectError::NotFound(id.to_string()))
    }

    pub fn create(
        &self,
        name: String,
        directory: String,
        description: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<Project, ProjectError> {
        let existing = self.repo.list()?;
        let existing_active: Vec<Project> =
            existing.into_iter().filter(|p| !p.is_deleted).collect();
        Project::check_for_duplicate_name_or_dir(&name, &directory, &existing_active)?;

        let mut project = Project::new(name, directory, description, tags)?;
        let detection = self.detectors.detect_project(Path::new(&project.directory));
        project.trackers = detection.trackers();
        for error in detection.errors() {
            eprintln!("Detector error for '{}': {}", project.directory, error);
        }
        self.repo.save(&project)?;
        Ok(project)
    }

    pub fn update(&self, id: &str, update: UpdateProject) -> Result<Project, ProjectError> {
        let mut project = self.load(id)?;
        project.update(update)?;
        self.repo.save(&project)?;
        Ok(project)
    }

    pub fn get(&self, id: &str) -> Result<Project, ProjectError> {
        self.load(id)
    }

    pub fn list(&self, options: SortOptions) -> Result<Vec<Project>, ProjectError> {
        let mut projects: Vec<Project> =
            self.repo.list()?.into_iter().filter(|p| !p.is_deleted).collect();
        sort_projects(&mut projects, options);
        Ok(projects)
    }

    pub fn list_deleted(&self, options: SortOptions) -> Result<Vec<Project>, ProjectError> {
        Ok(filter_deleted(&self.repo.list()?, options))
    }

    pub fn list_favorites(&self, options: SortOptions) -> Result<Vec<Project>, ProjectError> {
        let active: Vec<Project> =
            self.repo.list()?.into_iter().filter(|p| !p.is_deleted).collect();
        Ok(filter_favorites(&active, options))
    }

    pub fn list_missing_directories(&self) -> Result<Vec<String>, ProjectError> {
        Ok(self
            .repo
            .list()?
            .into_iter()
            .filter(|p| !p.is_deleted)
            .filter(|p| {
                matches!(
                    check_directory_status(&p.directory),
                    DirectoryStatus::DoesNotExist | DirectoryStatus::NotADirectory
                )
            })
            .map(|p| p.id)
            .collect())
    }

    pub fn refresh_trackers(&self, id: &str) -> Result<Project, ProjectError> {
        let mut project = self.load(id)?;
        Project::check_directory_health(&project.directory)?;
        project.trackers = self
            .detectors
            .detect_project(Path::new(&project.directory))
            .into_result()
            .map_err(|e| ProjectError::Detection(e.to_string()))?;
        self.repo.save(&project)?;
        Ok(project)
    }

    pub fn preview_detection(&self, directory: &str) -> Vec<Tracker> {
        let detection = self.detectors.detect_project(Path::new(directory));
        for error in detection.errors() {
            eprintln!("Detector error previewing '{directory}': {error}");
        }
        detection.trackers()
    }

    pub fn inspect(
        &self,
        id: &str,
        only: Option<&str>,
    ) -> Result<ProjectInspection, ProjectError> {
        let project = self.load(id)?;
        let (directory_status, results) =
            match Project::check_directory_health(&project.directory) {
                Ok(()) => {
                    let detection =
                        self.detectors.inspect(Path::new(&project.directory), only);
                    (DirectoryState { ok: true, message: None }, results_from(detection))
                }
                Err(error) => (
                    DirectoryState { ok: false, message: Some(error.to_string()) },
                    Vec::new(),
                ),
            };
        Ok(ProjectInspection { project, directory_status, results })
    }

    pub fn delete(&self, id: &str) -> Result<(), ProjectError> {
        let project = self.load(id)?;
        if !project.is_deleted {
            return Err(ProjectError::ProjectNotInBin(id.to_string()));
        }
        self.repo.delete(id)?;
        Ok(())
    }

    pub fn restore(&self, id: &str) -> Result<Project, ProjectError> {
        let mut project = self.load(id)?;
        project.restore();
        self.repo.save(&project)?;
        Ok(project)
    }

    pub fn untrack(&self, id: &str) -> Result<(), ProjectError> {
        self.load(id)?;
        self.repo.delete(id)?;
        Ok(())
    }

    pub fn delete_directory(
        &self,
        id: &str,
        delete_metadata: bool,
    ) -> Result<(), ProjectError> {
        let mut project = self.load(id)?;
        remove_directory(&project.directory).map_err(ProjectError::DirectoryInaccessible)?;
        if delete_metadata {
            self.repo.delete(id)?;
        } else {
            project.mark_deleted();
            self.repo.save(&project)?;
        }
        Ok(())
    }

    pub fn open(&self, id: &str) -> Result<Project, ProjectError> {
        let project = self.load(id)?;
        Project::check_directory_health(&project.directory)?;
        let open_with = project
            .open_with
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if let Some(command) = &open_with {
            if !self.launcher.is_available(command) {
                return Err(ProjectError::OpenWithAppMissing(command.clone()));
            }
        }
        self.open_and_mark(project, open_with.as_deref())
    }

    pub fn open_in_explorer(&self, id: &str) -> Result<Project, ProjectError> {
        let project = self.load(id)?;
        Project::check_directory_health(&project.directory)?;
        self.open_and_mark(project, None)
    }

    fn open_and_mark(
        &self,
        mut project: Project,
        open_with: Option<&str>,
    ) -> Result<Project, ProjectError> {
        self.launcher.open(&project.directory, open_with)?;
        project.mark_as_opened_recently();
        self.repo.save(&project)?;
        Ok(project)
    }

    pub fn find_by_directory(&self, directory: &str) -> Result<Option<Project>, ProjectError> {
        let normalized = crate::domain::normalize::normalize_directory(directory);
        Ok(self.repo.find_by_directory(&normalized)?)
    }

    pub fn ensure_project(&self, directory: &str) -> Result<Project, ProjectError> {
        if let Some(existing) = self.find_by_directory(directory)? {
            return Ok(existing);
        }
        let name = suggest_project_name(&[], directory)
            .unwrap_or_else(|| "project".to_string());
        self.create(name, directory.to_string(), None, None)
    }
}
```

> **Note on `check_directory_health` visibility:** it's currently `pub` on `Project`. Keep it `pub`. `validate_directory` stays private.

> **Note on `Project::check_for_duplicate_name_or_dir`:** takes `&[Project]`. The current command passes non-deleted projects — preserved above.

- [ ] **Step 5: Write `application/mod.rs` and update `lib.rs`**

```rust
// crates/core/src/application/mod.rs
pub mod inspection;
pub mod service;

pub use inspection::{DetectorResult, DetectorStatus, DirectoryState, ProjectInspection};
pub use service::ProjectService;
```

`lib.rs`: add `pub use application::{ProjectInspection, ProjectService};`

- [ ] **Step 6: Test, clippy, fmt, commit**

Run: `cargo test -p indexer-core` — every flow test green.
Run: `cargo clippy -p indexer-core` — clean.
Run: `cargo fmt --all`.

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(core): ProjectService — orchestration lifted out of the tauri commands

One method per current command, logic unchanged: best-effort create,
all-or-nothing refresh, bin-only delete, open-then-mark. Adds
find_by_directory / ensure_project for the future CLI. Inspection DTOs
move to application::inspection. Tested against in-memory SQLite + a fake
launcher.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: `OpenerLauncher` adapter in `src-tauri`

**Files:**
- Create: `src-tauri/src/adapters/mod.rs`, `src-tauri/src/adapters/opener_launcher.rs`
- Modify: `src-tauri/src/lib.rs` (declare `mod adapters;`), `src-tauri/src/commands/system.rs` (delete `open_in_app`), `src-tauri/src/commands/projects.rs` (drop the `open_in_app` import — temporary until Task 7)

**Interfaces:**
- Consumes: `indexer_core::ports::AppLauncher`, `indexer_core::error::LauncherError`, `indexer_core::platform::open_with_app_available`.
- Produces: `crate::adapters::OpenerLauncher` (a unit struct implementing `AppLauncher`).

- [ ] **Step 1: Create the adapter**

`src-tauri/src/adapters/opener_launcher.rs` — move the body of `src-tauri/src/commands/system.rs::open_in_app` in, wrapped:

```rust
use indexer_core::error::LauncherError;
use indexer_core::platform::open_with_app_available;
use indexer_core::ports::AppLauncher;

pub struct OpenerLauncher;

impl AppLauncher for OpenerLauncher {
    fn open(&self, directory: &str, open_with: Option<&str>) -> Result<(), LauncherError> {
        open_in_app(directory, open_with).map_err(LauncherError)
    }

    fn is_available(&self, open_with: &str) -> bool {
        open_with_app_available(open_with)
    }
}

/// (moved verbatim from commands/system.rs)
fn open_in_app(directory: &str, open_with: Option<&str>) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        // NOTE: the Linux path calls into indexer_core::platform for .desktop
        // command-line splitting. If `open_in_app` used `linux_impl::open_with_command`
        // directly, expose that as `indexer_core::platform::app_discovery::open_with_command`
        // (pub) in Task 3's move and call it here.
        if let Some(command) = open_with.map(str::trim).filter(|c| !c.is_empty()) {
            return indexer_core::platform::app_discovery::open_with_command(directory, command);
        }
    }

    #[cfg(windows)]
    {
        if let Some(app) = open_with.map(str::trim).filter(|c| !c.is_empty()) {
            let looks_like_path =
                std::path::Path::new(app).is_absolute() || app.contains(['\\', '/']);
            if looks_like_path {
                use std::os::windows::process::CommandExt;
                const DETACHED_PROCESS: u32 = 0x0000_0008;
                const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                return std::process::Command::new(app)
                    .arg(directory)
                    .env_remove("ELECTRON_RUN_AS_NODE")
                    .env_remove("ELECTRON_NO_ATTACH_CONSOLE")
                    .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW)
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| e.to_string());
            }
        }
    }

    tauri_plugin_opener::open_path(directory, open_with).map_err(|e| e.to_string())
}
```

> **Task 3 addendum:** when moving `linux_impl`, mark `open_with_command` and `split_command` `pub` and re-export `open_with_command` from `platform::app_discovery` so this adapter can call it. Everything else stays private.

`src-tauri/src/adapters/mod.rs`:

```rust
pub mod opener_launcher;
pub use opener_launcher::OpenerLauncher;
```

- [ ] **Step 2: Delete the old `open_in_app`**

Remove `open_in_app` (and any now-unused helpers it alone used) from `src-tauri/src/commands/system.rs`. That file is now just the `list_installed_apps` command.

`src-tauri/src/lib.rs`: add `mod adapters;`.

- [ ] **Step 3: Keep it compiling (temporary bridge)**

`src-tauri/src/commands/projects.rs` still calls `crate::commands::system::open_in_app` in `open_directory_and_mark_opened`. Task 7 deletes that function. For now, change that one call to `crate::adapters::OpenerLauncher.open(&project.directory, open_with).map_err(|e| ProjectError::OpenFailed(e.to_string()))` so the tree builds.

- [ ] **Step 4: Build + smoke-test**

Kill dev app. `cargo build -p project-indexer`. `npm run tauri dev` → set a project's "open with" to a real editor, click Open, confirm it launches; on Windows confirm VS Code opens a folder (the `ELECTRON_RUN_AS_NODE` scrub). Close. `git checkout -- pnpm-lock.yaml`.

- [ ] **Step 5: fmt + commit**

```bash
cargo fmt --all
git add -A
git commit -m "$(cat <<'EOF'
refactor(tauri): OpenerLauncher adapter implements AppLauncher

The open_in_app logic (Windows env-scrub, Linux .desktop exec, opener-plugin
fallback) moves behind the core port. commands/system.rs is now just the
installed-apps query.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Thin commands + `lib.rs` wiring

**Files:**
- Modify: `src-tauri/src/commands/projects.rs`, `src-tauri/src/commands/inspect.rs`, `src-tauri/src/lib.rs`
- Delete: `src-tauri/src/store/` (whole dir), `src-tauri/Cargo.toml` line `tauri-plugin-store`

**Interfaces:**
- Consumes: `indexer_core::application::ProjectService`, all the DTOs.
- Produces: the same Tauri command surface as today + `suggest_project_name`. Managed state: `Arc<ProjectService>`.

- [ ] **Step 1: Rewrite `commands/projects.rs`**

Replace the whole file with pass-throughs. Every command takes `service: State<'_, Arc<ProjectService>>` instead of `app: AppHandle` (+ `detectors: State<...>` where present). Example shape:

```rust
use std::sync::Arc;
use tauri::State;

use indexer_core::application::ProjectService;
use indexer_core::domain::sorting::SortOptions;
use indexer_core::domain::{Project, Tracker, UpdateProject};
use indexer_core::error::ProjectError;

#[tauri::command]
pub fn create_project(
    service: State<'_, Arc<ProjectService>>,
    name: String,
    directory: String,
    description: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<Project, ProjectError> {
    service.create(name, directory, description, tags)
}

#[tauri::command]
pub fn update_project(
    service: State<'_, Arc<ProjectService>>,
    id: String,
    update: UpdateProject,
) -> Result<Project, ProjectError> {
    service.update(&id, update)
}

#[tauri::command]
pub fn get_project(service: State<'_, Arc<ProjectService>>, id: String) -> Result<Project, ProjectError> {
    service.get(&id)
}

#[tauri::command]
pub fn get_all_projects(
    service: State<'_, Arc<ProjectService>>,
    options: Option<SortOptions>,
) -> Result<Vec<Project>, ProjectError> {
    service.list(options.unwrap_or_default())
}

#[tauri::command]
pub fn get_deleted_projects(
    service: State<'_, Arc<ProjectService>>,
    options: Option<SortOptions>,
) -> Result<Vec<Project>, ProjectError> {
    service.list_deleted(options.unwrap_or_default())
}

#[tauri::command]
pub fn get_favorite_projects(
    service: State<'_, Arc<ProjectService>>,
    options: Option<SortOptions>,
) -> Result<Vec<Project>, ProjectError> {
    service.list_favorites(options.unwrap_or_default())
}

#[tauri::command]
pub fn list_missing_directories(
    service: State<'_, Arc<ProjectService>>,
) -> Result<Vec<String>, ProjectError> {
    service.list_missing_directories()
}

#[tauri::command]
pub fn refresh_project_trackers(
    service: State<'_, Arc<ProjectService>>,
    id: String,
) -> Result<Project, ProjectError> {
    service.refresh_trackers(&id)
}

#[tauri::command]
pub fn detect_project_trackers(
    service: State<'_, Arc<ProjectService>>,
    directory: String,
) -> Vec<Tracker> {
    service.preview_detection(&directory)
}

#[tauri::command]
pub fn suggest_project_name(
    service: State<'_, Arc<ProjectService>>,
    directory: String,
) -> Option<String> {
    let trackers = service.preview_detection(&directory);
    indexer_core::domain::naming::suggest_project_name(&trackers, &directory)
}

#[tauri::command]
pub fn delete_project(service: State<'_, Arc<ProjectService>>, id: String) -> Result<(), ProjectError> {
    service.delete(&id)
}

#[tauri::command]
pub fn untrack_project(service: State<'_, Arc<ProjectService>>, id: String) -> Result<(), ProjectError> {
    service.untrack(&id)
}

#[tauri::command]
pub fn delete_project_directory(
    service: State<'_, Arc<ProjectService>>,
    id: String,
    delete_metadata: bool,
) -> Result<(), ProjectError> {
    service.delete_directory(&id, delete_metadata)
}

#[tauri::command]
pub fn restore_project(
    service: State<'_, Arc<ProjectService>>,
    id: String,
) -> Result<Project, ProjectError> {
    service.restore(&id)
}

#[tauri::command]
pub fn open_project(service: State<'_, Arc<ProjectService>>, id: String) -> Result<Project, ProjectError> {
    service.open(&id)
}

#[tauri::command]
pub fn open_project_in_explorer(
    service: State<'_, Arc<ProjectService>>,
    id: String,
) -> Result<Project, ProjectError> {
    service.open_in_explorer(&id)
}
```

> **`restore`:** `ProjectService::restore` is defined in Task 5. If tasks are done out of order and it's missing, it's load → `project.restore()` → `repo.save` → return, mirroring `untrack`.

- [ ] **Step 2: Rewrite `commands/inspect.rs`**

```rust
use std::sync::Arc;
use tauri::State;

use indexer_core::application::{ProjectInspection, ProjectService};
use indexer_core::error::ProjectError;

#[tauri::command]
pub fn inspect_project(
    service: State<'_, Arc<ProjectService>>,
    id: String,
    only: Option<String>,
) -> Result<ProjectInspection, ProjectError> {
    service.inspect(&id, only.as_deref())
}
```

Delete the DTO definitions and `results_from` (now in core). Delete the `#[cfg(test)]` block (moved to core).

- [ ] **Step 3: Wire `lib.rs`**

```rust
mod adapters;
mod commands;

use std::sync::Arc;
use tauri::Manager;

use indexer_core::application::ProjectService;
use indexer_core::detectors::DetectorRunner;
use indexer_core::infra::SqliteRepository;

use crate::adapters::OpenerLauncher;

// (keep disable_dmabuf_renderer_on_nvidia unchanged)

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    disable_dmabuf_renderer_on_nvidia();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&dir)?;
            let repo = SqliteRepository::open(&dir.join("projects.db"))
                .map_err(|e| format!("failed to open project database: {e}"))?;
            let service = ProjectService::new(
                Arc::new(repo),
                Arc::new(OpenerLauncher),
                Arc::new(DetectorRunner::default()),
            );
            app.manage(Arc::new(service));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::projects::create_project,
            commands::projects::update_project,
            commands::projects::get_project,
            commands::projects::get_all_projects,
            commands::projects::list_missing_directories,
            commands::projects::get_deleted_projects,
            commands::projects::get_favorite_projects,
            commands::projects::delete_project,
            commands::projects::delete_project_directory,
            commands::projects::untrack_project,
            commands::projects::restore_project,
            commands::projects::open_project,
            commands::projects::open_project_in_explorer,
            commands::projects::refresh_project_trackers,
            commands::projects::detect_project_trackers,
            commands::projects::suggest_project_name,
            commands::system::list_installed_apps,
            commands::inspect::inspect_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Removed: `pub mod migrations; pub mod models; pub mod store; pub mod errors; pub mod detectors; pub mod utils;` (all gone), `.plugin(tauri_plugin_store::Builder::new().build())`, `.manage(detectors::DetectorRunner::default())`, the `.on_window_event(... ProjectStore::flush ...)` block.

- [ ] **Step 4: Delete the store module and dep**

```bash
git rm -r src-tauri/src/store
```

`src-tauri/Cargo.toml`: remove `tauri-plugin-store = "2"`.

- [ ] **Step 5: Full build + manual regression pass**

Kill dev app. `cargo build --workspace`. `cargo test --workspace`.
`npm run tauri dev` and exercise **every** flow:
- create a project (real dir), see trackers detected
- Browse in the create form → name pre-fills
- edit name / tags / description / favorite / notes / client / open-with
- favorite toggle from the card menu; open the Favorites modal
- open a project (with and without an "open with" app)
- open in explorer
- delete directory → soft (bin) and hard
- restore from bin
- permanently delete from bin
- untrack
- per-project detail route `/project/[id]` — tabs, status strip, re-detect, Edit overlay, Refresh
- the "directory gone" bin icon (delete a project's folder outside the app, relaunch)
- sort controls

Confirm `projects.db` (not `projects.json`) is created in the app config dir. Close. `git checkout -- pnpm-lock.yaml`.

- [ ] **Step 6: fmt + commit**

```bash
cargo fmt --all
git add -A
git commit -m "$(cat <<'EOF'
refactor(tauri): commands are thin pass-throughs over ProjectService

Every #[tauri::command] drops AppHandle and calls the managed
Arc<ProjectService>. tauri-plugin-store, the flush-on-close hook, the
separate DetectorRunner state, and src-tauri/src/store are removed;
persistence is SQLite via core. New command: suggest_project_name.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Frontend name suggestion via command

**Files:**
- Modify: `src/lib/api/projects.ts`, `src/lib/components/CreateProjectForm.svelte`

**Interfaces:**
- Consumes: the `suggest_project_name` command from Task 7.

- [ ] **Step 1: Add the API wrapper**

`src/lib/api/projects.ts`, near `detectProjectTrackers`:

```ts
// Backend-computed project-name suggestion for a picked directory: the git
// remote's repo name if the dir is a repo with a remote, else the folder name.
// null when neither is available. Best-effort — only throws on IPC failure.
export async function suggestProjectName(directory: string): Promise<string | null> {
  try {
    return await invoke<string | null>("suggest_project_name", { directory });
  } catch (err) {
    throw toError(err);
  }
}
```

- [ ] **Step 2: Rewrite `handleDirectoryPicked` in `CreateProjectForm.svelte`**

Delete `isGitTracker`, `repoNameFromUrl`, `folderNameFromDirectory`, `suggestProjectName` (the local functions), and the now-unused imports (`detectProjectTrackers`, `GitInfo`, `Tracker`). Replace the handler body:

```ts
import { createProject, suggestProjectName } from "$lib/api/projects";
// ...
async function handleDirectoryPicked(dir: string) {
  if (name.trim().length > 0) return;
  try {
    const suggested = await suggestProjectName(dir);
    if (suggested) name = suggested;
  } catch {
    // No suggestion — the user types a name manually.
  }
}
```

- [ ] **Step 3: Type-check + manual test**

Run: `npm run check`
Expected: clean bar the 8 known `EditProjectForm` warnings.

Run: `npm run tauri dev` → open the create form, Browse to a git repo directory → name fills with the repo name; Browse to a plain directory → name fills with the folder name; type a name first, then Browse → name is not overwritten. Close. `git checkout -- pnpm-lock.yaml`.

- [ ] **Step 4: Run frontend unit tests**

Run: `npm test`
Expected: the 14 `trackers.test.ts` cases pass (untouched).

- [ ] **Step 5: Commit**

```bash
git add src/
git commit -m "$(cat <<'EOF'
refactor(ui): name suggestion via suggest_project_name command

Drops the inline repo-name / folder-name parsing from CreateProjectForm;
the logic now lives in core::domain::naming with real unit coverage.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Documentation

**Files:**
- Modify: `docs/architecture.md`, `docs/knowledgebase.md`, `docs/checklist.md`, `docs/accomplishments.md`, `docs/KNOWN-ISSUES.md`

- [ ] **Step 1: `architecture.md`**

- Replace the "The system today" ASCII diagram with the workspace / crate-boundary picture (three crates, dependency arrows, `core` can't see `tauri`).
- Remove the paragraph "What's *not* here yet: a service layer …" — it exists now.
- Add invariants (renumber as needed):
  - **`core` never depends on `tauri`** — compiler-enforced; `cargo tree -p indexer-core` has no tauri.
  - **All persistence goes through `ProjectRepository`** — the service is the only caller.
  - **The binary owns forward migration; it never reads a newer DB** — `SqliteRepository::open` runs `user_version` steps up to `CURRENT_SCHEMA_VERSION` and refuses anything higher.
- Add **Recorded decisions**:
  - *SQLite as a document store* — `Project` in a `data` blob; `is_deleted`/`directory_normalized`/`updated_at` promoted; `tags` mirrored to `project_tags`; `trackers` blob-only because it's a per-variant sum type (normalizing it would break "add a detector = zero persistence change"). Guarded by the `SqliteRepository` round-trip + tag tests.
  - *Tauri-free `core` crate* — the GUI is one frontend; a CLL/others attach via the same crate + the same `projects.db`.
  - *No `serde_json::Value` migration layer* — deleted with the JSON store; there was no production data. Schema evolution = `user_version` only.
- Move "Refresh is all-or-nothing" recorded decision's pointer from a command doc-comment to `ProjectService::refresh_trackers`; test is now `refresh_all_or_nothing_leaves_stored_trackers_on_detector_failure` in `application/service.rs`.
- In the quality backlog: mark **"Extract detection orchestration"** and **"Command-layer integration tests"** as done. Mark **"Platform provider traits"** partially done (`AppLauncher` exists; `InstalledAppProvider` still a plain fn). **Keep "Migration fixtures"** and note it's now load-bearing (auto-update makes cross-version migration real) — reference the spec's "App updates" section.
- Add a short "Cross-app & updates" pointer to the spec for the devmon contract and the updater fast-follow.

- [ ] **Step 2: `knowledgebase.md`**

Rewrite the "Backend architecture" section around the workspace:
- `crates/core` (`indexer-core`) — `domain` / `ports` / `application` / `detectors` / `platform` / `infra` / `error`. No tauri.
- `src-tauri` — `adapters/opener_launcher.rs`, thin `commands/`, `lib.rs` setup wiring.
- Persistence: `SqliteRepository` at `app_config_dir/projects.db`, WAL, `Project` as a JSON blob + `project_tags`, `user_version` schema runner. No more `tauri-plugin-store`, no autosave/flush.
- `suggest_project_name` command; `core::domain::naming`.
- Note `crates/cli` is a stub for Spec 2.

- [ ] **Step 3: `checklist.md`**

- New "## Frontend-agnostic core" section, all items checked: workspace, `indexer-core`, ports, `ProjectService`, `SqliteRepository`, thin commands, name suggestion in core.
- Update Rust test counts (`Gitector`, `UnrealDetector` unchanged; add `SqliteRepository` (8), `ProjectService` (~16), `naming` (8)).
- Under "Open": Spec 2 (observer CLI), and the updater / release-notification / CLI-install / signing-CI fast-follows.

- [ ] **Step 4: `accomplishments.md`**

New dated entry `## 2026-09-DD — Frontend-agnostic core` summarizing: the workspace split, SQLite swap, `ProjectService`, ports, the tests, zero user-visible change. Note the deleted `store/` + `migrations/` and the dropped `tauri-plugin-store` dep.

- [ ] **Step 5: `KNOWN-ISSUES.md`**

- PI-003 (`state_referenced_locally` in `EditProjectForm.svelte`) — line numbers unchanged (that file isn't touched). Leave as-is.
- Add a note under PI-001 if desired that the sort `<select>` etc. is unaffected (no persistence-layer relation).
- Nothing else changes.

- [ ] **Step 6: Verify docs build / links**

Grep the docs for stale references: `tauri-plugin-store`, `ProjectStore`, `src-tauri/src/models`, `src-tauri/src/store`, `migrations::migrate`, `crate::models`, `crate::errors`. Each hit should be either historical (in a dated `accomplishments.md` entry — leave it) or fixed.

- [ ] **Step 7: Commit**

```bash
git add docs/
git commit -m "$(cat <<'EOF'
docs: bring architecture/knowledgebase/checklist current with the core split

Workspace + crate-boundary diagram, new invariants (core is tauri-free,
persistence via ProjectRepository, forward-only migration), recorded
decisions (SQLite document store, no Value migration layer). Retire the
"extract orchestration" / "command-layer tests" backlog items; keep and
promote "Migration fixtures". Register the updater and CLI fast-follows.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

## Final whole-branch review

After Task 9, run the broad review (subagent-driven-development dispatches this automatically). Focus areas:
- `cargo tree -p indexer-core` genuinely tauri-free.
- Every command's JSON contract byte-identical to `main` (diff the `invoke_handler!` list; check arg names — Tauri maps `snake_case` Rust params to `camelCase` JS keys, so `delete_metadata` ↔ `deleteMetadata` must survive).
- `ProjectError` serialization unchanged (still a bare string over IPC).
- No `unwrap()` on the `Mutex` lock that could poison-cascade in a way that matters (acceptable, but confirm).
- The `restore` service method exists and is wired.
- All 72 original tests present (grep `#[test]` counts before/after) plus the new ones.
- `npm run build` succeeds; `npm run check` clean bar the known 8.

## Self-review notes (done while writing this plan)

- **Spec coverage:** every spec section maps to a task — workspace (T1), domain/errors/naming (T2), detectors/platform (T3), ports/SQLite (T4), service (T5), launcher adapter (T6), commands+wiring (T7), frontend (T8), docs (T9). The fast-follows (updater, notifications, CLI install, CI, Spec 2) are explicitly out of scope per the spec.
- **Type consistency:** `SortOptions` / `UpdateProject` / `Project` / `Tracker` names match the spec and the current code. `DirectoryStatusDto` → `DirectoryState` type rename with the JSON field kept as `directory_status` (frontend reads `.directory_status`, verified in `src/routes/project/[id]/+page.svelte`).
- **Gaps found & fixed inline:** (a) `ProjectService::restore` wasn't in the spec's method list — added to T5 (interface, impl, test); (b) the Linux `open_with_command` / `split_command` need to be `pub` when `linux_impl` moves — noted in T3 addendum and T6; (c) `platform/filesystem.rs` is needed by `domain/project.rs` in T2 before T3 runs — T2 Step 5 adds a stub, T3 Step 3 completes it; (d) `restore_project` command stays a thin pass-through like the rest.
- **Placeholder scan:** no "TBD"/"add error handling"/"similar to Task N". Every code step has real code or an exact file→file move with an import-rewrite table.
