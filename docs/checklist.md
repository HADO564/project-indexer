# Project Indexer — Checklist

What's done and what's still open, feature by feature. Check items off as they land; add new ones under "Open" as they come up. See `accomplishments.md` for the dated story of how the checked items got done, `knowledgebase.md` for how the finished pieces actually work, and `architecture.md` for the non-feature quality backlog (invariants, detection semantics, testing, platform seams).

## Git tracker

- [x] `GitInfo` model
- [x] `Gitector` connected to project detection (`detectors/registry.rs`)
- [x] `Tracker::Git` populated end to end (`create_project` / `refresh_project_trackers`)
- [x] Git info exposed to the frontend (`TrackerBadges` + full field list in `ProjectDetailModal`)
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

- [x] `DetectorRunner::detect_project` is the single canonical detection operation
- [x] `Detector` trait is one method (`detect(&Path) -> Result<Option<Tracker>, DetectorError>`)
- [x] `detectors/registry.rs` — one place to register a detector
- [x] `DetectorRunner` in Tauri managed state; commands take `State<'_, DetectorRunner>`
- [x] `DetectorError::Other` catch-all so a new detector needn't touch the shared enum
- [x] Resilient detection — `Detection { trackers, errors }`; one detector failing doesn't discard the others
- [x] `refresh_project_trackers` checks directory health before detecting
- [x] Refresh all-or-nothing is a recorded decision (`architecture.md`) with a guard test, not incidental

## Detection UX

- [x] Browse-to-prefill: suggest a project name from the picked directory (git remote repo name, else folder name)
- [x] `detect_project_trackers` command (preview detection before a project exists)
- [x] `ProjectDetailModal` — project identity + one tab per detected tracker, generically rendered (`lib/trackers.ts`) so a future detector needs no new frontend code

## Open (features)

- [ ] `GitInfo.contributors` (see above)
- [ ] Unity detector — add `Tracker::Unity` + a `UnityDetector` together (register in `detectors/registry.rs`)
- [ ] Blender detector — add `Tracker::Blender` + a `BlenderDetector` together
- [ ] macOS support — `list_installed_apps` returns empty, app-launch falls through to the generic opener path
- [ ] Global shortcut — plugin is registered but no shortcut is bound to any action

Non-feature work (testing, platform seams, tech debt, PI-004, lockfiles) lives in `architecture.md`.
