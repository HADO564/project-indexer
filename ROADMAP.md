# Roadmap

Where Project Indexer is going, why, and — just as usefully — what has been
considered and ruled out. Nothing here carries a date. Items move when the work
that unblocks them lands, not when a quarter ends.

For fine-grained feature status see [`docs/checklist.md`](docs/checklist.md); for
the non-feature quality backlog see
[`docs/architecture.md`](docs/architecture.md).

## Where things stand

**v0.1.1** is the current release. The app tracks projects, detects git and
Unreal Engine trackers, opens projects in your installed applications, and runs
in the background from the system tray. The Rust backend has been restructured so
that all logic lives in `indexer-core`, a library crate the compiler forbids from
importing Tauri — which is what makes everything in the next section possible
without touching the backend.

Windows and Linux are both built and tested in CI. macOS builds in the release
workflow but is not yet functionally complete (see below).

## Next — the observer CLI

The single largest planned piece, and the one the last refactor was for.

`indexer git init` runs the real `git init`, untouched, propagates its exit code,
and *notices* what happened — then records the project through the same
`ProjectService` the GUI uses. It never reimplements the tools it wraps. Because
both frontends open the same SQLite database, installing the CLI later connects
it to the GUI with no pairing and no IPC.

The backend seams already exist: `ensure_project` and `find_by_directory` have no
GUI caller and were added purely for this, and `projects.directory_normalized` is
indexed so directory lookup is not a table scan.

What is *not* yet decided — deliberately — includes which commands are recognised
first, how the project directory is derived from arguments and working directory
per recognizer, what happens when a directory's inferred name collides with an
existing project, and whether plain subcommands (`indexer list`, `indexer open`)
ship alongside the observer or after it.

The full briefing, including the open questions, is in
[`docs/handoffs/2026-09-04-observer-cli.md`](docs/handoffs/2026-09-04-observer-cli.md).

## More detectors

Detection is designed so a new tracker costs no frontend code: implement the
`Detector` trait, register it in one place, add the type, and the UI renders it
generically. See [`CONTRIBUTING.md`](CONTRIBUTING.md#adding-a-detector).

- **Unity** — the next detector, and the one the generic path was built for.
- **Blender** — same shape.
- **Git contributors.** `GitInfo.contributors` exists but is deliberately always
  empty. Populating it needs a full history walk (`revwalk`) on every detection
  pass, so it is gated on the fast/deep detection split below rather than being
  bolted on.

Placeholder tracker variants without a detector behind them were removed once and
will not come back — a type is added together with the code that produces it.

## Platform completeness

- **macOS.** Installed-application discovery returns an empty list, so the "open
  with" picker has nothing to offer and launching falls through to the generic
  opener. This is also the natural moment to put `list_installed_apps` behind a
  trait: a third implementation is what makes that seam pay for itself, and until
  then a plain function is honest.
- **Global shortcut.** The plugin is registered but no shortcut is bound to
  anything. Deciding what it should *do* is the open part.

## Updates and distribution

Designed but not built. All of it is fast-follow work on top of the release
pipeline that already exists.

- `tauri-plugin-updater` wiring, with a shared `core::updates::latest_stable`
  helper so the GUI and CLI agree on what "latest" means.
- A dismissible in-app release notification, rather than an interrupting dialog.
- `indexer self-update` for the CLI, plus a throttled hint on stderr.
- An on-demand "download and install the CLI" action in the GUI, minisign
  verified, so the CLI need not be bundled with the installer.
- Tag → signed bundle → GitHub Release in CI.

The obligation this places on the current code — a safe schema-migration path, so
a newer binary opening an older database is routine rather than dangerous — is
already met: migrations are numbered `user_version` steps and `open` refuses a
database written by a newer binary.

## Deferred — gated on a trigger, not a date

These are not "someday". Each has a specific condition that should start it.

- **Fast vs deep detection tiers.** Split cheap marker detection from opt-in deep
  inspection, with a cache keyed on directory and HEAD. Trigger: the first
  detector that genuinely needs expensive work — git contributors, or dependency
  parsing.
- **Migration fixtures.** A `fixtures/` scaffold that seeds a database at
  `user_version = N` and asserts the result of each step. Trigger:
  `CURRENT_SCHEMA_VERSION` going to 2.
- **Structured detection logging.** Low value at two to six detectors. Trigger:
  detection getting slow enough to need debugging.
- **Frontend page-state extraction.** `+page.svelte` is around 250 lines. Watch
  it; don't pre-split it.

## Considered and declined

Recorded so they are not re-proposed without new information. The full reasoning
for each is in [`docs/architecture.md`](docs/architecture.md).

- **Platform-aware case folding for directory identity.** It would prevent
  registering `C:\Foo` and `C:\foo` as two projects on Windows — a rare,
  self-correcting problem, since the user can see both. Not worth forking a
  simple, well-tested normalisation function across operating systems.
- **Detector metadata, priority, or short-circuiting.** Detectors are independent
  by design, so there is no contention to arbitrate. Revisit only if two
  detectors genuinely need to coordinate.
- **Encrypting `projects.db`.** It holds paths and labels, not credentials, and
  being readable by other local tools is a deliberate contract, not an oversight.

## Related work

**devmon** is a separate planned application — an activity tracker that attaches
`projects.db` read-only to attribute work to projects. It shapes nothing Project
Indexer must do, but it is why the `ProjectReader` port and the `meta` table
exist. The cross-app contract is a recorded decision; don't regress it.
