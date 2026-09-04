# Changelog

All notable changes to Project Indexer are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **A content security policy.** The app shipped with `"csp": null`, which was
  survivable only because every script, font and icon is bundled. There is now a
  real policy: SvelteKit emits the strict half (its inline boot script is hashed
  automatically on each build) and `tauri.conf.json` carries a complementary one,
  with the browser enforcing the intersection. Nothing may be fetched from the
  network, `object-src` and `form-action` are off, and only the IPC bridge is
  reachable over `connect-src`.
- **A pre-commit hook** at `.githooks/pre-commit`, installed with
  `git config core.hooksPath .githooks`. It mirrors the CI gates exactly and runs
  only the ones the staged files can affect.

### Changed

- **The delete dialog no longer pre-selects deleting the folder from disk.** It
  now opens on "just remove it from this app", the destructive option is second
  rather than first, and the confirm button says which of the two it will do
  instead of always reading "Delete".

### Fixed

- **A tray-icon failure stopped the app from starting at all, with no window and
  no message.** The tray is built during startup, and any failure there was
  propagated out of Tauri's `setup` hook into `run()`'s `.expect(..)`, killing
  the process before a window existed. On Linux this was reached simply by not
  having an appindicator library installed — `libappindicator-sys` panics rather
  than returning an error when it cannot load one, so the failure bypassed error
  handling entirely — and the app's Arch install instructions were missing that
  package. On Windows the same path was reachable, less often, when
  `Shell_NotifyIcon` fails. A tray failure is now caught and reported with the
  package to install, and the app keeps running without a tray. Because closing
  the window normally hides it to the tray, closing now genuinely quits when
  there is no tray to restore from, rather than hiding the window beyond reach.

### Documentation

- The Arch package list in the README gained `libayatana-appindicator`, which the
  system tray loads at runtime; the Debian and Fedora lists already covered it.
- Noted that the app must be run via `tauri dev` or `tauri build` — a plain
  `cargo build` produces a binary that expects the Vite dev server and reports
  "Connection refused" when launched on its own.
- `PI-004` is fixed and retired from the known-issues log: the NVIDIA workaround's
  comment claimed both probe paths came from the proprietary kernel module, when
  the open module creates them too — which is the intended behaviour, since the
  open module still uses the userspace GL stack with the GBM failure.
- The roadmap now treats version-control systems beyond git as plugin territory
  rather than first-party work, describes plugins as two shapes (a UI plugin over
  the existing backend, or a UI plugin paired with a Rust detector), records the
  trust boundary each shape has, declines a runtime loader for native plugins,
  and settles the CLI's `--json` compatibility contract.
- `PI-006` records that `tauri build` cannot produce an AppImage on Arch — two
  unrelated incompatibilities in linuxdeploy and its GTK plugin, neither caused by
  this project. The binary, `.deb` and `.rpm` all build; published AppImages come
  from CI's `ubuntu-22.04` runner and are unaffected.
- The observer-CLI handoff has been brought current: the Linux target has now
  been run rather than merely compiled, the clippy baseline is one warning rather
  than two, test counts are given per platform, and the commit trailer is
  described by rule instead of naming one model.

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

Built but never published: the startup hang fixed in 0.1.1 was found before
these installers were released, so 0.1.1 is the first version users can install.
The feature set below is what 0.1.1 ships.

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

[Unreleased]: https://github.com/HADO564/project-indexer/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/HADO564/project-indexer/releases/tag/v0.1.1
[0.1.0]: https://github.com/HADO564/project-indexer/tree/v0.1.0
