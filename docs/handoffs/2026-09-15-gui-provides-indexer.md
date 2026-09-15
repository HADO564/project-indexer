# Handoff — the GUI provides its own `indexer`

**Date:** 2026-09-15
**Status:** ready to start; not blocked. Decided by the user, not yet designed in
detail — settle the open questions in §6 before building.
**Read first:** [`../superpowers/specs/2026-09-14-cli-design.md`](../superpowers/specs/2026-09-14-cli-design.md)
(decision 6 and *One command layer*) and [`../cli/ROADMAP.md`](../cli/ROADMAP.md)
→ *Distribution and releases*.

---

## 1. The goal in one paragraph

Someone who installs **only the desktop app** should still have the `indexer`
command — the plain subcommands, `--json`, the observer and the TUI — without
installing a second package. The app provides it from **its own binary**, not
from a second copy of the CLI bundled beside it. This matters most for AI
assistants: an agent on a GUI-only machine can then run `indexer show --json`
instead of having nothing to call.

## 2. Why the app's own binary, not a bundled sidecar

Measured on 2026-09-15 (release builds, macOS arm64, the workspace's default
release profile):

| | Stripped | gzipped |
|---|---|---|
| `indexer` today (commands, observer stubs, no TUI) | 3.64 MB | 1.73 MB |
| What a ratatui + crossterm TUI adds (probe with layout, table, list, popup, mouse) | +0.37 MB | +0.15 MB |
| The app's own downloads (v0.3.1) | — | 4–7 MB (`.dmg` 5 MB, `-setup.exe` 4 MB; AppImage 80 MB) |

Most of `indexer`'s size is `indexer-core` and bundled SQLite, which the app
already contains. A sidecar would ship that twice — about 1.7 MB more per
download, a third of the app's size. Serving `indexer` from the app's binary
costs almost nothing, and the app and its `indexer` can never disagree about the
database schema, because they are the same build.

The TUI is **not** split out into its own package: it adds about 0.15 MB
compressed. Put it behind a default-on Cargo feature (`tui`) so a build without
it stays possible.

## 3. What exists

- `crates/cli` (`indexer-cli`) is a **binary-only** crate: `src/main.rs` parses
  `Cli`, opens a `Context`, runs a `Command`, prints through `output/`, and maps
  errors to exit code 1. The observer and TUI are stubs.
- `src-tauri` (`project-indexer`, lib `project_indexer_lib`) — `src/main.rs` is
  just `project_indexer_lib::run()`, under
  `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`.
- Both depend on `indexer-core`; the GUI does not depend on `indexer-cli`.
- `tauri.conf.json`: `productName` `project-indexer`, identifier
  `com.shaer.project-indexer`, bundle `targets: "all"` (dmg, app, msi, NSIS
  exe, deb, rpm, AppImage).
- The CLI finds the same `projects.db` as the app (`crates/cli/src/paths.rs`,
  pinned by a test).

## 4. The shape of the work

1. **Give `indexer-cli` a library target.** Move what `main.rs` does into
   `pub fn main_from(args: impl IntoIterator<Item = OsString>) -> ExitCode` (name
   it as fits); the `indexer` binary becomes a one-line call. Nothing about the
   commands changes. The TUI goes behind a default-on `tui` feature.
2. **The app's binary dispatches before Tauri starts.** In `src-tauri`, decide
   at the top of `main` whether this run is the CLI, and if so call the CLI's
   entry point and exit — no `tauri::Builder`, no window, no webview. The usual
   test is the name it was invoked under (`argv[0]`'s file stem is `indexer`),
   busybox-style, so a link named `indexer` pointing at the app runs the CLI.
   Settle in §6 whether any other trigger is needed.
3. **"Install command-line tool".** An action in the app (menu or settings)
   that puts `indexer` on the user's `PATH`, pointing at the app's binary:
   - **macOS:** a symlink in `/usr/local/bin` (or `/opt/homebrew/bin`) to
     `Project Indexer.app/Contents/MacOS/project-indexer`; needs an admin
     prompt for `/usr/local/bin`. `argv[0]` is the link's name, so the dispatch
     sees `indexer`.
   - **Windows:** see the pitfall below; likely a small `indexer.exe` or a copy
     placed by the installer, plus a `PATH` entry.
   - **Linux:** the `.deb` / `.rpm` can install a `/usr/bin/indexer` symlink at
     package time. The AppImage cannot; decide whether it offers the action at
     all.
   - **Uninstall / remove** the link again from the same place.
4. **Docs.** Update the spec's decision 6 note, the ROADMAP's distribution
   section, both checklists, and `crates/cli/README.md`.

## 5. Pitfalls already found

- **Windows console.** Release builds of the app use the `windows` subsystem,
  so the process has no console: `indexer list` run from PowerShell would print
  nothing and the shell would not wait for it. Options: `AttachConsole
  (ATTACH_PARENT_PROCESS)` at the top of the CLI path (output works, but the
  shell prompt returns before the program finishes and the output lands after
  it), or a tiny separate console-subsystem `indexer.exe` that forwards to the
  real work. Test this first; it may decide the Windows design.
- **Startup time.** The app binary links WebKit / WebView2 / GTK. On Linux the
  dynamic loader resolves those libraries at start even if the CLI path never
  touches them. `indexer list` must stay fast — measure against the standalone
  `indexer` and set a budget (under ~100 ms is the bar for a command people run
  constantly).
- **An `indexer` already on `PATH`** — from Homebrew, winget or `cargo install`.
  The install action must detect it and say so, never overwrite it silently.
  Which one wins is the user's `PATH` order; say that in the message.
- **Versions.** The standalone CLI is versioned as `cli-v*`, the app as `v*`.
  `indexer --version` from the app's binary should make clear it is the app's
  build (e.g. `indexer 0.1.0 (project-indexer 0.3.1)`).
- **Single-instance / deep-link plugins** in the app must not intercept a CLI
  run; the dispatch happens before any plugin is registered.
- **macOS app translocation / quarantine.** A symlink into an app still in
  `~/Downloads` breaks once the app is moved; offer the action only from
  `/Applications`, or re-resolve the path each time.

## 6. Open questions — settle before building

- Is `argv[0]` the only trigger, or does `project-indexer <subcommand>` also run
  the CLI? (Passing arguments to the app is otherwise unused, but the
  single-instance plugin may forward them.)
- Windows: `AttachConsole` in the app binary, or a small console `indexer.exe`?
- Where the install action lives in the app's UI, and whether it is offered on
  first run.
- AppImage: offer nothing, or print the command to create the link?
- Does the standalone package-manager `indexer` still ship? (Expected yes — for
  people who never install the app — but confirm.)

## 7. Done when

- On a machine with **only the app installed**, after "Install command-line
  tool": `indexer list`, `indexer show <query>`, `indexer --json list` and
  `indexer --help` work from a terminal on macOS, Windows and Linux (deb/rpm),
  with correct output and exit codes.
- Launching the app normally still opens the GUI; running `indexer` never opens
  a window.
- The app's download grows by no more than a few hundred KB.
- `indexer list` via the app's binary starts within the budget set in §5.
- An existing `indexer` on `PATH` is detected and not overwritten.
- `cargo test`, `cargo clippy --workspace --all-targets` and CI pass; docs
  updated.
