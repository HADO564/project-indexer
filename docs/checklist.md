# Project Indexer — Checklist

What's done and what's still open, feature by feature. Check items off as they land; add new ones under "Open" as they come up. See `accomplishments.md` for the dated story of how the checked items got done, `knowledgebase.md` for how the finished pieces actually work, and `architecture.md` for the non-feature quality backlog (invariants, detection semantics, testing, platform seams).

## Git tracker

- [x] `GitInfo` model
- [x] `Gitector` connected to project detection (`detectors/registry.rs`)
- [x] `Tracker::Git` populated end to end (`create_project` / `refresh_project_trackers`)
- [x] Git info exposed to the frontend (`TrackerBadges` on the list, full field list in `TrackerPanel` on `/project/[id]`)
- [x] `Gitector` unit tests (11 — recognizes-repo/plain-dir, unborn HEAD, committed HEAD, dirty, remote URL, branches, detached HEAD, `web_url` normalization, `kind`)
- [ ] `GitInfo.contributors` — deliberately deferred, still `Vec::new()`. Planned: `git2::Repository::revwalk()`, field becomes `Vec<Contributor { name, email }>` rather than plain strings. Deferred over the cost of walking full history on every detection run.

## Unreal tracker

- [x] `UnrealInfo` model
- [x] `UnrealError` type
- [x] `UnrealDetector`
- [x] `find_project_file()` (immediate `.uproject` lookup, not upward-discovered like git)
- [x] `.uproject` JSON parsing (engine association, category, description, modules, enabled plugins)
- [x] Source-control provider detection (`SourceControlSettings.ini`)
- [x] `Detector::detect()` returning `Option<Tracker>`
- [x] Returns `Tracker::Unreal(UnrealInfo)`
- [x] `UnrealDetector` unit tests (10 — incl. `kind`)

## Detection plumbing

- [x] `DetectorRunner::detect_project` is the single canonical detection operation (`inspect(path, only)` is the same pass scoped to one detector `kind`)
- [x] `Detector` trait is two methods — `kind() -> &'static str` (stable identity) + `detect(&Path) -> Result<Option<Tracker>, DetectorError>`
- [x] `detectors/registry.rs` — one place to register a detector
- [x] `DetectorRunner` in Tauri managed state; commands take `State<'_, DetectorRunner>`
- [x] `DetectorError::Other` catch-all so a new detector needn't touch the shared enum
- [x] Resilient detection — `Detection { outcomes }`, one `DetectorOutcome::{Detected,NotDetected,Failed}` per detector; one detector failing doesn't discard the others (`.trackers()` / `.errors()` project them out)
- [x] Explicit detector identity — `Detector::kind()` tags every outcome; the frontend no longer infers detection identity from JSON shape (`architecture.md` backlog)
- [x] `refresh_project_trackers` checks directory health before detecting
- [x] Refresh all-or-nothing is a recorded decision (`architecture.md`) with a guard test (`into_result_discards_partial_trackers_on_any_error`), not incidental

## Detection UX

- [x] Browse-to-prefill: suggest a project name from the picked directory (git remote repo name, else folder name)
- [x] `detect_project_trackers` command (preview detection before a project exists)
- [x] Per-project detail view — project identity + one tab per detected tracker, generically rendered (`lib/trackers.ts`) so a future detector needs no new frontend code (see "Project view" below; started as `ProjectDetailModal`, now the `/project/[id]` route)

## Project view

- [x] `/project/[id]` route replaces `ProjectDetailModal`
- [x] Live read-only detection on open (`inspect_project`) + per-detector status strip
- [x] Generic `TrackerPanel` — typed fields, open/reveal/copy affordances, no per-kind UI code
- [x] `GitInfo.web_url` (SSH→HTTPS) — "open remote" for any project in git
- [x] Per-tab re-detect, jump-to-Edit, Refresh
- [x] `vitest` covering the `trackers.ts` inference rules
- [x] Edit overlay has a solid surface; undetected detectors fold into a disclosure

## GUI v1

