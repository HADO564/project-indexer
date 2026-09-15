# Command-line tool — design

**Date:** 2026-09-14
**Status:** architecture and organisation settled; implementation not started.
**Builds on:** [`docs/handoffs/2026-09-04-observer-cli.md`](../../handoffs/2026-09-04-observer-cli.md)
(what exists, and the questions it left open) and the 2026-09-02
frontend-agnostic core spec, whose *CLI — updates* and *GUI installs the CLI on
demand* sections this document supersedes.

The CLI is the second frontend over `indexer-core`. It records projects as you
create them, exposes most of what the desktop app does as commands, and offers a
keyboard-driven TUI over those commands — and it works whether or not the app is
installed.

This document is the contract for how the CLI is **organised and structured**.
The per-feature detail of each command, the observer's recognizers and the TUI's
keymap are settled milestone by milestone, and recorded here as they are.
Product-level plans live in [`docs/cli/ROADMAP.md`](../../cli/ROADMAP.md).

## Decisions locked

1. **Same repository, its own crate.** The CLI lives in `crates/cli`, a member of
   the existing Cargo workspace — not a separate repository.
   *Why:* nearly everything the CLI does is a call into `indexer-core`, and both
   frontends open one SQLite file whose schema core owns. In one repository a
   schema change, the migration and both consumers land in one pull request
   under one CI run. Across two, every core change becomes a tag, a dependency
   bump and a second pull request, with nothing to stop core breaking the CLI.
   A TUI that must agree with the GUI also forces logic out of the frontend and
   into core (see *Logic that moves into core*), which is one change here and
   two coordinated ones otherwise.
   *Revisit when:* `indexer-core` becomes a published library with its own
   semver promise, or the CLI gains maintainers separate from the app's.
   Splitting later with `git filter-repo` keeps the history; merging back is the
   harder direction.

2. **Released independently.** The CLI has its own version (starting at `0.1.0`),
   its own changelog ([`crates/cli/CHANGELOG.md`](../../../crates/cli/CHANGELOG.md))
   and its own tags, `cli-v<version>`. The app keeps `v<version>`. See
   *Versioning and releases*.

3. **The observer is core scope.** `indexer git init` running the real command and
   recording what it created is what makes the CLI more than a second GUI. It is
   not a later add-on.

4. **Plain subcommands replicate most of the GUI, and ship first.** They are the
   layer the observer and the TUI both run through, so they are built before
   either.

5. **Every change in the TUI goes through a command.** Panes, keybinds and a `:`
   command line, in the manner of LazyVim. Letter shortcuts (`n` to add, `o` to
   open) and a per-project action menu (on a key, and on right-click where the
   terminal reports it) are shortcuts that run or pre-fill commands — no forms,
   buttons or dialogs, and nothing that changes data without a command behind
   it. Anything that needs typing happens on the `:` line. *Revised 2026-09-15
   from "view-only", which ruled out the shortcuts and the menu along with the
   forms.*

6. **Distribution is through package managers, and comes after the package.**
   Homebrew, winget and similar; users never download a binary by hand.
   `indexer self-update` and the GUI downloading the CLI are dropped: both
   duplicate or fight the package manager. *Added 2026-09-15:* the desktop app
   also provides `indexer` **from its own binary** — nothing downloaded, no second
   copy bundled — so a GUI-only install still has the command, for people and AI
   assistants alike. Planned in
   [`docs/handoffs/2026-09-15-gui-provides-indexer.md`](../../handoffs/2026-09-15-gui-provides-indexer.md).

7. **`indexer` is a placeholder binary name.** It is set in one place,
   `[[bin]] name` in `crates/cli/Cargo.toml`, and will change.

8. **Documentation is split per product, with one overview.** The root
   `ROADMAP.md` highlights both products and holds what they share;
   `docs/app/ROADMAP.md` and `docs/cli/ROADMAP.md` hold each product's detail.
   Release-facing files live with what is released: the app's `CHANGELOG.md`
   at the repository root, the CLI's in `crates/cli/`.

## Architecture

### The workspace

```
crates/
  core/        indexer-core — domain, orchestration, persistence. No Tauri, no CLI.
  cli/         indexer-cli — the `indexer` binary: commands, observer, TUI
src-tauri/     the desktop app
src/           the app's SvelteKit frontend
```

**Dependency direction, enforced by Cargo:**

- `cli → core`. Never `core → cli`.
- `cli` never depends on `src-tauri`, and `src-tauri` never on `cli`. Anything
  both need belongs in `core`.
- The existing rule stands: `core` cannot `use tauri`. The CLI is the second
  frontend that rule was written for.

All three crates are `publish = false`. Nothing is on a registry; installs come
from package managers or `cargo install --git`.

