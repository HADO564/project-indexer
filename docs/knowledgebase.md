# Project Indexer — Knowledgebase

Reference for how the app works and why it's built this way. Unlike `accomplishments.md`, this file describes current state, not history — update it in place as things change rather than appending. See `checklist.md` for what's still open.

## What this is

A Tauri v2 desktop app (Rust backend, Svelte 5 + SvelteKit + Tailwind v4 frontend) for tracking local project directories: register a directory as a "project," tag/favorite it, launch it in a chosen app or the file explorer, soft-delete/restore it via a bin, and see what kind of project it is (git, Unreal, ...) via per-project detection. Single-instance, with window-state persistence and a global shortcut plugin wired in (shortcut itself not yet bound to an action).

## Backend architecture (`src-tauri/src/`)

- `models/` — `Project` (`trackers: Vec<Tracker>`), `UpdateProject` (partial-update DTO with a double-`Option` deserializer to distinguish "omit field" from "set to null"), `Tracker` (enum: `Git(GitInfo)` / `Unreal(UnrealInfo)` — a variant exists only if a detector can produce it, so a new variant lands together with its detector), `GitInfo`, `UnrealInfo`, `InstalledApp`.
- `commands/` — `projects.rs` (CRUD, open/delete/restore/untrack, plus detection — see "Tracker detection" below) and `system.rs` (installed-app discovery for the "open with" picker, directory removal, app-launch helpers with separate Windows/Linux implementations; macOS still unimplemented).
- `store/` — `ProjectStore`, a thin wrapper over `tauri-plugin-store` (a JSON k/v file, `projects.json`) with an autosave debounce; `ProjectStore::flush()` is called explicitly on window close to avoid losing last-second writes. Chosen over always-explicit `.save()` because the plugin's default autosave already debounces writes — see the reasoning in the project's memory (`project_store_save_strategy`) if that trade-off needs revisiting.
- `migrations/` — a `schema_version`-stamping migration pipeline for the stored JSON. Currently a no-op beyond stamping (`CURRENT_VERSION = 1`), but the seam exists for future field renames/type changes that `#[serde(default)]` can't absorb.
- `utils/` — `filesystem.rs` (directory status checks), `sorting.rs` (`sort_projects`, alphabetical/last-opened with direction, favorite/deleted filters), `normalize.rs` (directory-path normalization, tag normalization, space-stripping — free functions, each independently unit-tested).
- `detectors/` — the `Detector` trait (`detector.rs`: one method, `detect(&Path) -> Result<Option<Tracker>, DetectorError>`), a `DetectorRunner` (`runner.rs`) whose `detect_project(&Path) -> Detection` is the canonical detection operation, and `registry.rs` (`default_detectors()`), the single place detectors are registered (`Gitector`, `UnrealDetector`). `detect_project` is infallible by construction: it runs every detector and collects results into `Detection { trackers, errors }`, so one detector erroring neither stops the others nor discards their trackers. `Detection::into_result()` gives the all-or-nothing view for callers that persist. The app builds one `DetectorRunner` at startup into Tauri managed state; commands take `State<'_, DetectorRunner>`.
- `errors/` — `ProjectError` (surfaced to the frontend as a plain string), `GitError`, `UnrealError`, `DetectorError` (`Io`/`Git`/`Unreal` typed variants plus an `Other(Box<dyn Error + Send + Sync>)` catch-all so a new detector needn't edit the enum; converted to `ProjectError::Detection` at the command boundary).

## Frontend architecture (`src/`)

- SvelteKit with static adapter, Tailwind v4. `+page.svelte` is the single main view; feature UI is split into modals/components: `CreateProjectForm`, `EditProjectForm`, `ProjectList`/`ProjectCard`, `ProjectDetailModal`, `TrackerBadges`, `BinModal`, `FavoritesModal`, `DeleteModal`, `AppPicker`, `OpenWithMissingModal`, `DirectoryField`, `SortControls`, `ErrorBanner`.
- `lib/trackers.ts` — `trackerKind()`/`trackerFields()` read a tracker's variant name and payload generically off its serde shape (`{ Git: {...} }`, or a bare string for a future unit variant) instead of switching on known type names. Both `TrackerBadges` and `ProjectDetailModal` use this, so a future detector that starts returning real data shows up in the UI automatically — no frontend changes needed.
- `lib/api/` mirrors backend commands 1:1: `projects.ts`, `apps.ts`, `types.ts` (hand-mirrors the Rust structs), `errors.ts` (normalizes thrown values), `opener.ts`.
- `app.css` declares `color-scheme: light dark` on `<html>` so native widgets (selects, scrollbars, spinners) follow the active theme — see `KNOWN-ISSUES.md` PI-001 for why this was needed.
- No frontend test runner (no vitest/playwright config); `svelte-check` (`npm run check`) is the only frontend verification.

## Domain model (`Project`)

Fields: `id`, `name`, `description`, `directory`, `created_at`/`updated_at`, `last_opened_at`, `tags`, `favorite`, `open_with`, `notes`, `client`, `is_deleted`, `trackers: Vec<Tracker>` (defaults to empty via `#[serde(default)]`, not `Option`).

Invariants (see doc comments in `project.rs`):

- New fields must be `Option<T>` or `#[serde(default)]` so old stored records keep loading — enforced by a test that deserializes a "legacy record" missing every optional field.
- `id`/`directory` are strict — a record missing either fails to load rather than loading blank.
- Directory and name uniqueness checks happen at creation via directory-path normalization (`utils/normalize.rs`) so `C:\Foo\` and `C:/Foo` collide correctly; case is deliberately _not_ normalized.
- Soft-delete: `delete_project_directory` removes the directory from disk and either purges metadata immediately or marks `is_deleted` for the bin; `delete_project` (metadata-only purge) refuses to run on a project that isn't already soft-deleted; `untrack_project` drops tracked metadata without touching the directory at all, usable on any project regardless of `is_deleted`.

## Tracker detection, end to end

All three entry points call the same `DetectorRunner::detect_project` (pulled from managed state) and differ only in what they do with `Detection { trackers, errors }`:

1. `create_project` — best-effort: `trackers` are kept, `errors` are logged not surfaced, since the project is still worth tracking. Resilient, so a git hiccup doesn't cost you an Unreal tracker on the same directory.
2. `refresh_project_trackers` — all-or-nothing (`Detection::into_result`): checks directory health first (a moved directory gets its own clear error), then any detector failure is surfaced and the stored trackers are left untouched. It's an explicit user-triggered retry, so a half-applied refresh would be more confusing than a clear failure.
3. `detect_project_trackers` — advisory preview against a directory that isn't a project yet; nothing touches the store. Best-effort like `create_project`; returns `Vec<Tracker>` directly. Backs the browse-autocomplete flow below.
4. Two real detectors are registered (in `detectors/registry.rs`):
   - `Gitector` — repo root, dirty (untracked + modified, not ignored), detached-HEAD, remote URL (`origin`), current branch (handles the unborn-HEAD case for a fresh repo with no commits), every local branch, HEAD commit hash. `contributors` is deliberately left `Vec::new()` — see `checklist.md`.
   - `UnrealDetector` — finds the `.uproject` file directly inside a directory (not discovered upward like git), parses its JSON for engine association/category/description/modules/enabled plugins, and reads the configured source-control provider from `Saved/Config/<Platform>Editor/SourceControlSettings.ini` (a per-user file most `.gitignore`s exclude, so `None` on a fresh clone is the common case, not a bug).
5. **Browse-to-prefill:** picking a directory in `CreateProjectForm` calls `detect_project_trackers` and suggests a name (git remote's repo name, else the folder name) — only while Name is still empty, and silently skipped on any detection failure.
6. **Project detail view:** `ProjectCard`'s "Details" button opens `ProjectDetailModal` — project identity up top, then one tab per detected tracker with its fields rendered generically via `lib/trackers.ts`.

## Test coverage

53 Rust unit tests (`cargo test --lib` in `src-tauri/`), covering: directory/tag normalization, duplicate detection, directory health checks, soft-delete/restore/untrack, legacy-record migration compatibility, sorting (alphabetical/last-opened, both directions, tie-breaking), favorite/deleted filtering, installed-app availability checks, (Linux-only) `.desktop` exec-string parsing/launching, git detection (repo/non-repo, unborn HEAD, committed HEAD, dirty state, remote URL, multiple branches, detached HEAD), Unreal detection (`.uproject` discovery, descriptor parsing incl. missing-field defaults, source-control provider incl. an explicit `Provider=None`), and detector-runner behaviour (`Send + Sync`, empty result, one detector failing without discarding the others' trackers). No integration tests against the Tauri command layer itself. `svelte-check` is the only frontend verification.

## Dependencies of note

`git2` (libgit2 bindings, used by `Gitector`), `serde_json` (Unreal `.uproject` parsing), `chrono`, `uuid`, `thiserror`, `tauri-plugin-store/dialog/shell/opener/global-shortcut/single-instance/window-state`. Windows-only: `winreg`, `parselnk` (Start Menu `.lnk` resolution for the app picker).
