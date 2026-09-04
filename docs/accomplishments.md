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

## 2026-09-02 — GUI v1

The first real visual pass on the desktop UI (branch `feat/project-view`,
continued), plus two feature adds and an open-with fix.

- **Terminal restyle.** Dropped the Tailwind grey/blue + `prefers-color-scheme`
  dual theme for a single committed dark look driven by semantic tokens in
  `@theme` (`--color-void` / `-panel` / `-line` / `-phos` / `-accent` / `-gold`
  / `-rust`). Cool-slate surfaces, off-white text, cyan interactive accent,
  gold for state, rust for errors. `VT323` (SIL OFL) bundled in `static/fonts/`
  as the display face; system mono for data. Bordered not shadowed, 2px
  radius. `styles.ts` + every component swept off raw colour classes.
  `color-scheme: dark` retires PI-001's bug class (no light menulist to fight).
- **`trackerColor(kind)`.** Hand-picked hues for git/unreal/unity/blender/…,
  a stable name-hashed hue for anything new, all `hsl(H 65% 70%)` so the text
  contrasts on the dark ground without per-tracker tuning. On card badges, the
  `/project/[id]` status strip, and the active tab. +4 vitest cases (14 total).
- **Card action menu.** Open / Details / Detect / Edit / Delete collapse into
  a `···` dropdown so the row can't collide with a long project name.
- **Directory-gone marker.** New `list_missing_directories` command (ids of
  live projects whose folder is deleted/moved; inaccessible ≠ gone). The list
  fetches it once per load and shows an amber bin icon with the path struck
  through.
- **Open-with fix.** `open_in_app` on Windows now launches a chosen `.exe`
  directly via `std::process::Command` with `ELECTRON_RUN_AS_NODE` /
  `ELECTRON_NO_ATTACH_CONSOLE` stripped, instead of `ShellExecuteExW`. Root
  cause: running the app from a VS Code terminal inherits `ELECTRON_RUN_AS_NODE=1`,
  which makes `Code.exe <folder>` run as Node and `require()` the folder
  instead of opening it — while ShellExecute still reported success.
- **Polish.** `/project/[id]` Edit overlay got a solid surface + title bar
  (was see-through); undetected detectors fold into a "Not detected (N)"
  disclosure; the list no longer flashes "Loading…" on a re-sort; sort
  `<select>` and direction button matched to one height.
- Green: `cargo test --lib` 61, `cargo fmt`/`clippy` clean, `cargo build`
  clean; `npm test` 14, `npm run check` 0 errors, `npm run build` clean.
  Open verified in the running app.

## 2026-09-03 — Frontend-agnostic core

Branch `refactor/frontend-agnostic-core`. Spec 1 of 2
(`docs/superpowers/specs/2026-09-02-frontend-agnostic-core-design.md`): the Rust
backend restructured so the GUI is one frontend over a Tauri-free library crate,
with SQLite replacing the JSON store. **Zero user-visible change** — same
windows, command names, IPC payloads, and behaviour throughout. Nine tasks, each
ending green and committable.

- **Cargo workspace.** Root `[workspace]` over `src-tauri`, `crates/core`
  (`indexer-core`), `crates/cli` (`indexer-cli` — a one-line stub for the Spec 2
  observer CLI). Dependency direction is compiler-enforced: `src-tauri →
  indexer-core`, and a `use tauri::` in `core` fails to build.
- **`indexer-core`.** All domain logic, orchestration and persistence, in
  `domain` / `ports` / `application` / `detectors` / `platform` / `infra` /
  `error`. `models/` + `utils/{normalize,sorting}` → `domain`; `errors/` →
  `error`; `detectors/` moved wholesale; `utils/filesystem` + the non-Tauri
  half of `system.rs` (installed-app discovery, the `.desktop` parser,
  `remove_directory`) → `platform`. `git2` / `winreg` / `parselnk` moved with
  them.
- **`ProjectService`** (`core::application`) — one method per Tauri command,
  logic lifted verbatim out of the handlers, plus `find_by_directory` and
  `ensure_project` added for Spec 2. Holds `Arc<dyn ProjectRepository>` +
  `Arc<dyn AppLauncher>` + `Arc<DetectorRunner>`.
- **Two ports.** `ProjectReader` (get / list / find_by_directory — the
  read-only half, for an external consumer like devmon) +
  `ProjectRepository: ProjectReader` (save / delete); `AppLauncher` (open /
  is_available). New port errors `RepositoryError` / `LauncherError`, mapped in
  the service to `ProjectError::Store` / `::OpenFailed`.
