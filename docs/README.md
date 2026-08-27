# Project Indexer — Project State Summary

_Written 2026-08-25 as planning input. Reflects the codebase as of commit `2e54aee` plus one uncommitted change (a comment tweak in `tracker.rs` and the `project.rs` normalization refactor below)._

## What this is

A Tauri v2 desktop app (Rust backend, Svelte 5 + SvelteKit + Tailwind v4 frontend) for tracking local project directories: register a directory as a "project," tag/favorite it, launch it in a chosen app or the file explorer, and soft-delete/restore it via a bin. Single-instance, with window-state persistence and a global shortcut plugin wired in (shortcut itself not yet bound to an action).

## Architecture

**Backend** (`src-tauri/src/`):
- `models/` — `Project` (the core record), `UpdateProject` (partial-update DTO with a double-`Option` deserializer to distinguish "omit field" from "set to null"), `Tracker` (enum: Git/Unreal/Unity/Blender — see below), `InstalledApp`.
- `commands/` — Tauri command handlers: `projects.rs` (CRUD + open/delete/restore) and `system.rs` (installed-app discovery for the "open with" picker, plus directory removal and app-launch helpers with separate Windows/Linux implementations).
- `store/` — `ProjectStore`, a thin wrapper over `tauri-plugin-store` (a JSON k/v file, `projects.json`) with an autosave debounce; `flush()` is called explicitly on window close to avoid losing last-second writes.
- `migrations/` — a `schema_version`-stamping migration pipeline for the stored JSON. Currently a no-op beyond stamping (`CURRENT_VERSION = 1`), but the seam exists for future field renames/type changes that `#[serde(default)]` can't absorb.
- `utils/` — `filesystem.rs` (directory status checks), `sorting.rs` (alphabetical / last-opened sort with direction, favorite/deleted filters), `normalize.rs` (just extracted: directory-path normalization, tag normalization, space-stripping — previously private methods on `Project`).
- `detectors/git/gitector.rs` — **built but unwired.** Git repo detection (discover repo, current branch incl. unborn-branch handling, dirty-tree check via untracked+modified). Has its own `GitError` type registered in `errors::mod`. Nothing in `commands/` calls it, `Tracker` is never actually set to `Git` anywhere, and the frontend's `Project` TS type doesn't even have a `tracker` field yet. This is the clearest "in-flight, half-landed feature" in the repo.
- `errors/` — `ProjectError` (the one surfaced to the frontend, serialized as a plain string for `invoke()` callers) and `GitError` (exists, not yet connected to `ProjectError` or any command).

**Frontend** (`src/`):
- SvelteKit with static adapter, Tailwind v4. `+page.svelte` is the single main view; feature UI is split into modals/components: `CreateProjectForm`, `EditProjectForm`, `ProjectList`/`ProjectCard`, `BinModal`, `FavoritesModal`, `DeleteModal`, `AppPicker`, `OpenWithMissingModal`, `DirectoryField`, `SortControls`, `ErrorBanner`.
- `lib/api/` mirrors backend commands 1:1 (`projects.ts`, `apps.ts`, `types.ts` hand-mirrors the Rust structs, `errors.ts` normalizes thrown values, `opener.ts`).
- No test setup on the frontend (no vitest/playwright config found); `svelte-check` is the only frontend verification (`npm run check`).

## Domain model (`Project`)

Fields: `id`, `name`, `description`, `directory`, `created_at`/`updated_at`, `last_opened_at`, `tags`, `favorite`, `open_with`, `notes`, `client`, `is_deleted`, `tracker` (`Option<Vec<Tracker>>`, unused in practice — always `None` today).

Notable invariants enforced in code (see doc comments in `project.rs`):
- New fields must be `Option<T>` or `#[serde(default)]` so old stored records keep loading — enforced by a test that deserializes a "legacy record" missing every optional field.
- `id`/`directory` are strict — a record missing either fails to load rather than loading blank.
- Directory and name uniqueness checks happen at creation via directory-path normalization (`utils/normalize.rs`) so `C:\Foo\` and `C:/Foo` collide correctly; case is deliberately *not* normalized.
- Soft-delete: `delete_project_directory` removes the directory from disk and either purges metadata immediately or marks `is_deleted` for the bin; `delete_project` (metadata-only purge) refuses to run on a project that isn't already soft-deleted.

## Test coverage

32 Rust unit tests, all passing (`cargo test --lib`), covering: directory/tag normalization, duplicate detection, directory health checks, soft-delete/restore, legacy-record migration compatibility, sorting (alphabetical/last-opened, both directions, tie-breaking), favorite/deleted filtering, installed-app availability checks, and (Linux-only) `.desktop` exec-string parsing/launching. No integration tests against the Tauri command layer itself (commands are thin wrappers over tested store/model logic, so this is a reasonable but real gap). No frontend tests at all.

## Recent history (last 10 commits)

1. `feat: added git repo reading` — the `gitector.rs` module (unwired, see above).
2. `feat: Added git errors` — `GitError` type.
3. `feat: added some features and applied all kinds of sorting` — `SortBy`/`SortDirection`/`SortOptions`, favorites/bin filtering.
4. `feat: sort projects by most recently opened`.
5. `feat: directory safety checks and soft-delete workflow` — the bin/soft-delete system.
6. Earlier: error-handling cleanup, invariants, structure cleanup, Linux compatibility fix, first commit.

Trajectory: core CRUD → safety/soft-delete → sorting/views → now starting on per-project tooling detection (git, and per the `Tracker` enum's `TODO` comment, presumably Unreal/Unity/Blender next), which is mid-flight.

## Known gaps / loose ends worth planning around

1. **Git detection is dead code.** `detectors/git/gitector.rs` has no caller. Deciding whether/how to surface it (a command? auto-run on create? a badge in `ProjectCard`?) is the most obvious next-step decision.
2. **`Tracker` enum has no detection logic for Unreal/Unity/Blender**, despite existing as variants — only Git has a detector module, and even that isn't wired up.
3. **Frontend `Project` type is behind the backend struct** — missing `tracker`. Any UI work surfacing tracker info needs this added first.
4. **`GitError` isn't connected to `ProjectError`** — there's no `From<GitError> for ProjectError` or equivalent, so a command calling into `gitector` today would need new plumbing.
5. **No command-layer integration tests** — `commands/projects.rs` and `commands/system.rs` are exercised only indirectly through the unit tests of the things they call.
6. **macOS is unimplemented** for both `list_installed_apps` (returns empty) and app-launch specifics (falls through to the generic opener path) — Windows and Linux are the only platforms with real support.
7. **Global shortcut plugin is registered but no shortcut is bound** to any action yet (`tauri_plugin_global_shortcut::Builder::new().build()` with no handler wired beyond plugin init).
8. **No frontend automated tests** (`svelte-check` only).
9. Just-completed housekeeping: normalization helpers (`normalize_directory`, `normalize_tag(s)`, `remove_spaces`) were extracted from `Project` into `utils/normalize.rs` as free functions, each independently unit-tested — this was pure refactor, no behavior change, all 32 tests still pass.

## Dependencies of note

`git2` (libgit2 bindings, already in use for the unwired detector), `chrono`, `uuid`, `thiserror`, `tauri-plugin-store/dialog/shell/opener/global-shortcut/single-instance/window-state`. Windows-only: `winreg`, `parselnk` (Start Menu `.lnk` resolution for the app picker).
