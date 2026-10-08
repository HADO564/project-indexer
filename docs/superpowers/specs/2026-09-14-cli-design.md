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
   *Revised 2026-09-27 — one form, for editing.* `indexer edit <project>` with
   no field flags opens a full-screen form of the project's fields, as the GUI
   shows them. The form is an input method for the `edit` command, not a
   second write path: saving it makes the same single `update` the flags make.
   It is built **after** the flags (`--description`, `--add-tag`, `--set`, …),
   which scripts, agents, `--json` and the `:` line still need. With stdin not
   a terminal, or under `--json`, a bare `edit` stays a usage error. Every
   other change is still a command with no form in front of it.
   *Widened 2026-09-29:* the form is **human centric** — a way for a person to
   see their project at a glance and change the fields they want — and never
   the only way to change a field. `edit app` opens a compact form
   (description, tags, properties); `edit app --full` opens every field the
   GUI's edit view has, favourite as a checkbox. Each field the full form adds
   gains its `edit` flag in the same step, so nothing the form changes lacks a
   command-line way to change it (favourite's is the `favorite` /
   `unfavorite` verbs).

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

**Decided 2026-10-08, at the start of the observer milestone** (`feat/observer`):

- **Sequential, in three phases; no threads.** The wrapped command is already a
  separate process, so dexily only waits while it runs. Running the recording
  alongside it was considered and rejected: what to record — whether a project
  exists, and where — is only known once the command has finished (a failed
  `git clone`, a Ctrl+C halfway), recording is milliseconds against the
  command's seconds, and reading the command's output would mean piping it,
  which costs it its colours, progress bars, prompts and clean Ctrl+C.
  1. **Before** — read argv and the disk and form an *expectation*, held in
     memory only: which kind of command this is, which folder should hold the
     project, and whether that folder exists yet. Nothing is written.
  2. **Run** — the command, with inherited stdio, untouched; its exit status.
  3. **After** — check the evidence: the exit code, that the expected folder
     now exists (and was not there before, where that matters), what is inside
     it. Only if the signs agree does dexily call `ensure_project`, the one
     write. A failed command leaves nothing behind: no phantom projects.
- **dexily sees the outcome, not the output.** The command's text goes straight
  to the terminal and is never read; the exit code and the filesystem are the
  evidence, and are steadier than text that changes between tool versions.
- **Details come from the folder, not the command line.** Core's detectors fill
  in the kind, branch, remote and name when `ensure_project` runs, exactly as
  for the GUI's Add, so the two can never describe one repo differently. The
  command line's job is narrower: which folder, and whether this command makes
  projects at all — plus anything only it knows (`git clone --branch dev`).
- **A before/after snapshot** of the folder is a recognizer technique worth
  having: what is new after the command is what it created, which covers a
  custom target folder or a tool whose output is hard to guess from its
  arguments. A live filesystem watcher is not needed.
- **Ctrl+C:** dexily ignores it while the command runs and lets the command
  decide, so it still reports the command's real exit code; on Unix a command
  killed by a signal has no exit code, which dexily has to map to one.
- **dexily's own flags set the new project up in the same breath:**
  `dexily --add-tag rust --group Work --notes "demo" git clone <url>`.
  - **They go before the command, never after.** Everything from the command's
    name on belongs to it untouched — many commands share flag names with
    dexily (`--name`, `--color`, `--set`, `-t`), so dexily never guesses whose
    a flag is. clap already hands the rest over as-is.
  - **They are `edit`'s flags,** with the same names, rules and checks
    (`--name`, `--description`, `--add-tag`, `--set`, `--notes`, `--group`,
    `--color`, `--icon`, `--open-with`), applied after `ensure_project` as one
    `UpdateProject` built the way `edit` builds one. Not `--directory` or
    `--ungroup`, which mean nothing for a new project. Trackers are detected,
    never set, so there is no flag for them.
  - **They are checked in phase 1, before the command runs.** A group that
    does not exist or a bad colour stops everything with exit 2 and the
    command never runs — all or nothing, rather than a clone left behind with
    a half-recorded project. Likewise flags on a command no recognizer knows
    (`dexily --add-tag rust ls`): nothing to apply them to, so it says so and
    does not run.
  - **Still to decide in the plan:** whether they apply when the folder was
    already tracked (leaning yes — they were asked for; only the create is
    skipped), and what dexily says when the command failed and they were
    dropped.