### Inside `crates/cli`

```
crates/cli/src/
  main.rs            no arguments → the TUI; otherwise parse argv and run one command
  paths.rs           locates the shared projects.db
  context.rs         opens the database and builds the core services once
  commands/          one module per command: definition + execution
    mod.rs             the Command enum; run(command, &Context) -> Result<Outcome>
  output/            renders an Outcome for a shell: human tables, or the --json envelope
  observe/           the observer: spawn the wrapped command, then recognizers
  tui/               the TUI
    app.rs             state: current view, selection, query, mode
    keys.rs            normal-mode keybinds, including shortcuts that run or pre-fill commands
    cmdline.rs         the `:` line — parsed into the same Command the shell uses
    ui/                panes, and the per-project action menu
```

**Scaffolded 2026-09-14.** The whole layout exists up front, rather than each
module appearing at the milestone that first needs it: every command, the
observer and the TUI start as stubs returning "not implemented", so each
milestone fills in a module instead of inventing one. `launcher.rs` (a
placeholder `AppLauncher`) and `confirm.rs` (the confirmer) sit beside
`context.rs`. Tests follow the repository rule — beside the tree, under
`crates/cli/src/tests/`, mirroring the module paths.

The TUI is a module of the CLI crate rather than a crate of its own, because a
TUI whose every action is a CLI command is inseparable from the command layer. Split it out only if it grows a consumer other than the `indexer` binary.

### One command layer, three entry points

```
 shell argv ─┐
             ├─► Command ─► run(command, &Context) ─► Outcome ─┬─► output/ (stdout)
 TUI `:` line┘                                                 └─► TUI message line + redraw

 observer ─► spawn real command ─► recognizers ─► core services (ensure_project, refresh…)
```

- **A command is defined once.** The shell and the TUI's `:` line parse into the
  same `Command`; the TUI splits the typed line into words and hands them to the
  same parser. `:open` and `indexer open` are one command, not two that agree.
- **`run` returns a typed `Outcome` and prints nothing.** Formatting belongs to
  the caller: `output/` for a shell, the message line for the TUI. This is what
  keeps behaviour identical in both.
- **`Context` carries the services and, in the TUI, the selection.** A command that
  needs a project takes it explicitly in a shell and defaults to the selected one
  in the TUI.
- **Confirmation is an interface, not a prompt.** Destructive commands
  (`delete_directory` above all) ask through a confirmer the caller supplies: a
  stdin prompt or `--yes` in a shell, a `y/n` on the command line in the TUI.
- **The observer records through core services, not through `Command`.** It is
  not a user typing a command; it is facts inferred after one ran.
- **Behaviour lives in core.** A command that needs more than a call or two into
  `ProjectService`, `GroupService` or `ScanService` is a sign the logic belongs in
  core.

### The database

- **One file, shared.** The CLI must open the file the app opens —
  `app_config_dir()/projects.db` under the identifier `com.shaer.project-indexer`.
  Without Tauri, `paths.rs` derives it (almost certainly
  `dirs::config_dir().join("com.shaer.project-indexer")`) and a test pins it
  against the app's path on each platform. A mismatch fails silently — two
  databases, neither tool seeing the other's projects — which is why the test is
  not optional.
- **The version-skew guard is surfaced, never swallowed.** `SqliteRepository::open`
  refuses a database written by a newer schema; the CLI prints that message and
  exits non-zero.
- **Concurrency is already handled.** Every connection sets WAL and
  `busy_timeout = 5000`; the app and the CLI reading and writing at once is
  supported.
- **The TUI notices other writers** by polling `PRAGMA data_version`, which
  changes when another connection commits. No file watcher.

### Opening projects

`ProjectService::open` needs an `AppLauncher`. The app's `OpenerLauncher` lives in
`src-tauri` and uses `tauri-plugin-opener`, so the CLI cannot reuse it. Most of the
logic is already Tauri-free in `indexer_core::platform` (`open_with_command`,
`open_with_app_available`). Decide at the `open` milestone whether a Tauri-free
launcher moves into `core::platform` for both frontends, or the CLI carries its
own adapter; a stub returning an error is acceptable until then.

### Logic that moves into core

The GUI keeps some behaviour in TypeScript that the CLI and TUI must match
exactly:

| Today | What it decides |
|---|---|
| `src/lib/views.ts` — `resolveView`, `viewCounts`, `matchesQuery`, `isPropertyQuery`, `propertyKeys` | what each sidebar view contains, its count, and what a search matches, including `name: value` |
| `src/lib/scanSettings.ts` — `restoreScanSettings`, `toScanRequest` | scan defaults, depth clamping, dropping unknown detector kinds |

