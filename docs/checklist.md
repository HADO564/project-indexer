# Project Indexer — Checklist

What's done and what's still open, feature by feature. Check items off as they land; add new ones under "Open" as they come up. See `accomplishments.md` for the dated story of how the checked items got done, `knowledgebase.md` for how the finished pieces actually work, and `architecture.md` for the non-feature quality backlog (invariants, detection semantics, testing, platform seams).

## Git tracker

- [x] `GitInfo` model
- [x] `Gitector` connected to project detection (`detectors/registry.rs`)
- [x] `Tracker::Git` populated end to end (`create_project` / `refresh_project_trackers`)
- [x] Git info exposed to the frontend (`TrackerBadges` on the list, full field list in `TrackerPanel` on `/project/[id]`)
- [x] `Gitector` unit tests (8 — recognizes-repo/plain-dir, unborn HEAD, committed HEAD, dirty, remote URL, branches, detached HEAD)
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
- [x] `UnrealDetector` unit tests (9)

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

## Open (features)

- [ ] `GitInfo.contributors` (see above)
- [ ] Unity detector — add `Tracker::Unity` + a `UnityDetector` together (register in `detectors/registry.rs`)
- [ ] Blender detector — add `Tracker::Blender` + a `BlenderDetector` together
- [ ] macOS support — `list_installed_apps` returns empty, app-launch falls through to the generic opener path
- [ ] Global shortcut — plugin is registered but no shortcut is bound to any action

Non-feature work (testing, platform seams, tech debt, PI-004, lockfiles) lives in `architecture.md`.