- **SQLite swap.** `tauri-plugin-store` dropped entirely. `SqliteRepository`
  (`rusqlite`, `bundled`, WAL, `busy_timeout`, `foreign_keys=ON`) at
  `app_config_dir()/projects.db` — `Project` as a serde-JSON `data` blob +
  promoted `is_deleted` / `directory_normalized` / `updated_at` columns, `tags`
  mirrored into a derived `project_tags` table, a `meta(app, schema_version)`
  table. Schema evolution is a numbered `user_version` runner against
  `CURRENT_SCHEMA_VERSION`, with a version-skew guard (`open` refuses a DB from
  a newer binary). No autosave, no flush-on-close hook — writes are synchronous.
  **No JSON→SQLite import** — the app had no production data; the
  `serde_json::Value` migration layer (`migrations/`) was deleted, not ported.
- **`src-tauri` is now an adapter.** Every `#[tauri::command]` is a ~3-line
  pass-through over `State<Arc<ProjectService>>` (`AppHandle` gone from every
  signature). `adapters/opener_launcher.rs` (`OpenerLauncher impl AppLauncher`)
  is the one genuine adapter and the only remaining `tauri-plugin-opener` use;
  the Windows `ELECTRON_RUN_AS_NODE` env-scrub moved into it verbatim. `lib.rs`
  `setup` opens the DB and assembles the service into managed state. Removed:
  `store/`, `migrations/`, the flush hook, the separate `.manage(DetectorRunner)`.
- **Name suggestion moved to Rust.** `repo_name_from_url` /
  `folder_name_from_directory` / `suggest_project_name` ported from
  `CreateProjectForm.svelte` into `core::domain::naming` (unit-tested for the
  first time) behind a new `suggest_project_name` command; the inline JS
  helpers and the dead `@tauri-apps/plugin-store` npm dependency were deleted.
- **Dependency trim.** `src-tauri` dropped `serde_json` / `chrono` / `uuid` /
  `thiserror` — and `serde` — now that it holds no models or errors of its own.
- **JS toolchain committed to pnpm** — `packageManager: pnpm@11.21.0`,
  `beforeDevCommand` / `beforeBuildCommand` are `pnpm dev` / `pnpm build`, no
  `package-lock.json`.
- **Tests.** The pre-refactor 72 moved into `core` and pass unchanged; net 91
  executed on Windows (72 − 1 deleted `serde_json::Value` migration test + 8
  `naming` + 8 `SqliteRepository` + 15 `ProjectService`, with the `results_from`
  test relocated — 102 `#[test]` attributes total, 11 of them
  `#[cfg(unix/linux)]`-gated, so 91 run on Windows).
  `cargo test -p project-indexer` is now 0. Frontend: 14
  `trackers.test.ts` vitest cases untouched, `pnpm run check` 0 errors / 8
  known `EditProjectForm` warnings, `pnpm run build` clean.

## 2026-09-03 — v0.1.1: tray, CI, and the first release

The refactor's follow-ups closed out, the app made to run in the background, and
the project given the scaffolding a published release needs.

- `f4ee6cc` Post-refactor follow-ups cleared.
- `6b36bd7` Hardened repository lookups, surfaced database-open failures to the
  user instead of failing silently, and restored the project-name guard.
- `1d2b032` **System tray.** Closing the window now hides the app instead of
  quitting it: left-click the tray icon to restore, right-click for Show / Quit.
  A second launch (from a Start Menu shortcut while hidden) brings the running
  window forward rather than starting a second copy, via
  `tauri-plugin-single-instance`.
- `363a5d3` **CI and release workflows.** CI runs `cargo fmt --check`, `clippy`,
  `cargo test --workspace` on Linux and Windows, plus `pnpm check` / `test` /
  `build`, for every push and pull request. The release workflow builds bundles
  for Windows, Linux, and both macOS architectures on a `v*` tag.
- `272648d` **Released v0.1.1.** Fixed a startup hang where a failing database
  open produced no window and no message — `tauri-plugin-dialog`'s
  `blocking_show()` queues onto an event loop that has not started yet when
  called from `setup`, so it deadlocked. Replaced with a synchronous `rfd`
  dialog that also writes to stderr. Also fixed SQLite mutex poisoning
  permanently breaking saves after any unrelated panic, and `ensure_project`
  failing to infer a name from the git remote. Added a real README and a
  `CHANGELOG.md`.
