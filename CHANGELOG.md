# Changelog

All notable changes to Project Indexer are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] — 2026-09-03

### Fixed

- **The app could hang on startup, showing no window at all, when the project
  database failed to open.** The fatal-startup-error dialog was rendered with
  `tauri-plugin-dialog`'s `blocking_show()` from Tauri's `setup` hook. That
  plugin queues the dialog onto the main-thread event loop and then blocks the
  caller until it resolves — but `setup` runs on the main thread *before* the
  event loop starts, so the queued dialog could never run and the app deadlocked
  with no window and no message. This was reachable whenever the database
  couldn't be opened: a corrupt file, a permissions problem, a full disk, or the
  version-skew guard after installing an older build over a newer one. The
  dialog is now rendered synchronously and no longer depends on the event loop,
  and the message is also written to stderr so a terminal launch or a captured
  log records it even where no GUI is available.

- **A panic anywhere in the app could permanently break saving for the rest of
  the session.** The SQLite connection mutex propagated poisoning, so one
  unrelated panic left every subsequent read and write failing. A panic does not
  leave SQLite itself inconsistent — an in-flight transaction rolls back when its
  guard drops — so the connection is now recovered instead of being abandoned.

- **Auto-registering a project could not infer a name from its git remote.**
  `ensure_project` (the entry point a future CLI uses to register a directory it
  has just seen) skipped detection, so it always fell back to the folder name
  instead of using the repository name the GUI would have suggested.

### Changed

- `ProjectService` implements `Debug`, so types that hold one can derive it.
- Removed a dead frontend API wrapper (`detectProjectTrackers`) left over from
  moving project-name inference to the backend in 0.1.0. The underlying command
  is unchanged.

## [0.1.0] — 2026-09-02

First release.

### Added

- Track local project directories with a name, description, tags, notes, client,
  and a preferred "open with" application.
- Automatic project-type detection: **git** (branch, dirty state, remote and its
  browser URL, current commit, detached HEAD, branch list) and **Unreal Engine**
  (`.uproject` parsing for engine association, category, description, modules,
  enabled plugins, and the configured source-control provider).
- Per-project detail view with one tab per detected tracker, a live status strip
  for every registered detector, and per-tracker re-detection.
- Open a project in your editor of choice or in the system file explorer, with
  an app picker populated from installed applications.
- Favorites, tags, sorting, a recycle bin with restore, and "untrack" to forget
  a project without touching its directory.
- A marker for projects whose directory has been deleted or moved.
- System-tray icon: closing the window keeps the app running in the background,
  and the tray restores it or quits.

### Technical

- All domain logic, orchestration, and persistence live in `indexer-core`, a
  library crate with no Tauri dependency, so additional frontends (a CLI is
  planned) can be built on the same backend without changing it.
- Projects are stored in SQLite at `projects.db` in the platform config
  directory, with synchronous transactional writes.

[0.1.1]: https://github.com/HADO564/project-indexer/releases/tag/v0.1.1
[0.1.0]: https://github.com/HADO564/project-indexer/releases/tag/v0.1.0
