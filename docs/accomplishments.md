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