Each moves to Rust in `core` before the CLI command or TUI pane that needs it.
**The GUI switches to the core version in the same change**, or the two
implementations drift and a view means different things in different frontends.
UI-only state — `viewState.ts`, the selected view and view mode — stays in the
frontend that owns it.

### The observer

As in the handoff (§1): `indexer <cmd> [args…]` runs `<cmd>` with inherited stdio,
then matches argv, working directory and exit code against recognizers and
records inferred facts through core.

- **The wrapped command's exit code always wins.** A failure to record never
  changes it.
- **Recognizers live in `crates/cli/src/observe/`.** A `CommandObserver` trait in
  core is only worth it once something other than the CLI consumes recognizers.
- **Name collisions are settled:** `ensure_project` resolves them with
  `domain::naming::disambiguate`, the function the scanner uses.
- **Open, settled at the observer milestone:** the first recognizers
  (`git init`, `git clone`, `mkdir` are the candidates), the per-recognizer rule
  for which directory is the project, and what the human path prints — nothing,
  or one line on stderr.

### Output

- **The `--json` contract is settled** in
  [`docs/cli/ROADMAP.md`](../../cli/ROADMAP.md#the---json-contract): a versioned
  envelope, additive-only within a version, unknown tracker kinds serialise,
  stdout is data and stderr is prose.
- Human-readable output is the default. Errors go to stderr with a non-zero exit
  code, never into stdout.

## Versioning and releases

| | Desktop app | CLI |
|---|---|---|
| Version lives in | `src-tauri/Cargo.toml`, `crates/core/Cargo.toml`, `package.json`, `tauri.conf.json` | `crates/cli/Cargo.toml` only |
| Changelog | root `CHANGELOG.md` | `crates/cli/CHANGELOG.md` |
| Tag | `v<version>` | `cli-v<version>` |
| Release workflow | `.github/workflows/release.yml` (`v*`) | not yet — added with packaging |

- **An app release does not bump the CLI**, and a CLI release does not bump the
  app. `indexer-core` is internal and moves with the app's version.
- **Schema compatibility is the contract between them.** A CLI older than the
  schema the app migrated to refuses the database. So: **a schema bump ships with
  a CLI release that understands it.** When `CURRENT_SCHEMA_VERSION` changes, the
  pull request says so, and the CLI release follows it.
- **"Latest release" must be filtered by tag prefix** anywhere it is asked for —
  the app's planned updater included — because both products publish releases to
  one repository.
- **A CLI release workflow** builds only `-p indexer-cli` per target, with none
  of the app's WebKit or appindicator dependencies. It is added together with
  packaging, not before.

## CI

Today's CI runs `cargo fmt`, `clippy --workspace` and `test --workspace` on Linux
and Windows, which already covers `crates/cli`. Nothing changes yet. Later, and
only when there is CLI code to justify it: a CLI job on macOS (the app's CI never
builds macOS), and path filters so a CLI-only change need not rebuild the app.

## Milestones

1. **Foundations.** Filter binned projects out of `ensure_project`'s directory
   check (the parked checklist item); add `indexer-core` to `crates/cli`;
   `paths.rs` and its same-path-as-the-app test.
2. **First slice.** `indexer list` printing real rows from the real database.
   Proves the premise — same database, no backend change.
3. **Plain subcommands.** `show`, `add`, `open`, `untrack`, groups, the bin, scan,
   each returning an `Outcome`; human and `--json` rendering. `views.ts` and
   `scanSettings.ts` logic moves into core as these need it.
4. **The observer.** Spawn-and-passthrough, then the first recognizers.
5. **The TUI.** Panes, keybinds, the `:` line over the same commands, letter
   shortcuts and the action menu that run or pre-fill those commands,
   `data_version` refresh.

Packaging and package managers follow, as their own piece of work.

## Non-goals

- Package-manager distribution and a CLI release workflow — after the package
  works.
- `indexer self-update`, and the GUI downloading a separate CLI — superseded by
  package managers. (The app providing `indexer` from its own binary is planned;
  see decision 6.)
- An MCP server. `docs/cli/ROADMAP.md` → *Agent access* orders it: subcommands and
  `--json` first, then judge.
- Editing in the TUI through forms or dialogs.
- The running GUI reflecting CLI writes live. The TUI polls for changes; the GUI
  doing the same is separate work.

## Open questions

Settled at the milestone that needs each; recorded here when they are.

- Which recognizers ship first, and each one's project-directory rule.
- What the observer prints on the human path; whether `--quiet` exists.
- Where a Tauri-free `AppLauncher` lives.
- The TUI's keymap, and whether it has a help overlay or a hint line.
- Linux distribution channels, when packaging starts.
- The final binary name.