- `64c52f3` Recorded 0.1.0 as built but never published.

## 2026-09-04 — Observer CLI handoff, and the first Linux run of the new core

- `f372e6d` / `5cf2275` **Handoff written** for the next initiative, the
  observer CLI (`docs/handoffs/2026-09-04-observer-cli.md`): what exists, what
  was already decided, and the nine questions deliberately left open for
  brainstorming. A "picking this up on Linux" section flags that CI proves the
  Linux target *compiles* but nobody had launched the window since before the
  core refactor.

- `567934f` **PI-005 — a missing appindicator library killed the app at
  startup.** That first Linux run found it immediately. Everything compiled, 102
  tests passed, and the app then exited with only a panic on stderr — invisible
  when launched from a `.desktop` entry.

  `libappindicator-sys` calls a bare `panic!` when it cannot load a library
  rather than returning an error, so `setup_tray(app.handle())?` never observed
  the failure: the `?` was dead code for it. The trigger is Linux-only, but the
  mishandling was not — on Windows a `Shell_NotifyIcon` failure returns an `Err`
  that took the same `?` → `.expect()` route out of `setup`, with the same
  no-window-no-message result. CI could never have caught either: it installs
  `libayatana-appindicator3-dev` and never launches the app.

  `setup_tray_or_warn()` now catches the unwind and names the package to
  install, and a `TRAY_AVAILABLE` flag gates the close handler — the load-bearing
  half, since closing hides to the tray, so degrading to "no tray" without it
  would strand the app with the window hidden and nothing to restore it.
  Verified on both paths by masking all four candidate libraries with bind mounts
  in an unprivileged user namespace. The README's Arch package list, which
  predated the tray, gained `libayatana-appindicator`.

- **Verified on Arch** beyond the tray, since this was the first real run of
  post-refactor `main` on Linux: `.desktop` app discovery returns 79 entries with
  Flatpak file-forwarding markers intact (~450 lines only CI had ever compiled),
  git detection reports the right branch/commit/remote, the NVIDIA DMABUF
  workaround engages on the proprietary driver, single-instance restore works,
  and the tray registers on the StatusNotifier watcher.

- **Community documentation added.** `LICENSE` (MIT — declared in the README and
  `package.json` since the start, but the file was missing), `CONTRIBUTING.md`,
  `CODE_OF_CONDUCT.md`, `SECURITY.md`, `ROADMAP.md`, `docs/USAGE.md`, and GitHub
  issue / pull-request templates.

- **Hardened and decided, in the same pass.** The `"csp": null` that had been
  sitting in `tauri.conf.json` since the first commit is now a real policy.
  SvelteKit owns the strict half because the inline boot script's hash changes
  every build and `mode: "hash"` recomputes it; Tauri carries a complementary
  policy and the browser enforces the intersection. Verified by running the
  release binary, not by reading the config: the window rendered, VT323 loaded
  under `font-src 'self'`, and the project list populated — which only happens if
  an `invoke` round-trip completed, so the IPC bridge survives `connect-src`.

  The delete dialog stopped defaulting to the destructive choice. It now opens on
  "just remove it from this app", that option is listed first, and the confirm
  button names the action it will take rather than always saying "Delete".

  `PI-004` was fixed and retired: the NVIDIA workaround's comment described a
  narrower trigger than the code has, since the *open* kernel module creates the
  same probe paths — deliberately caught, because it still pairs with the
  proprietary userspace GL stack.

- **A pre-commit hook** (`.githooks/pre-commit`, opt in with
  `git config core.hooksPath .githooks`) mirroring the CI gates, running only the
  ones the staged files can affect. It closes the "forgot to run the checks" gap
  but explicitly not the PI-005 gap — neither it nor CI launches the app, and
  that distinction is now written into `CONTRIBUTING.md` and the handoff rather
  than being something you had to discover.

- **The roadmap took its real shape.** Version control beyond git is plugin
  territory, not first-party work. Plugins are two shapes — a UI plugin over data
  the backend already produces, or a UI plugin paired with a Rust `Detector` for
  anything it cannot yet see — and each has its own containment story: a UI
  plugin can be sandboxed with capability scoping and a host API that replaces
  raw `invoke`, while native code in-process cannot be sandboxed at all, so a
  runtime `.so`/`.dll` loader is declined outright in favour of source
  distribution. The CLI's `--json` contract is settled ahead of the code:
  versioned envelope, additive-only within a version, unknown tracker kinds
  serialise instead of failing, stdout is data and stderr is prose.
