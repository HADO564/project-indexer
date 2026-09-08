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

**196 lib tests execute on Windows**, 204 on Linux, plus a `tests/migrations.rs`
integration suite (8, cross-platform). The difference either way is
platform-gated tests, mostly the Linux `.desktop`/Flatpak launch-argument suite
under `platform::app_discovery`'s `linux_impl`, the rest `#[cfg(windows)]`.
Migrations run identically on both. Workspace total via
`cargo test --workspace`: **212 on Linux, 204 on Windows**. `cargo test -p
project-indexer` is 0; everything lives in `indexer-core`.

The tests themselves live in `crates/core/src/tests/`, mirroring the module
tree — `domain/scan.rs`'s tests are at `tests/domain/scan.rs`. They are still
unit tests (declared from `lib.rs` behind `#[cfg(test)]`, compiled out of
release builds, able to reach `pub(crate)` internals); `crates/core/tests/` is
reserved for real integration tests, of which `migrations.rs` is the only one.
See `CONTRIBUTING.md` → *Project layout*.

- [x] `Gitector` (11), `UnrealDetector` (10), detector-runner (10) + `results_from` — incl. `inspect_kinds`' set filter: a named subset runs, an empty selection runs **nothing** (so a scan with no detectors ticked finds nothing rather than everything), and an unknown kind matches nothing rather than erroring
- [x] `normalize`, `sorting`, `Project` invariants / soft-delete / health checks
- [x] `naming` (14) — SSH/HTTPS remotes, `.git` suffix, trailing separators, no-remote fallback, empty; plus `disambiguate` — a free name passes through, a collision parent-qualifies (`api` → `work/api`), a second collision suffixes (`work/api (2)`), matching is case-insensitive to agree with `check_for_duplicate_name_or_dir`, Windows parents split on `\`, and a parent that yields no segment falls straight to the suffix rather than looping
- [x] `domain::group` + `palette` (14) — name trimmed and timestamped on create, empty name / empty icon / unknown colour rejected, duplicate-name check ignores case and surrounding space, `update` applies only the fields present and changes nothing when it rejects one; the eight swatch names are unique and an unknown one is not a swatch
- [x] `GroupService` (7) — create appends after the last group (so `reorder` stays the only thing that renumbers), duplicate name rejected case-insensitively, rename works, a name another group holds is rejected while a group keeping its own name is not, a missing id is `GroupNotFound`, reorder returns the persisted order rather than echoing the caller
- [x] `SqliteRepository` (17: 11 project + 6 group) — round-trip, upsert, idempotent+cascading delete (tag mirror asserted non-empty first), tag-mirror populate+replace, `list` incl. deleted, `find_by_directory` (normalized index + prefers live most-recent row), corrupt blob, fresh-DB schema, file-backed `open` creates schema (wal / `user_version` / `meta.schema_version`), refuses-newer-DB; plus group round-trip, position ordering, `set_group_positions` rewrites the whole ordering, deleting a group cascades to ungroup its members in both blob and column, deleting a missing group is idempotent, `save` writes `group_id` to both blob and column
- [x] Migration suite (`tests/migrations.rs`, 8) — a v1 database opens and keeps its rows, v1→v2 adds `groups` and `projects.group_id` and leaves existing projects ungrouped, opening an already-current database is a no-op, `meta.schema_version` is stamped regardless of which steps ran, a database from a newer binary is refused
- [x] Icon sanitizer (`icons::sanitize`, 25) — strips `<script>`/`<style>`/event handlers/`foreignObject`/`use`/`image`, decodes entities before scanning so numeric or undefined entities can't smuggle a `url()` past the filter, closes the CSS-escape `url()` bypass, drops `href` incl. the `xlink:href` form, enforces exactly one well-formed `<svg>` root, a size cap, and rejects non-SVG input or an SVG with nothing drawable left
- [x] Icon store (`infra::icon_store`, 10) — import sanitizes and stores the sanitized form, never the original; a name collision doesn't overwrite the first icon; delete refuses a name that could escape the store directory; listing skips a non-UTF-8 file or a directory masquerading as an icon without failing the rest of the listing
- [x] `ProjectService` (23) — dup rejects, best-effort create, open (missing dir / missing app / success), all-or-nothing refresh, bin-only delete, `delete_directory` both branches, restore, inspect-bad-dir, `ensure_project` idempotency, group assignment (an existing group succeeds, a nonexistent one is `GroupNotFound` and leaves the project unchanged, clearing the group needs no group to exist); plus the naming work — `ensure_project` disambiguates a colliding name instead of failing with `DuplicateName`, `ensure_project_named` honours the caller's name, is idempotent and **never renames** an already-tracked directory, and resolves an already-tracked directory from `find_by_directory` alone without paying for a detector run (guarded by a call-counting detector)
- [x] `domain::scan` (11) — the walk: quick stops one level down, deep honours *n*, a matched directory is a **boundary** and its children are never visited, the prune list applies and `include_ignored` defeats it, `.git` stays pruned **even with `include_ignored` on** (a repo's submodules live under `.git/modules` and would otherwise each be offered as a project), nothing ticked finds nothing, candidates carry a suggested name and matched kinds, two runs over one tree produce identical reports, a missing root is an empty report rather than a panic, and the `MAX_DIRECTORIES` ceiling sets `stopped_early` (tested via `walk_limited` with a tiny limit rather than 50,000 real directories)
- [x] `ScanService` (9) — names are assigned at scan so the review list shows what will be created, two candidates in one report cannot collide with each other, an already-tracked directory is flagged and keeps its plain name (rather than being parent-qualified against itself on every rescan), import registers the selection, an already-tracked row is counted skipped rather than re-created, **a failing row does not stop the rest**, a disambiguated candidate is flagged so the rename is visible, and a binned-then-recreated directory is offered again rather than reported as tracked

## Test counts (frontend, `pnpm test`)

99 vitest tests across six suites, all pure modules — no component is mounted,
which is the whole reason the logic lives in modules rather than in `.svelte`
files. `vite.config.js` runs them in the `node` environment, from
`src/tests/lib/`, mirroring the modules they cover the same way the Rust tests
do. **`ScanFolderModal` is the one substantial piece with no automated
coverage**, which is why the 0.3.0 scanner was released as a beta first.

- [x] `trackers` (14) — variant key, hand-picked vs name-hashed hues, and the field-affordance inference that keeps a new detector zero-frontend-code
- [x] `palette` (15) — the eight names mirror `core::domain::palette` in order, a known name resolves to its token, an unknown one and a null both fall back to neutral, a hex literal passes through only in the exact six-digit form, the mark colour cascades project → group → neutral, and no input can make `swatchVar` return anything but a token or that exact literal
- [x] `icons` (11) — 24 unique names each with at least one path, an unknown or absent or `custom:`-prefixed name falls back to the `folder` glyph, and the data URI percent-encodes what would otherwise break the attribute
- [x] `views` (36) — each of the five views resolves to the right projects, Ungrouped catches `group_id == null`, filtering preserves the backend's sort order, the query applies within a view rather than across everything, counts include an empty group as 0 while a project whose group was deleted counts towards nothing, and the `name: value` property syntax matches only projects carrying that property while `D:\Games` and `build at 12:30` stay ordinary text searches
- [x] `viewState` (9) — a persisted view whose group has since been deleted falls back to All, Bin is never restored as the landing view, and junk or absent storage defaults cleanly
- [x] `scanSettings` (14) — settings round-trip through storage; unparseable JSON, a wrong-shaped value and absent storage all fall back to defaults; stored detector kinds the build no longer registers are dropped, but if that leaves none it falls back to **every** kind rather than none (an empty list means a scan that silently finds nothing); depth is clamped both at restore and, critically, in `toScanRequest` — a cleared number input yields `null` and a typed fraction yields `2.5`, and Rust's `depth: u32` rejects both; and a quick scan carries no `depth` key at all, since `ScanMode` is a flattened internally-tagged enum

## Background operation

- [x] System tray — closing the window hides the app instead of quitting; left-click restores, right-click gives Show / Quit
- [x] `tauri-plugin-single-instance` — a second launch brings the running window forward rather than starting a copy
- [x] Tray failure degrades instead of killing startup (`setup_tray_or_warn` + `TRAY_AVAILABLE`); no tray means closing genuinely quits, so the window can't hide beyond reach (`KNOWN-ISSUES.md` PI-005)

## Release engineering

- [x] CI — `cargo fmt --check` / `clippy` / `test --workspace` on Linux + Windows, plus `pnpm check` / `test` / `build`, on every push and PR
- [x] Release workflow — `v*` tag builds bundles for Windows, Linux, and both macOS architectures
- [x] `CHANGELOG.md` (Keep a Changelog) and published releases through **v0.3.0**
- [x] Release workflow derives `prerelease` from the tag — a tag carrying a semver pre-release identifier (anything with a `-`) publishes as a prerelease, `v0.3.0` does not. It was hardcoded `false`, so tagging a beta would have shipped a release GitHub presented as stable. Releases stay drafts either way
- [x] A pre-release version must be **numeric-only** (`0.3.0-1`, not `0.3.0-beta.1`) — the Windows MSI bundler rejects the readable form, and only after the full release compile, so in CI it surfaces as three green platforms and one red. Verify with a local `pnpm tauri build` before tagging; see `CHANGELOG-infra.md` 2026-09-07
- [x] `LICENSE` (FSL-1.1-ALv2), `CLA.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `ROADMAP.md`, `docs/USAGE.md`, issue + PR templates
- [ ] Signed bundles and the tag → signed-bundle → GitHub-Release path (see `architecture.md` "Cross-app & updates")
- [ ] CI never *launches* the app — it compiles and tests the Linux target only. A green run says nothing about whether the window appears; PI-005 is what that gap looks like in practice. A smoke launch under a virtual display would close it.

