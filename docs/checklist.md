# Project Indexer — Checklist

What's done and what's still open. Check items off as they land; add new ones under "Open" as they come up. See `accomplishments.md` for the dated story of how the checked items got done, and `knowledgebase.md` for how the finished pieces actually work.

## Git tracker

- [x] `GitInfo` model
- [x] `Gitector` connected to project detection (`DetectorRunner::default()`)
- [x] `Tracker::Git` populated end to end (`create_project` / `refresh_project_trackers`)
- [x] Git info exposed to the frontend (`TrackerBadges` + full field list in `ProjectDetailModal`)
- [x] `Gitector` unit tests (9 — detect/no-repo, unborn HEAD, committed HEAD, dirty, remote URL, branches, detached HEAD)
- [ ] `GitInfo.contributors` — deliberately deferred, still `Vec::new()`. Planned: `git2::Repository::revwalk()`, field becomes `Vec<Contributor { name, email }>` rather than plain strings. Deferred over the cost of walking full history on every detection run.

## Unreal tracker

- [x] `UnrealInfo` model
- [x] `UnrealError` type
- [x] `UnrealDetector`
- [x] `find_project_file()` (immediate `.uproject` lookup, not upward-discovered like git)
- [x] `.uproject` JSON parsing (engine association, category, description, modules, enabled plugins)
- [x] Source-control provider detection (`SourceControlSettings.ini`)
- [x] `Detector::detect()` / `Detector::get_info()`
- [x] Returns `Tracker::Unreal(UnrealInfo)`
- [x] `UnrealDetector` unit tests (9)

## Detection UX

- [x] Browse-to-prefill: suggest a project name from the picked directory (git remote repo name, else folder name)
- [x] `detect_project_trackers` command (preview detection before a project exists)
- [x] `ProjectDetailModal` — project identity + one tab per detected tracker, generically rendered (`lib/trackers.ts`) so a future detector needs no new frontend code

## Open

- [ ] `GitInfo.contributors` (see above)
- [ ] Unity detector — `Tracker::Unity` is still a bare unit variant
- [ ] Blender detector — `Tracker::Blender` is still a bare unit variant
- [ ] macOS support — `list_installed_apps` returns empty, app-launch falls through to the generic opener path
- [ ] Global shortcut — plugin is registered but no shortcut is bound to any action
- [ ] Frontend automated tests — `svelte-check` is the only frontend verification
- [ ] Command-layer integration tests — `commands/projects.rs`/`commands/system.rs` are only exercised indirectly today
- [ ] `PI-004` (`docs/KNOWN-ISSUES.md`) — NVIDIA DMABUF-workaround comment in `lib.rs` understates its own trigger scope; code is fine, wording needs a fix
- [ ] Reconcile `package-lock.json` vs `pnpm-lock.yaml` — both present since early history, pick one