- [x] Single dark "terminal" theme — semantic `@theme` tokens, VT323 (OFL) display font, cyan/gold/rust accents; every component off raw Tailwind colours
- [x] `color-scheme: dark` — retires the light/dark `<select>` bug class (PI-001)
- [x] `trackerColor(kind)` — per-kind badge/strip/tab hue, contrast-safe by construction
- [x] Project-card actions in a `···` menu
- [x] `list_missing_directories` + bin-icon marker for a project whose folder is gone
- [x] `open_in_app` strips `ELECTRON_RUN_AS_NODE` so Electron `open_with` targets launch (not run as Node)
- [x] List doesn't flash "Loading…" on refetch; sort control heights matched

## Frontend-agnostic core

The Rust backend restructured so the GUI is one frontend over a Tauri-free
library crate. Spec:
`docs/superpowers/specs/2026-09-02-frontend-agnostic-core-design.md`.

- [x] Cargo workspace — root `[workspace]` (`src-tauri`, `crates/core`, `crates/cli`)
- [x] `indexer-core` library crate — `domain` / `ports` / `application` / `detectors` / `platform` / `infra` / `error`, no `tauri` / `clap` in its dep tree (compiler-enforced)
- [x] `crates/cli` (`indexer-cli`) — one-line stub for the Spec 2 observer CLI
- [x] `ProjectReader` + `ProjectRepository` port (`ProjectReader` is the read-only half for an external consumer like devmon)
- [x] `AppLauncher` port + `OpenerLauncher` adapter in `src-tauri` (the only place `tauri-plugin-opener` is still used)
- [x] `ProjectService` — one method per command, orchestration lifted verbatim out of the Tauri handlers; `find_by_directory` / `ensure_project` added for Spec 2
- [x] `SqliteRepository` (`rusqlite`, bundled, WAL, `foreign_keys=ON`) at `app_config_dir/projects.db` — `Project` as a JSON blob + promoted `is_deleted` / `directory_normalized` / `updated_at` + a derived `project_tags` table + `meta`; `user_version` migration runner + version-skew guard
- [x] `#[tauri::command]` functions are ~3-line pass-throughs over `State<Arc<ProjectService>>`; `AppHandle` gone from every signature
- [x] Name suggestion moved to `core::domain::naming` + a `suggest_project_name` command (was inline JS in `CreateProjectForm.svelte`)
- [x] `tauri-plugin-store` + `store/` + `migrations/` deleted; `serde_json` / `chrono` / `uuid` / `thiserror` dropped from `src-tauri`; dead `@tauri-apps/plugin-store` npm dep removed
- [x] Zero user-visible change — same windows, command names, payloads, behaviour

## Test counts (Rust, `cargo test --workspace`)

105 `#[test]` attributes in total. **102 execute on Linux**, 94 on Windows — the
difference either way is platform-gated tests (11 `#[cfg(unix/linux)]`, the rest
`#[cfg(windows)]`). `cargo test -p project-indexer` is 0; everything lives in
`indexer-core`.

- [x] `Gitector` (11), `UnrealDetector` (10), detector-runner + `results_from`
- [x] `normalize`, `sorting`, `Project` invariants / soft-delete / health checks
- [x] `naming` (8) — SSH/HTTPS remotes, `.git` suffix, trailing separators, no-remote fallback, empty
- [x] `SqliteRepository` (11) — round-trip, upsert, idempotent+cascading delete (tag mirror asserted non-empty first), tag-mirror populate+replace, `list` incl. deleted, `find_by_directory` (normalized index + prefers live most-recent row), corrupt blob, fresh-DB schema, file-backed `open` creates schema (wal / `user_version` / `meta.schema_version`), refuses-newer-DB
- [x] `ProjectService` (15) — dup rejects, best-effort create, open (missing dir / missing app / success), all-or-nothing refresh, bin-only delete, `delete_directory` both branches, restore, inspect-bad-dir, `ensure_project` idempotency

## Background operation

