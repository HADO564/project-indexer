# Frontend-agnostic core — design

**Date:** 2026-09-02
**Status:** approved, ready for implementation planning

## Goal

Restructure the Rust backend so the GUI is one frontend among several, sitting
on a Tauri-free library crate that holds all domain logic, orchestration, and
persistence. A CLI can then be added later as a second thin frontend with **no
change to the backend**.

This is **Spec 1 of 2**. Spec 2 (the observer CLI) is previewed at the end so
that `core` is shaped correctly now; it is not built here.

## Motivation

- Today the `#[tauri::command]` functions *are* the application layer:
  orchestration (dup-checks, all-or-nothing refresh, open-then-mark-opened),
  persistence, and filesystem work are all interleaved with Tauri's
  `State`/`AppHandle`. Nothing is reachable without a live Tauri runtime.
- The user wants a CLI that shares the GUI's project database. The only way to
  guarantee "add the CLI without reworking the backend" is a crate boundary the
  compiler enforces: `core` cannot `use tauri`.
- The detector subsystem already proves the pattern works (trait + registry +
  runner, held in managed state). This spec applies the same shape to
  persistence and orchestration.
- **There is no production data.** The app has no real users; every current
  `projects.json` is throwaway test data. So there is no migration-from-JSON
  step, no import task, and no need to carry the existing
  `serde_json::Value`-level migration layer (which existed only to upgrade
  pre-existing stored records). A fresh SQLite database on first run is the
  whole story.

