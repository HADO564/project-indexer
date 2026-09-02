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

## 2026-08-31 — Detection consolidated around one canonical operation

- `7f8c3a0` Made `DetectorRunner::detect_project` the single detection entry point and tightened the surface around it:
  - `Detector` trait collapsed from two methods (`detect() -> bool` + `get_info() -> Option<Tracker>`, the former only ever called by tests) to one: `detect(&Path) -> Result<Option<Tracker>, DetectorError>`. `Gitector`/`UnrealDetector` lost their redundant presence-check impls; `Gitector::is_repo` (now unused) removed.
  - `detectors/registry.rs` added — `default_detectors()` is the one place detectors are registered. `DetectorRunner::default()` delegates to it.
  - `DetectorRunner` now built once at startup into Tauri managed state; `create_project` / `refresh_project_trackers` / `detect_project_trackers` take `State<'_, DetectorRunner>` instead of calling a free `detect_project` function (removed). Frontend `invoke` calls unchanged.
  - Detection is resilient: `detect_project` returns `Detection { trackers, errors }` and is infallible by construction — a failing detector lands in `errors` without discarding the trackers other detectors produced or stopping the ones after it. `Detection::into_result()` is the all-or-nothing view.
    - `create_project` / `detect_project_trackers` are best-effort (keep `trackers`, log `errors`); `detect_project_trackers` returns `Vec<Tracker>` directly.
    - `refresh_project_trackers` stays all-or-nothing via `into_result()` and now runs `Project::check_directory_health` first, so a moved/deleted directory reports `DirectoryDeletedOrMoved` rather than a raw detector I/O string.
  - `DetectorError` gained an `Other(Box<dyn Error + Send + Sync>)` catch-all so a new detector with its own error type needn't edit the shared enum. (The enum was kept over a boxed newtype — the typed `Git`/`Unreal` variants are what make `?` ergonomic inside detectors.)
  - Placeholder `Tracker::Unity` / `Tracker::Blender` variants (no detector behind them) dropped from the enum and `types.ts` — re-add each with its detector.
  - Dead empty `src-tauri/src/state/` module deleted.
  - 53 lib tests (one redundant git test dropped in the trait collapse, one resilience test added — a deliberately-failing detector alongside `Gitector`). `cargo build` / `cargo clippy` / `npm run check` clean (same pre-existing warnings).
- `8602bd0` `style: apply rustfmt across src-tauri` — the tree was never rustfmt-clean (mixed 2-space indent, hand-wrapped call chains, unsorted `use`/`mod` lines). Ran `cargo fmt` once as a standalone commit; no behavior change.

## 2026-08-31 — Project view

The `/project/[id]` detail view (branch `feat/project-view`), plus the backend
work it needed. Nine commits on the branch, then this docs pass.

- **`Detector::kind() -> &'static str`** (`"git"`, `"unreal"`) — a stable,
  lowercase detector identity. The frontend no longer infers *detection*
  identity from serde shape; `trackers.ts` still reads the variant name for
  tab labels only. Checks off the "explicit tracker/detector identity"
  backlog item (`Tracker::kind()` itself wasn't needed — the outcome `kind`
  covers every call site).
- **`Detection` carries `outcomes`** — `Vec<DetectorOutcome>`, one
  `DetectorOutcome::{Detected { kind, tracker } | NotDetected { kind } |
  Failed { kind, error }}` per detector consulted, replacing the parallel
  `{ trackers, errors }` lists. `.trackers()` / `.errors()` project them out;
  `.into_result()` is unchanged (still all-or-nothing, still guarded by
  `into_result_discards_partial_trackers_on_any_error`). `DetectorRunner::inspect(path, only)`
  added — the same pass restricted to one detector `kind`, for per-tab re-detect.
- **`GitInfo.web_url: Option<String>`** — browser-openable form of the remote,
  `git@` / `ssh://` / `https://` all normalized to `https://host/owner/repo`
  with the trailing `.git` stripped, derived by `Gitector`. `None` when the
  remote isn't a recognizable http/ssh git URL. Mirrored in `types.ts`.
- **`commands/inspect.rs` — `inspect_project(id, only) -> ProjectInspection`**
  — read-only: loads the stored project, runs a live detection pass against
  its directory, returns `{ project, directory_status: { ok, message? },
  results: [{ kind, status, tracker?, error? }] }`. Does **not** persist;
  `refresh_project_trackers` stays the only write path. A missing directory
  comes back as `directory_status.ok = false` with empty results, not a
  command error, so the view can still render identity. Registered in `lib.rs`.
- **`/project/[id]` route** (`+page.svelte` + `+page.ts`, `prerender = false`)
  replacing `ProjectDetailModal` (deleted). `ProjectIdentity.svelte` (the
  identity `<dl>`, lifted out of the modal), a per-detector status strip, one
  tab per detected tracker rendered by the generic **`TrackerPanel.svelte`**,
  per-tab re-detect, an Edit overlay, and Refresh (the persist action).
  `ProjectCard`'s "Details" is now `<a href="/project/{id}">`.
- **`trackers.ts` typed fields** — `trackerFields()` returns `text` / `code` /
  `link` / `path` / `chips` / `flag`, inferred from each key's name and value
  shape, zero per-tracker-kind code. `http(s)://` → link, `git@` / `ssh` →
  copyable `code` (not a broken link), `*_root` / `*_path` / `*_dir` → path,
  `*hash*` / `*commit*` → code, arrays → chips, true booleans → flag.
  `TrackerPanel` renders each type with open / reveal / copy affordances
  (clipboard failure swallowed, open/reveal failure → banner).
- **`src/lib/api/`** — `inspectProject()` (projects.ts), `openExternalUrl()` +
  `revealPath()` (opener.ts), the inspection DTOs (types.ts).
- **`vitest`** added (dev-dep, `test` script, `test` block in `vite.config.js`
  — the repo uses `.js`). `src/lib/trackers.test.ts` — 10 tests over the
  field-type inference. Now a second frontend check alongside `svelte-check`.
- Final checks green: `cargo test --lib` 61 passing, `cargo fmt --check`
  clean, `cargo clippy --lib` only the 2 pre-existing warnings (`sort_by_key`,
  module-name), `cargo build` clean; `npm test` 10 passing, `npm run check` 0
  errors (8 pre-existing `EditProjectForm` warnings), `npm run build` succeeds.
