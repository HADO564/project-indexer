# Project Indexer — Accomplishments Log

A dated record of what's been completed, in the order it landed. Append new entries at the bottom as work lands — don't rewrite history here. For current-state reference (not history), see `knowledgebase.md`; for what's still open, see `checklist.md`.

## 2026-08-20 — Project start

- `9e5cd85` First commit: initial Tauri v2 + Svelte 5/SvelteKit/Tailwind v4 scaffold.

## 2026-08-21 – 2026-08-22 — Linux compatibility

- `a5cdbaf` Made the app buildable and runnable on Linux.
- `44bd077` Fixed the Linux build so it actually starts and launches apps correctly.

## 2026-08-23 — Invariants and error handling

- `c5f7c4d` Implemented core `Project` invariants (name/directory validation, uniqueness).
- `c726da7` Applied consistent error handling across the backend.
- `83de582` Structure clean-up and optimization pass.

## 2026-08-24 — Soft-delete, sorting, and views

- `7f8d8ae` Directory safety checks and the full soft-delete/restore ("bin") workflow.
- `5020bf4` / `9b925c9` Sorting support: alphabetical and most-recently-opened, with direction, plus favorites/bin filtering.

## 2026-08-25 — Git detection built

- `2d2e8d5` `GitError` type added.
- `2e54aee` `Gitector` built: repo discovery, current branch (incl. unborn-branch handling for a fresh repo with no commits), dirty-tree check, and more — but not yet wired into any command.

## 2026-08-27 — Detection wired end to end, Unreal support added

- `e2797ee`
  - `detect_project` wired into `create_project` (best-effort) and a new `refresh_project_trackers` command (explicit retry, surfaces failures).
  - `untrack_project` command (drop tracked metadata without touching the directory).
  - `get_all_projects` made sort-aware.
  - `Project.tracker: Option<Vec<Tracker>>` renamed to `trackers: Vec<Tracker>`.
  - `Project`'s normalization helpers extracted into `utils/normalize.rs` as free, independently-tested functions.
  - Full **Unreal detector** added, mirroring `Gitector`/`GitInfo`: `UnrealInfo` model, `UnrealError`, `UnrealDetector` (finds the `.uproject` file, parses engine association/category/description/modules/enabled plugins, reads the configured source-control provider from `SourceControlSettings.ini`), registered in `DetectorRunner::default()`. `Tracker::Unreal` went from a bare unit variant to `Unreal(UnrealInfo)`.
  - Frontend `types.ts`/`TrackerBadges.svelte` updated to match the new `{ Unreal: UnrealInfo }` shape.
- `9761e80` "Quality of life changes":
  - `detect_project_trackers` command — runs detection against a directory that isn't a project yet.
  - **Browse-to-prefill**: picking a directory in `CreateProjectForm` now suggests a project name (git remote's repo name, else the folder name), only when Name is still empty.
  - **`ProjectDetailModal`** added: project identity plus one tab per detected tracker.
  - **`lib/trackers.ts`** added: generic `trackerKind()`/`trackerFields()` helpers so `TrackerBadges` and `ProjectDetailModal` render any tracker's data automatically, without per-tracker-type frontend code.
  - 9 new unit tests for `gitector.rs` (previously had none) covering detect/no-repo, unborn HEAD, committed HEAD, dirty state, remote URL, multiple branches, and detached HEAD.

## 2026-08-28 — Linux verification pass

- `761d848` Fixed a Linux-only dark-mode bug: the sort `<select>` rendered unreadable because the page never declared `color-scheme`, so the engine assumed `light` and painted it as a native light menulist under `dark:` text. Fixed with `color-scheme: light dark` on `<html>` (also covers scrollbars/spinners; the same bug is latent on Windows switched to dark mode).
- `f032f18` `docs/KNOWN-ISSUES.md` added: a 4-issue triage from a full Linux build-and-run pass (`cargo test` — 61 passed at the time, `pnpm build`/`pnpm tauri build` clean). See that file for details; only one item (`PI-004`, a comment-accuracy nit) remains open.

## 2026-08-30 — Docs restructure

- Replaced the single evolving `docs/README.md` summary with this knowledgebase/accomplishments/checklist split, plus verified the then-current state: 53 Rust tests passing, `cargo clippy` clean (2 pre-existing warnings), `npm run check` clean (8 pre-existing warnings, all in `EditProjectForm.svelte`, documented in `KNOWN-ISSUES.md` PI-003 as a false positive).

## 2026-08-30 — Detection consolidated around one operation

- Made `DetectorRunner::detect_project(&Path) -> Result<Vec<Tracker>, DetectorError>` the single canonical detection operation. Supporting changes:
  - `Detector` trait collapsed from two methods (`detect() -> bool` + `get_info() -> Option<Tracker>`, the former only ever called by tests) to one: `detect(&Path) -> Result<Option<Tracker>, DetectorError>`. `Gitector`/`UnrealDetector` lost their redundant presence-check impls; `Gitector::is_repo` (now unused) removed.
  - `detectors/registry.rs` added — `default_detectors()` is the one place detectors are registered. `DetectorRunner::default()` delegates to it.
  - `DetectorRunner` now built once at startup into Tauri managed state; `create_project` / `refresh_project_trackers` / `detect_project_trackers` take `State<'_, DetectorRunner>` instead of calling a free `detect_project` function (removed). Frontend `invoke` calls unchanged.
  - `DetectorError` gained an `Other(Box<dyn Error + Send + Sync>)` catch-all so a new detector with its own error type needn't edit the shared enum.
  - Placeholder `Tracker::Unity` / `Tracker::Blender` variants (no detector behind them) dropped from the enum and `types.ts` — re-add each with its detector.
  - Dead empty `src-tauri/src/state/` module deleted.

## 2026-08-31 — Detection made resilient

- Follow-up from a senior-standards review of the consolidation above.
  - `detect_project` no longer aborts all detection on the first detector error. It returns `Detection { trackers, errors }` and is infallible by construction — a failing detector lands in `errors` without discarding the trackers other detectors produced or stopping the ones after it. `Detection::into_result()` is the all-or-nothing view.
  - `create_project` and `detect_project_trackers` are now best-effort (keep `trackers`, log `errors`); `detect_project_trackers` returns `Vec<Tracker>` directly. `refresh_project_trackers` stays all-or-nothing via `into_result()` and now runs `Project::check_directory_health` first, so a moved/deleted directory reports `DirectoryDeletedOrMoved` instead of a raw detector I/O string.
  - `DetectorError`/managed-state/registry decisions from the review were kept as-is (the error enum earns its keep via `?` ergonomics inside detectors; managed state was a deliberate choice for the config seam).
  - 53 Rust tests (added one for the resilience path — a deliberately-failing detector alongside `Gitector`). `cargo clippy` / `npm run check` clean (same pre-existing warnings).