- **The first recognizers are system commands, starting with `mkdir`;**
  tracker-based ones (`git init`, `git clone`, Unreal) come after. The `dexily`
  prefix is the signal: a folder made with `dexily mkdir` is a project, with no
  "is this really a project?" guessing. The rule: every folder named in the
  command that mkdir actually created — not `-p`'s parents, not an option's
  value (`-m 755`) — and only folders absent before and present after.
  `mkdir` is a shell built-in on Windows, not a program; how to run it there is
  still to choose (`cmd /C`, dexily making the folder itself, or Unix-only for
  now). Later system commands: `mv` (update a tracked project's directory),
  `rm -rf` (a tracked folder gone), `cp -r` (a copy as a new project).
- **One prompt confirms and opens the form** (decided 2026-10-08), in phase 3
  once the command has succeeded: `dexily: track "friction" at
  ~/code/friction? [Y/n/e]`. Enter or `y` tracks; `n` leaves an ordinary folder,
  nothing to clean up; `e` tracks, then opens the edit form (`TerminalEditor`,
  as `edit --full`) on the new project — Esc there keeps the project and skips
  the details. The default is yes, so the common case is one keystroke and the
  prefix's intent is not second-guessed. dexily's own flags apply before the
  form, so it opens showing them. No prompt and no form when piped, in a
  script or under `--json` — it tracks, as the prefix says — or with `--yes`.
  The confirmer needs a third answer for `e`. **Still to decide:** a `config`
  setting to turn the prompt off.
- **Settled 2026-10-08, one question at a time:**
  1. **Windows:** built-ins run through `cmd /C` (`cmd /C mkdir thing`), so
     dexily still runs the real command everywhere. The program is looked up
     first (the `which` crate); a known `cmd` built-in (`mkdir`/`md`, `move`,
     `copy`, `del`, …) goes through `cmd /C`, which also covers `npm.cmd`-style
     scripts. cmd's `mkdir` makes parents itself and has no `-p` or `-m`, so the
     folder rule has a Windows variant. Folder names with spaces need
     `CommandExt::raw_arg` and a test of their own.
  2. **Several folders** (`mkdir a b c`): one `[Y/n/e]` prompt per folder, so
     each answer, `e` included, is about one project. dexily's flags apply to
     every folder said yes to.
  3. **Flags given:** no prompt. Flags mean the user has already decided and
     described the project; dexily tracks and applies them. The prompt is for
     the bare `dexily mkdir thing`.
  4. **Output:** one line on stderr when no prompt was shown —
     `dexily: tracking "thing" at ~/code/thing` — and nothing more after a
     prompt the user just answered. A failed command:
     `dexily: mkdir failed (exit 1), nothing tracked`. `--quiet` hides dexily's
     own lines, never the command's output and never a warning. `--json`: the
     command keeps stdout, so dexily's document goes to **stderr**, as one line,
     printed last, after the command has finished — the command's own errors
     share stderr, and a script reads the final line. A `--json-file <path>` is
     the upgrade if that proves awkward.
  5. **Recording fails** after the command succeeded: the command's exit code
     stands, and a warning says why and how to retry —
     `dexily: could not track "thing": database is locked` then
     ``run `dexily add ~/code/thing` to try again`` — shown even with `--quiet`.
  6. **Tests, four layers:** recognizer rules on argv alone (no processes);
     evidence checks with temp folders and made-up exit codes; `spawn` running
     tiny real commands (`true`, `false`, `cmd /C exit 3`) for exit-code
     passthrough, a missing program, and a signal as 128 + n on Unix; and end
     to end with an in-memory database and a stand-in prompt — a flag that does
     not check out stops the command before it runs, a failed command records
     nothing, a recording failure keeps exit 0.
- **Comes back with `git init`:** whether dexily's flags apply to a folder that
  was already tracked. `mkdir` cannot meet it — its folders are always new.
- **Security (decided 2026-10-08).** dexily runs a command as the user, with
  the user's rights and no more, so passing a command through adds no power on
  its own; these rules keep it from adding risk.
  1. **An allowlist: dexily runs only commands it has a recognizer for.**
     Anything else is refused before it runs, exit 2:
     ``dexily: `ls` isn't a command dexily observes (it knows: mkdir). Run it
     directly.`` An unrecognised command gains nothing from dexily; refusing it
     means an AI agent or script given dexily cannot use it to run arbitrary
     programs, and no command "works" while recording nothing. The allowlist
     *is* the list of recognizers, so the two cannot drift; for a command with
     subcommands only the recognised ones pass (`git init`, `git clone` — not
     `git push`).
  2. **Never a text line, never a shell.** `Command::new(program).args(list)`
     hands each argument over separately with no shell, so
     `dexily mkdir 'x; rm -rf ~'` makes one oddly named folder. Arguments are
     never joined into a string.
  3. **Windows `cmd /C` is a shell, so it is guarded:** an argument containing
     any of cmd's special characters (`&`, `|`, `<`, `>`, `^`, `%`, `"`) is
     refused before anything runs — `dexily mkdir "a & calc"` would otherwise
     make `a` and then start Calculator. Tests of its own.
  4. **The right program:** looked up on `PATH` only, never in the current
     folder, where a planted `mkdir.exe` could be picked first; `cmd.exe` is run
     by its full system path (`%SystemRoot%\System32\cmd.exe`), never looked up.
  5. **The recorded folder is resolved** to its real absolute path before it
     is tracked, as `add` does, so a symlink or `..` cannot point dexily at an
     unexpected place.

  Phase 1 therefore opens with a gatekeeper, before anything runs: is the
  command on the allowlist; do dexily's flags check out; on the `cmd /C` path,
  are the arguments free of special characters — any "no" is exit 2 with the
  command not run. Only then is the expectation formed.

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
- Editing in the TUI through forms or dialogs — except the one `edit` form of
  decision 5 (revised 2026-09-27).
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