## Open (features)

- [ ] **NEXT — re-detect sweep when the detector set gains a kind.** Detection
  results are persisted, so a project registered before a detector existed
  carries an incomplete tracker set forever and nothing says so. This lands the
  day the **Unity detector** ships, not the day plugins do: every existing
  project would silently lack its Unity tracker until manually refreshed. Run
  the one new detector across existing projects —
  `DetectorRunner::inspect(path, only)` is already that primitive. Best-effort
  (`trackers()` + logged errors, **never** `into_result()`, which is
  all-or-nothing and correct only for a single project). Small, and a
  hard prerequisite for shipping any new detector rather than an initiative of
  its own — it does not gate the scanner, but the scanner makes it matter more:
  registering two hundred projects before Unity ships turns five stale records
  into two hundred. See `architecture.md` → *Detection semantics*.
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
- [ ] **`ensure_project` and `ensure_project_named` disagree about binned projects.** `ensure_project_named` skips soft-deleted rows (so a binned-then-re-cloned directory is offered again by the scanner), but `ensure_project`'s early `find_by_directory` check is unfiltered and returns the binned project. No caller today — `crates/cli` is a stub — but the observer CLI will hit it and resurrect binned projects where the GUI creates new ones. A one-line `!p.is_deleted` filter. Found by the 0.3.0 whole-branch review and parked deliberately.
- [ ] **`ScanFolderModal`'s settings effect can discard input.** It merges only `scanRoot` when `list_detector_kinds` resolves; `mode`, `depth` and `includeIgnored` are still replaced wholesale, so a setting toggled inside that window is silently reset. The window is one local IPC call, hence parked rather than fixed.
- [ ] **No automated coverage for `ScanFolderModal`.** The three-step scan dialog is the largest untested surface in the app; the 0.3.0 review's one Critical finding (an unclamped depth reaching a `u32`) lived there and was caught by reading, not by a test. Component testing is not set up in this repo — `vite.config.ts` runs vitest in the `node` environment and nothing mounts a component — so this is a tooling decision, not just a missing file.
- [ ] `GitInfo.contributors` (see above)
- [ ] Unity detector — add `Tracker::Unity` + a `UnityDetector` together (register in `detectors/registry.rs`)
- [ ] Blender detector — add `Tracker::Blender` + a `BlenderDetector` together
- [ ] macOS support — `list_installed_apps` returns empty, app-launch falls through to the generic opener path
- [ ] Global shortcut — plugin is registered but no shortcut is bound to any action
- [ ] Keyboard navigation between sidebar entries — the obvious first binding for that unbound shortcut, and deliberately **not** absorbed into the views work: it is its own roadmap item with its own scope (focus model, which keys, whether it wraps).
- [x] **Project views, colour, icons and groups** — shipped in two halves, backend then frontend. Two view modes (list / grid) over one `ProjectList`, each presentation taking its action set as a snippet prop — which is what lets the same layouts serve both the standard `···` menu and the Bin's Restore/Purge pair. A third, compact, was built and then removed: it differed from list only by dropping the tracker badges, which is not a mode. A **sidebar** — All / Favourites / groups / Ungrouped / Bin, every entry with a count — replaces the flat single column and retires `FavoritesModal` and `BinModal`, so there is one navigation system rather than two and the two modals' duplicated `SortControls` is gone. Inline collapsible bands were designed and rejected: a collapsed band hides projects with nothing on screen saying they exist, where a sidebar keeps the current selection visible (which is also why the selected view can be persisted and collapse state could not). For the same reason the sidebar is a fixed column and does **not** collapse to an icon rail — two groups may pick the same glyph, and an icon-only rail would show two entries you cannot tell apart. Group membership is exclusive; tags stay the non-exclusive mechanism. Colours are stored as **palette names** resolved through `@theme` tokens, so a future theme plugin recolours everything coherently, or as a colour-picker literal validated as exactly `#` + six hex digits — strict because the value reaches a `style` attribute. Projects and groups both accept either; an unknown name falls back to neutral and an unknown icon to the `folder` glyph, since `core` owns neither list. Carried the project's **first schema migration** (`user_version` 1→2) and the migration-fixture scaffold `architecture.md` had deferred. Custom icons are user-supplied SVGs — a trust boundary — sanitized in `core` on import and rendered **only** through `<img>` + a `data:` URI, two independent layers, so they cannot execute regardless of what the sanitizer misses. `viewState.ts` persists the selected view and view mode in `localStorage`, never `projects.db` (the devmon cross-app contract): a view whose group was deleted falls back to All, and Bin is never restored as the landing view. Spec: `docs/superpowers/specs/2026-09-05-project-views-grouping-design.md`; plans: `docs/superpowers/plans/2026-09-05-groups-backend.md` and `2026-09-06-project-views-frontend.md`.
- [ ] **Spec 2 — observer CLI.** Fill in `crates/cli`: `indexer <cmd>` wraps a real command, matches argv+cwd+exit against recognizers, records project facts through `ProjectService`. Plain subcommands too.
- [ ] **Updater fast-follows** (see `architecture.md` "Cross-app & updates"): `tauri-plugin-updater` wiring + `core::updates::latest_stable`, a dismissible GUI release-notification chip, CLI `self-update` + stderr hint, GUI on-demand minisign-verified CLI download, tag→signed-bundle→GitHub-Release CI.
- [ ] **Plain CLI subcommands** — `indexer list` / `show` / `add` / `open` / `untrack` over the existing `ProjectService` methods. Separable from the observer; the open question is the output contract (`--json` is a compatibility promise).
- [ ] **Deeper git support** — ahead/behind upstream, the last commit's author/date/subject, stash count, uncommitted-file counts instead of a bare `dirty` bool, submodules/worktrees/tags/LFS. All refs-and-config reads; only `contributors` needs a `revwalk` and stays deferred.
- [ ] **Other version-control systems — not first-party.** Mercurial, Subversion, Jujutsu, Perforce, Fossil are plugin territory (`ROADMAP.md` → Plugins), not work this project takes on. What makes that reasonable to ask of a contributor: detectors are independent, so a jj repo colocated with git correctly reports both, and the generic renderer means a backend-only plugin is already useful. Perforce is the likely first, since the Unreal detector already reads the configured provider from `SourceControlSettings.ini`.
- [ ] **UI plugins — themes and layout as config.** Data, not code: a config file in the app's config directory, the way a dotfile is. `src/app.css` already declares the whole visual system as `@theme` tokens, so a theme is an override set for those. Unblocked — nothing to sandbox, because there is nothing to execute. Two requirements: a malformed file must fall back to the built-in values rather than stop the app starting, and token values must be parsed to their expected type rather than interpolated into CSS as strings. Layout is the half where the declarative rule will come under pressure; holding it is what keeps this category cheap. **Brainstormed 2026-09-05, then parked — revisit once the view/grouping work above has landed.** Two decisions were settled before parking: a `themes/` **directory** of TOML files plus a stored active-theme selection, not a single `theme.toml`; and **every** `@theme` token is overridable rather than a curated subset, which deliberately trades keeping internal names refactorable for author flexibility — the token names, the seven identical `--radius-*` aliases included, are now a compatibility surface. **Three more settled 2026-09-07.** *Restart to apply* (§5.4) — a file watcher is a background failure mode bought for convenience on an action taken rarely, and stays addable later. *Layout goes exactly as far as the current components already go* (§5.3) — a theme selects among existing behaviour and introduces none; anything it cannot express against the code as it stands is an issue and first-party work, which is a stricter line than "declarative only" and needs no argument to hold. *The active-theme selection lives in `tauri-plugin-store`, owned by the frontend* (§5.11) — presentational state belongs with the layer that applies it, and not in `projects.db`, which is the devmon contract; note the plugin was removed in `70278a2` and has to be re-added, and that its async JS API means a flash of the default palette unless the window stays hidden until the theme is applied. Nothing about the file format or the token surface is open any more. Tracked as issue #1. Briefing: `docs/handoffs/2026-09-04-plugins.md` §5.
- [ ] **Utility plugins — capability, in three shapes.** Backend-only (a `Detector`; already useful alone, since the generic renderer displays unfamiliar trackers), frontend-only (a feature over data the backend already returns), or both. Only the shapes with a frontend half are gated on containment: the CSP is in place, but capability scoping and a host API that replaces raw `invoke` are not, and choosing between them depends first on whether a frontend plugin is an imported module or its own webview — which is not reversible once an API ships. **That decision is deferred deliberately (2026-09-07): build backend-only plugins first and revisit once somebody has wanted a frontend one.** Backend plugins need no containment and the generic renderer already displays what they produce, so nothing is blocked by waiting. The first real work is elsewhere anyway — `Tracker` is a closed enum, and a plugin that produces a tracker kind the app was not compiled to know about is what opening it means. **Distribution settled 2026-09-07** (§5.7): a third-party detector is downloaded as WebAssembly, compiled by CI at publish time and run under Wasmtime with the project directory preopened read-only — Zed's and Lapce's model. Recompiling on the user's machine was rejected: it charges every user a Rust toolchain and a multi-gigabyte build tree, and breaks the macOS app signature on rebuild. Prerequisites are opening `Tracker` and relaxing `Detector::kind()` from `&'static str` to `&str`; the runner is unaffected, and first-party detectors stay native because the git one needs libgit2. A runtime loader for native plugins is declined outright, which is why the licence needed an **Additional Permission** (`LICENSE`) allowing a build that differs from a release only by added plugins to be distributed — without it a backend-plugin author could publish a crate and nothing anyone could install. A plugin in its own repository is not a contribution, so the CLA does not reach it. Tracked as issue #1. See `ROADMAP.md` → Plugins and *Plugins and the licence*.
- [ ] `detect_project_trackers` command / `detectProjectTrackers` in `src/lib/api/projects.ts` — no frontend callers since Task 8 (name pre-fill moved to `suggest_project_name`). Kept deliberately: the command is still registered for a future CLI preview / Spec 2. Remove the JS wrapper if it's still unused when the API surface is next revised.

Non-feature work (testing, platform seams, tech debt, lockfiles) lives in `architecture.md`.