`docs/architecture.md` currently argues *against* a full service layer ("the
file is 300 lines, that's premature") and for deferring platform traits until
the macOS work. This spec supersedes those two backlog entries: the CLI
requirement is the concrete trigger they were waiting for.

## Non-goals (YAGNI)

- **The observer CLI itself.** `crates/cli/` is created as a stub only.
- **A separate slim CLI-only binary.** Deferred; the workspace is laid out to
  accept it.
- **Any data migration / JSON import.** No production data exists. First run
  creates an empty `projects.db`; a stray `projects.json` is ignored.
- **The `serde_json::Value` migration layer** (`migrations::migrate`,
  `CURRENT_VERSION`, the `schema_version` stamp). Deleted, not moved. Its only
  purpose was upgrading legacy stored JSON. Structural schema evolution now
  lives in the SQLite `user_version` runner; field additions rely on serde
  `#[serde(default)]` / `Option<T>` as before.
- **Any CLI dispatch inside the GUI binary.** `src-tauri` stays GUI-only — no
  argv sniffing, no `core::cli`, no `clap`.
- **A fully relational SQLite schema.** `Project` is stored as a JSON blob with
  a few promoted columns (see "Persistence"). `Tracker` is a sum type with
  per-variant payloads; a half-normalized schema is worse than a consistent
  document store.
- **Layered error taxonomies.** One public application error (`ProjectError`),
  plus two small port-level errors mapped into it.
- **Any user-visible change to the GUI.** Same windows, same command names, same
  payloads, same behaviour. This is a pure refactor plus a storage swap.
- **`GitInfo.contributors`**, migration-fixture infrastructure, structured
  detection logging — still tracked in `architecture.md`, untouched here.

## Decisions locked during brainstorming

| Area | Decision |
|---|---|
| Structure | Cargo workspace. New `crates/core` (`indexer-core`) library, no `tauri` dependency. `src-tauri` becomes a thin adapter binary. `crates/cli/` is a stub. |
| Frontends | GUI and (future) CLI are separate binaries/packages, each a thin adapter over `core`. Neither contains the other. `project-indexer` is the command name both install. |
| Ports | Two: `ProjectRepository`, `AppLauncher`. `Detector` already exists. Filtering/sorting are service logic, not ports. |
| Persistence | Drop `tauri-plugin-store`. One `SqliteRepository` in `core` (rusqlite, bundled, WAL), used by every frontend. `Project` stored as a JSON blob + promoted columns. |
| Existing data | None (no real users). Fresh `projects.db` on first run; no importer. The `serde_json::Value` migration layer is deleted — schema evolution moves to SQLite `user_version`. |
| Orchestration | One `ProjectService` in `core`, one method per current command, logic lifted verbatim. |
| Errors | `ProjectError` stays the single public app error with its `Display`→string `Serialize`. Ports get small errors (`RepositoryError`, `LauncherError`) mapped in the service. |
| Name inference | `repo_name_from_url` / folder-name fallback move from `CreateProjectForm.svelte` into `core::domain::naming`, exposed as a `suggest_project_name` command. |
| `core` shaping for Spec 2 | `ProjectService` gains `find_by_directory` and `ensure_project(dir)` now; `SqliteRepository` indexes normalized directory. |

## Architecture

### Workspace layout

```
project-indexer/
├── Cargo.toml                      [workspace] members = crates/core, crates/cli, src-tauri
├── crates/
│   ├── core/                       «indexer-core» — library, NO tauri, NO clap
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── domain/             project, tracker, update_project, git, unreal,
│   │       │                       installed_app, normalize, sorting, naming
│   │       ├── ports/              repository, launcher
│   │       ├── application/        service (ProjectService), inspection (DTOs)
│   │       ├── detectors/          detector, runner, registry, git/, unreal/
│   │       ├── platform/           filesystem, installed_apps/{windows,linux}
│   │       ├── infra/              sqlite_repository
│   │       └── error/              project_error, repository, launcher,
│   │                               detector_error, git, unreal
│   └── cli/                        stub bin — prints "not implemented", placeholder Cargo.toml
└── src-tauri/                      GUI binary — adapter over core
    └── src/
        ├── main.rs
        ├── lib.rs                  Builder::setup assembles the service into managed state
        ├── adapters/
        │   └── opener_launcher.rs  impl AppLauncher (tauri-plugin-opener + std::process)
        └── commands/
            ├── projects.rs         ~3-line #[tauri::command] wrappers
            ├── inspect.rs
            └── system.rs
```

Frontend (`src/`, SvelteKit) is untouched except for one command call
(name suggestion).

### Dependency direction (compiler-enforced)

```
src-tauri  ──►  indexer-core
crates/cli ──►  indexer-core        (later)
indexer-core ──►  std, serde, serde_json, chrono, uuid, thiserror, git2, rusqlite
indexer-core ──X──►  tauri, tauri-plugin-*, clap
```

Inside `core`: `application` → `ports` + `domain` + `detectors` + `infra`;
`domain` depends on nothing but std/serde/chrono/uuid; `platform` and `infra`
are leaf modules.

### `core` Cargo.toml (dependencies)

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
thiserror = "2"
git2 = "0.21"
rusqlite = { version = "0.32", features = ["bundled"] }

[target.'cfg(windows)'.dependencies]
winreg = "0.56"     # moved from src-tauri
parselnk = "0.1"    # moved from src-tauri
```

`src-tauri` drops `git2`, `tauri-plugin-store`, `winreg`, `parselnk`; adds
`indexer-core = { path = "../crates/core" }`.

## The `core` crate

### `domain/`

Straight move from `src-tauri/src/models/` + two `utils` files. No logic
change.

- `project.rs` — `Project` + `new` / `update` / `check_directory_health` /
  `mark_*` / duplicate checks / validation. Still calls
  `platform::check_directory_status` (a plain `std::fs` call — allowed in
  `core`; not a Tauri concern). `Utc::now()` and `Uuid::new_v4()` stay inline
  (no Clock port — YAGNI).
- `tracker.rs`, `update_project.rs`, `git.rs` (`GitInfo`), `unreal.rs`
  (`UnrealInfo`), `installed_app.rs`.
- `normalize.rs`, `sorting.rs` — from `utils/`.
- `naming.rs` — **new**, ported from `CreateProjectForm.svelte`:

```rust
/// "https://github.com/user/my-repo.git" / "git@github.com:user/my-repo.git" -> "my-repo"
pub fn repo_name_from_url(url: &str) -> Option<String>;

/// Last path segment of a directory, separators normalized. "D:\Projects\friction" -> "friction"
pub fn folder_name_from_directory(directory: &str) -> Option<String>;

/// Git remote repo name if the project is in git with a remote, else the folder name.
pub fn suggest_project_name(trackers: &[Tracker], directory: &str) -> Option<String>;
```

Unit-tested (SSH and HTTPS remotes, trailing `.git`, trailing separators,
no-remote fallback, empty).

### `ports/`

```rust
// ports/repository.rs
pub trait ProjectRepository: Send + Sync {
    fn get(&self, id: &str) -> Result<Option<Project>, RepositoryError>;
    fn list(&self) -> Result<Vec<Project>, RepositoryError>;      // ALL projects, deleted included, no ordering guarantee
    fn find_by_directory(&self, normalized_directory: &str) -> Result<Option<Project>, RepositoryError>;
    fn save(&self, project: &Project) -> Result<(), RepositoryError>;   // upsert by id
    fn delete(&self, id: &str) -> Result<(), RepositoryError>;          // idempotent — missing id is Ok
}

// ports/launcher.rs
pub trait AppLauncher: Send + Sync {
    fn open(&self, directory: &str, open_with: Option<&str>) -> Result<(), LauncherError>;
    fn is_available(&self, open_with: &str) -> bool;
}
```

`find_by_directory` takes an already-normalized string (the caller uses
`domain::normalize::normalize_directory`). It exists now because Spec 2's
recognizers need it; the GUI's duplicate check can also use it instead of
scanning `list()`.

### `application/`

```rust
// application/service.rs
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
    ) -> Self;
}
```

| Method | Replaces | Behaviour — unchanged from today |
|---|---|---|
| `create(name, directory, description, tags) -> Result<Project>` | `create_project` | normalize dir, dup name/dir check, `Project::new`, **best-effort** detect (persist whatever matched, log errors), save |
| `update(id, UpdateProject) -> Result<Project>` | `update_project` | load-or-`NotFound`, `Project::update`, save |
| `get(id) -> Result<Project>` | `get_project` | load-or-`NotFound` |
| `list(SortOptions) -> Result<Vec<Project>>` | `get_all_projects` | `repo.list()` → drop deleted → `sort_projects` |
| `list_deleted(SortOptions) -> Result<Vec<Project>>` | `get_deleted_projects` | `repo.list()` → `filter_deleted` |
| `list_favorites(SortOptions) -> Result<Vec<Project>>` | `get_favorite_projects` | `repo.list()` → `filter_favorites` |
| `list_missing_directories() -> Result<Vec<String>>` | `list_missing_directories` | `repo.list()` → ids whose dir is `DoesNotExist`/`NotADirectory` |
| `refresh_trackers(id) -> Result<Project>` | `refresh_project_trackers` | load, `check_directory_health`, **all-or-nothing** detect (`Detection::into_result`), save |
| `preview_detection(directory) -> Vec<Tracker>` | `detect_project_trackers` | best-effort, no persistence |
| `inspect(id, only: Option<&str>) -> Result<ProjectInspection>` | `inspect_project` | load-or-`NotFound`, directory health → `DirectoryState` (not an error), live detection, per-detector results |
| `delete(id) -> Result<()>` | `delete_project` | load-or-`NotFound`, must be `is_deleted` else `ProjectNotInBin`, `repo.delete` |
| `untrack(id) -> Result<()>` | `untrack_project` | load-or-`NotFound`, `repo.delete` (any state) |
| `delete_directory(id, delete_metadata: bool) -> Result<()>` | `delete_project_directory` | load, `platform::remove_directory`, then `repo.delete` or `mark_deleted` + save |
| `open(id) -> Result<Project>` | `open_project` | load, `check_directory_health`, `launcher.is_available` else `OpenWithAppMissing`, `launcher.open`, `mark_as_opened_recently`, save |
| `open_in_explorer(id) -> Result<Project>` | `open_project_in_explorer` | load, `check_directory_health`, `launcher.open(dir, None)`, mark opened, save |
| `find_by_directory(directory) -> Result<Option<Project>>` | **new** | normalize, `repo.find_by_directory` |
| `ensure_project(directory) -> Result<Project>` | **new** (for Spec 2) | `find_by_directory` → return it, else `create(suggest_project_name(...), directory, None, None)` |

```rust
// application/inspection.rs  — moved from commands/inspect.rs, one rename
pub struct ProjectInspection { pub project: Project, pub directory_state: DirectoryState, pub results: Vec<DetectorResult> }
pub struct DirectoryState   { pub ok: bool, #[serde(skip_serializing_if = "Option::is_none")] pub message: Option<String> }  // was DirectoryStatusDto
pub struct DetectorResult   { pub kind: String, pub status: DetectorStatus, pub tracker: Option<Tracker>, pub error: Option<String> }
pub enum   DetectorStatus   { Detected, NotDetected, Failed }   // #[serde(rename_all = "snake_case")], unchanged from today
```

The rename `DirectoryStatusDto` → `DirectoryState` removes the name clash with
`platform::DirectoryStatus` flagged in the project-view review.

### `detectors/`

Moved wholesale from `src-tauri/src/detectors/`. Zero code change — already
Tauri-free (`Detector`, `DetectorRunner`, `Detection`, `DetectorOutcome`,
`registry::default_detectors`, `Gitector`, `UnrealDetector`). All detector
tests move with it.

### `platform/`

- `filesystem.rs` — `check_directory_status` / `DirectoryStatus` (from
  `utils/filesystem.rs`), plus `remove_directory` (from `system.rs`).
- `installed_apps/` — `list_installed_apps()` dispatch + `windows.rs` /
  `linux.rs` (Start Menu, registry App Paths, `.desktop` parsing, the
  `Exec=`/command-line splitting and its tests). Pure std + `winreg` +
  `parselnk`. Not Tauri code, so it lives in `core`.
- `open_with_app_available` / `command_exists` / `PATHEXT` handling move here
  too; the `AppLauncher` impl in `src-tauri` calls
  `core::platform::open_with_app_available` for `is_available`.

### `infra/`

```rust
// infra/sqlite_repository.rs
pub struct SqliteRepository { conn: Mutex<rusqlite::Connection> }

impl SqliteRepository {
    /// Opens (creating if absent) the DB at `path`, sets WAL + busy_timeout,
    /// runs schema migrations to the current `user_version`.
    pub fn open(path: &Path) -> Result<Self, RepositoryError>;