- [x] System tray — closing the window hides the app instead of quitting; left-click restores, right-click gives Show / Quit
- [x] `tauri-plugin-single-instance` — a second launch brings the running window forward rather than starting a copy
- [x] Tray failure degrades instead of killing startup (`setup_tray_or_warn` + `TRAY_AVAILABLE`); no tray means closing genuinely quits, so the window can't hide beyond reach (`KNOWN-ISSUES.md` PI-005)

## Release engineering

- [x] CI — `cargo fmt --check` / `clippy` / `test --workspace` on Linux + Windows, plus `pnpm check` / `test` / `build`, on every push and PR
- [x] Release workflow — `v*` tag builds bundles for Windows, Linux, and both macOS architectures
- [x] `CHANGELOG.md` (Keep a Changelog) and a published v0.1.1
- [x] `LICENSE` (MIT), `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `ROADMAP.md`, `docs/USAGE.md`, issue + PR templates
- [ ] Signed bundles and the tag → signed-bundle → GitHub-Release path (see `architecture.md` "Cross-app & updates")
- [ ] CI never *launches* the app — it compiles and tests the Linux target only. A green run says nothing about whether the window appears; PI-005 is what that gap looks like in practice. A smoke launch under a virtual display would close it.

## Open (features)

- [ ] `GitInfo.contributors` (see above)
- [ ] Unity detector — add `Tracker::Unity` + a `UnityDetector` together (register in `detectors/registry.rs`)
- [ ] Blender detector — add `Tracker::Blender` + a `BlenderDetector` together
- [ ] macOS support — `list_installed_apps` returns empty, app-launch falls through to the generic opener path
- [ ] Global shortcut — plugin is registered but no shortcut is bound to any action
- [ ] **Spec 2 — observer CLI.** Fill in `crates/cli`: `indexer <cmd>` wraps a real command, matches argv+cwd+exit against recognizers, records project facts through `ProjectService`. Plain subcommands too.
- [ ] **Updater fast-follows** (see `architecture.md` "Cross-app & updates"): `tauri-plugin-updater` wiring + `core::updates::latest_stable`, a dismissible GUI release-notification chip, CLI `self-update` + stderr hint, GUI on-demand minisign-verified CLI download, tag→signed-bundle→GitHub-Release CI.
- [ ] **Plain CLI subcommands** — `indexer list` / `show` / `add` / `open` / `untrack` over the existing `ProjectService` methods. Separable from the observer; the open question is the output contract (`--json` is a compatibility promise).
- [ ] **Deeper git support** — ahead/behind upstream, the last commit's author/date/subject, stash count, uncommitted-file counts instead of a bare `dirty` bool, submodules/worktrees/tags/LFS. All refs-and-config reads; only `contributors` needs a `revwalk` and stays deferred.
- [ ] **Other version-control systems** — Mercurial, Subversion, Jujutsu, Perforce, Fossil. Independent detectors, so a jj repo colocated with git correctly reports both. Perforce complements the Unreal detector, which already reads the configured provider from `SourceControlSettings.ini`. Open decision: read on-disk metadata vs. shell out to the tool.
- [ ] **Scan a folder for projects** — point the app at `~/code` and register everything inside. Needs a depth limit, pruning (`node_modules`, `target`, `.venv`), stopping at an already-detected project, and a review step before bulk-adding. Shares the duplicate-name problem with the CLI's `ensure_project` — solve once. `find_by_directory` + the indexed `directory_normalized` already make the "already tracked?" check cheap.
- [ ] **Frontend plugins** — purpose-built panels, per-kind actions, and non-tracker views. Lowest priority: the generic renderer already handles any tracker acceptably. Needs a real CSP first — `tauri.conf.json` has `"csp": null`.
- [ ] `detect_project_trackers` command / `detectProjectTrackers` in `src/lib/api/projects.ts` — no frontend callers since Task 8 (name pre-fill moved to `suggest_project_name`). Kept deliberately: the command is still registered for a future CLI preview / Spec 2. Remove the JS wrapper if it's still unused when the API surface is next revised.

Non-feature work (testing, platform seams, tech debt, PI-004, lockfiles) lives in `architecture.md`.