    /// In-memory DB for tests.
    pub fn in_memory() -> Result<Self, RepositoryError>;
}

impl ProjectRepository for SqliteRepository { /* ... */ }
```

**Schema (`user_version = 1`):**

```sql
CREATE TABLE projects (
    id                   TEXT PRIMARY KEY,
    data                 TEXT    NOT NULL,   -- full Project as serde JSON
    is_deleted           INTEGER NOT NULL,   -- promoted: list filtering
    directory_normalized TEXT    NOT NULL,   -- promoted: find_by_directory / dup check
    updated_at           TEXT    NOT NULL    -- promoted: sorting
);
CREATE INDEX idx_projects_is_deleted           ON projects(is_deleted);
CREATE INDEX idx_projects_directory_normalized ON projects(directory_normalized);
```

- `save`: `INSERT ... ON CONFLICT(id) DO UPDATE` with `data = to_string(project)`
  and the three promoted columns recomputed.
- `get` / `find_by_directory`: one `SELECT`, `from_str::<Project>(data)`.
- `list`: `SELECT data` over all rows, deserialize each.
- `delete`: `DELETE WHERE id = ?1`, missing row is `Ok`.
- A row whose `data` fails to deserialize → `RepositoryError::Corrupt`
  (fail loud — matches today's `rejects_a_record_missing_its_identity`).
- **Schema evolution:** a numbered runner keyed on `PRAGMA user_version`.
  `v0→v1` creates the table. A future field change that serde
  `#[serde(default)]` / `Option<T>` can't absorb becomes a numbered step that
  rewrites the `data` blobs in a transaction. This is the *only* migration
  mechanism — there is no separate `serde_json::Value` layer.

### `error/`

- `project_error.rs` — `ProjectError` unchanged, including
  `impl serde::Serialize` (Display → string) so the JS side keeps seeing plain
  strings.
- `repository.rs` — **new**:

```rust
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("project store is unavailable: {0}")]
    Backend(String),
    #[error("project store holds a record that can't be read: {0}")]
    Corrupt(String),
}
```

- `launcher.rs` — **new**: `pub struct LauncherError(pub String)` (or a
  one-variant enum), `Display` = the message.
- Service maps `RepositoryError` → `ProjectError::Store(e.to_string())`,
  `LauncherError` → `ProjectError::OpenFailed(e.0)`.
- `detector_error.rs`, `git.rs`, `unreal.rs` — moved unchanged.

## The `src-tauri` adapter

### `adapters/opener_launcher.rs`

```rust
pub struct OpenerLauncher;

impl AppLauncher for OpenerLauncher {
    fn open(&self, directory: &str, open_with: Option<&str>) -> Result<(), LauncherError> {
        // exactly today's commands::system::open_in_app body:
        //  - Linux: core::platform split-command + spawn for a full .desktop command line
        //  - Windows: std::process::Command with ELECTRON_RUN_AS_NODE / _NO_ATTACH_CONSOLE
        //    scrubbed for a concrete exe path; else tauri_plugin_opener::open_path
        //  - otherwise: tauri_plugin_opener::open_path
    }
    fn is_available(&self, open_with: &str) -> bool {
        core::platform::open_with_app_available(open_with)
    }
}
```

This is the one genuine adapter — it's the only place `tauri_plugin_opener` is
still used. The recorded decision in `architecture.md` about the Windows
env-scrub carries over verbatim; only its location changes.

### `commands/`

Every `#[tauri::command]` collapses to the same shape:

```rust
#[tauri::command]
fn create_project(
    service: State<'_, Arc<ProjectService>>,
    name: String, directory: String,
    description: Option<String>, tags: Option<Vec<String>>,
) -> Result<Project, ProjectError> {
    service.create(name, directory, description, tags)
}
```

`AppHandle` disappears from every signature. New command:

```rust
#[tauri::command]
fn suggest_project_name(
    service: State<'_, Arc<ProjectService>>,
    directory: String,
) -> Option<String> {
    let trackers = service.preview_detection(&directory);
    core::domain::naming::suggest_project_name(&trackers, &directory)
}
```

`commands/system.rs::list_installed_apps` → `core::platform::list_installed_apps()`.

### `lib.rs`

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_single_instance::init(|_, _, _| {}))
    .plugin(tauri_plugin_window_state::Builder::new().build())
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_opener::init())
    .setup(|app| {
        let dir = app.path().app_config_dir()?;
        std::fs::create_dir_all(&dir)?;
        let repo = SqliteRepository::open(&dir.join("projects.db"))?;
        let service = ProjectService::new(
            Arc::new(repo),
            Arc::new(OpenerLauncher),
            Arc::new(DetectorRunner::default()),
        );
        app.manage(Arc::new(service));
        Ok(())
    })
    .invoke_handler(tauri::generate_handler![ /* … same list + suggest_project_name */ ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
```

Removed: `tauri-plugin-store` (dep + plugin line), the
`.manage(DetectorRunner::default())` line (folded into the service), the
`on_window_event(CloseRequested)` flush hook (writes are synchronous now), the
whole `src-tauri/src/store/` module, and `src-tauri/src/migrations/` (deleted,
not moved).

### Database location

`app.path().app_config_dir().join("projects.db")` — the same directory
`tauri-plugin-store` used for `projects.json`. No compatibility constraint:
there is no data to carry over, so the DB is simply created empty on first
run. A leftover `projects.json` from testing is ignored (a future cleanup
could delete it, but it's not worth a task).

## Data flow

```
GUI:  webview  --invoke-->  #[tauri::command] (3 lines)
                              --> State<Arc<ProjectService>>
                                    --> ports: SqliteRepository / OpenerLauncher / DetectorRunner

CLI (Spec 2):  argv  -->  recognizers + passthrough runner
                              --> core::ProjectService  (same instance shape)
                                    --> ports: SqliteRepository / <std launcher> / DetectorRunner
```

Both processes open the same `projects.db`. SQLite WAL + `busy_timeout`
serialize writes across processes; readers never block. This is the concrete
reason SQLite beats a shared JSON file here.

## Invariants

### Preserved (all existing `architecture.md` invariants hold)

1. New detector = implement + register, zero frontend code — **unchanged**;
   detectors just live in `core` now.
2. Basic detection stays cheap and bounded — **unchanged**.
3. Project identity is a stable UUID — **unchanged** (`Project::new`).
4. Directory normalization is deliberate and case-sensitive — **unchanged**;
   the normalized form is now also a stored column.
5. Validation advisory, final filesystem op authoritative — **unchanged**.
6. "What a project is" vs "what opens it" stay decoupled — **unchanged**
   (`Tracker` vs `AppLauncher`/`open_with`).
7. Detectors independent and unordered — **unchanged**.
8. Forward-compatible record shape — **kept, simplified**: new `Project`
   fields stay `Option<T>` or `#[serde(default)]`;
   `loads_a_record_missing_every_absorbable_field` and
   `rejects_a_record_missing_its_identity` move to `core` and still guard it.
   What's gone: the `serde_json::Value` `migrate()` step and its
   `schema_version` stamp (legacy-only). A shape change serde can't absorb is
   now a `user_version` migration step.

### New

9. **`core` never depends on `tauri`.** Enforced by the crate graph — a
   `use tauri::` in `core` fails to compile. Guards the "add a frontend
   without reworking the backend" goal.
10. **All persistence goes through `ProjectRepository`.** No frontend touches
    SQLite (or any store) directly. The service is the only caller of the
    port.
11. **Refresh stays all-or-nothing** — the recorded decision moves from a
    command doc-comment to a `ProjectService::refresh_trackers` doc-comment;
    the guard test moves to a service test
    (`refresh_leaves_stored_trackers_untouched_when_a_detector_fails`).

## Testing

| Layer | Approach |
|---|---|
| Existing 72 tests | Move with their code into `core`; pass unchanged. `cargo test -p indexer-core`. |
| `ProjectService` (new) | `SqliteRepository::in_memory()` + a `FakeLauncher` (records calls, returns configured results). One test per flow: dup-name reject, dup-dir reject, best-effort create with a real git temp dir, all-or-nothing refresh, open (missing dir → `DirectoryDeletedOrMoved`, missing app → `OpenWithAppMissing`, success → launcher args + `mark_opened` persisted), bin-only delete guard, `delete_directory` both branches, restore, list/deleted/favorites filtering + ordering, inspect (bad dir → `DirectoryState { ok: false }` not an `Err`), `ensure_project` idempotency. |
| `SqliteRepository` (new) | round-trip a `Project`; upsert replaces; `delete` idempotent; `list` returns deleted + active; `find_by_directory` hits the index; corrupt `data` → `RepositoryError::Corrupt`; `PRAGMA journal_mode` = `wal`; `open` on a fresh path creates the schema at `user_version = 1`. |
| `src-tauri` | `cargo build` gate + one smoke test that the `setup` closure assembles a `ProjectService` (using a temp dir) without panicking. Command wrappers too thin to unit-test. |
| Frontend | Unchanged behaviour. `CreateProjectForm` swaps its inline JS name logic (`repoNameFromUrl` / `folderNameFromDirectory` / `suggestProjectName` — currently untested) for the `suggest_project_name` command. The 14 `trackers.test.ts` vitest cases are untouched; `svelte-check` stays green. The name logic gains real coverage for the first time — as Rust unit tests in `core::domain::naming` (Task 2). |
| CI | `cargo test --workspace`, `cargo clippy --workspace`, `cargo fmt --check`, `npm test`, `npm run check`, `npm run build`. |

## Tasks

Each task ends green and committable. A task reviewer can reject one without
rejecting its neighbour.

1. **Workspace scaffold.** Root `Cargo.toml` `[workspace]`. Empty `crates/core`
   lib (`pub fn placeholder() {}`), `crates/cli` stub bin (`fn main() { eprintln!("project-indexer CLI: not implemented (spec 2)"); }`).
   `src-tauri` joins the workspace, adds the `indexer-core` path dep (unused so
   far). `cargo build --workspace` green; `cargo tauri dev` still launches the
   app.

2. **Domain + errors + naming → `core`.** Move `models/` → `core::domain`,
   `errors/` → `core::error`, `utils/normalize.rs` + `utils/sorting.rs` →
   `core::domain`. Add `RepositoryError`, `LauncherError`. Write
   `core::domain::naming` (`repo_name_from_url`, `folder_name_from_directory`,
   `suggest_project_name`) ported from `CreateProjectForm.svelte`, with unit
   tests (SSH/HTTPS remotes, `.git` suffix, trailing separators, no-remote
   fallback). Delete `src-tauri/src/migrations/` and every `migrate()` call
   site; drop the two legacy-migration tests, keep
   `loads_a_record_missing_every_absorbable_field` /
   `rejects_a_record_missing_its_identity`. Re-export from `core::lib`; fix all
   `src-tauri` imports. `cargo test -p indexer-core` green; `cargo tauri dev`
   still works.

3. **Detectors + platform → `core`.** Move `detectors/` wholesale. Move
   `utils/filesystem.rs` → `core::platform::filesystem`; move `system.rs`'s
   non-Tauri items (`remove_directory`, `open_with_app_available`,
   `command_exists`, `windows_path_extensions`, `windows_impl`, `linux_impl`,
   the `.desktop` parser + its tests) → `core::platform`. `git2`, `winreg`,
   `parselnk` deps move to `core`. `src-tauri::commands::system` keeps only
   `list_installed_apps` (delegating) and `open_in_app` (temporarily). Tests
   pass.

4. **Ports + `SqliteRepository`.** `core::ports::{repository, launcher}`.
   `core::infra::sqlite_repository` — `rusqlite` (bundled), `open` / `in_memory`,
   WAL + `busy_timeout`, the schema, the `user_version` runner (`v0→v1` creates
   the table), all five `ProjectRepository` methods. Repository tests with
   `in_memory()`.

5. **`ProjectService`.** All methods from the table, logic lifted verbatim from
   `commands/projects.rs` + `commands/inspect.rs`. `find_by_directory` /
   `ensure_project` added. Inspection DTOs → `core::application::inspection`
   with the `DirectoryStatusDto`→`DirectoryState` rename. Full service test
   suite (in-memory SQLite + `FakeLauncher`). The all-or-nothing guard test
   lands here.

6. **`OpenerLauncher` adapter.** `src-tauri::adapters::opener_launcher` — impl
   `AppLauncher`, body = today's `open_in_app` + `is_available` via
   `core::platform::open_with_app_available`. Delete `src-tauri`'s copy of
   `open_in_app`. `commands/system.rs` is now just `list_installed_apps`.

7. **Thin commands + wiring.** Rewrite every `#[tauri::command]` as a
   `State<Arc<ProjectService>>` pass-through. Add `suggest_project_name`. Wire
   `lib.rs` `setup` (SQLite open → assemble → manage). Remove
   `tauri-plugin-store` (dep, plugin line, `store/` module), the flush hook,
   the separate `.manage(DetectorRunner)`. `cargo tauri dev`: exercise
   create/list/edit/favorite/open/delete/restore/refresh/inspect by hand — GUI
   behaves exactly as before, now on SQLite.

8. **Frontend name suggestion.** `CreateProjectForm.svelte` calls
   `invoke("suggest_project_name", { directory })` on directory-pick instead of
   the inline helpers. Delete `repoNameFromUrl` / `folderNameFromDirectory` /
   `suggestProjectName` (no vitest cases exist for them — the coverage now
   lives in `core::domain::naming`). `svelte-check` + the 14 `trackers.test.ts`
   cases stay green. Manually verify: pick a git-repo directory in the Browse
   dialog → name field pre-fills with the remote repo name; pick a plain
   directory → fills with the folder name.

9. **Docs.** Rewrite `architecture.md`: new diagram (workspace / crate
   boundary), invariants 9–11, recorded decisions ("SQLite as a document
   store", "Tauri-free `core` crate", "one `ProjectRepository`, all
   persistence through it"), retire the "extract detection orchestration" and
   "platform provider traits" backlog items as done, note the
   `serde_json::Value` migration layer's removal. Update `knowledgebase.md`
   (module inventory, persistence section), `checklist.md` (new section),
   `KNOWN-ISSUES.md` if PI-003's line refs move. Add `accomplishments.md`
   entry. Register Spec 2 (observer CLI) as the next initiative.

Task 6 may fold into 7 during planning.

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| `rusqlite` bundled needs a C compiler in CI | Standard on all three runners (MSVC / clang). Documented as a build prerequisite. First-build time increases ~10–20s; cached afterward. |
| Moving 30+ files across a crate boundary breaks imports in bulk | Tasks 2–3 move by concern, each ending green. `cargo tauri dev` is re-verified after 2, 3, 7. |
| Windows linker can't relink `project-indexer.exe` while the dev app runs (known issue) | Kill the running app before `cargo build` in each task; noted for implementers. |
| `pnpm-lock.yaml` churn from `pnpm dev` (known issue) | `git checkout -- pnpm-lock.yaml` before each commit; unaffected by this work but implementers will hit it. |
| Trait objects (`Arc<dyn ProjectRepository>`) vs generics — managed-state ergonomics | `Arc<dyn>` chosen deliberately (matches `Box<dyn Detector>`); `ProjectService` is a concrete type in managed state, so commands stay non-generic. |
| Scope creep from Spec 2 leaking in | `crates/cli` is a stub with one `eprintln!`. Only two service methods (`find_by_directory`, `ensure_project`) and the naming module are added "early", each independently useful to the GUI. |

## Spec 2 preview — the observer CLI (not built here)

Captured so `core` is shaped right; full design is its own spec.

- `indexer <cmd> [args…]` runs `<cmd>` untouched (inherited stdio, propagated
  exit code) and, after it exits, matches `argv` + cwd + exit code against a
  set of **recognizers** — `mkdir <name>`, `git init`, `git clone <url>`,
  `gh repo create`, `cargo new`, … — and records inferred project facts via
  `core::ProjectService` (`ensure_project`, `refresh_trackers`,
  `find_by_directory`). It never reimplements the wrapped tool.
- Plain subcommands (`indexer list`, `indexer show <id>`, …) also exist,
  straight over the service.
- "Connecting" a later-installed CLI to the GUI is automatic: both open the
  same `projects.db`. No pairing, no IPC. (A live GUI reacting to CLI writes —
  via DB-file watching — is a possible Spec 2 nice-to-have, not a requirement.)
- Likely `core` addition in Spec 2: a `CommandObserver` trait (argv/cwd/exit →
  `Vec<ProjectFact>`), mirroring `Detector`. Out of scope now.
- The recognizer plumbing (subprocess spawn, shell integration) lives in
  `crates/cli`, not `core`.
