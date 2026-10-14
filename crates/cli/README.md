# dexily

The command-line tool for Project Indexer. It records projects as you create them
(`dexily git init` runs the real `git init`, then registers the repository),
exposes most of what the desktop app does as commands, and offers a
keyboard-driven TUI. It opens the same database as the app, so the two stay in
step with no pairing — and it works without the app installed.

**Status: every plain subcommand works.** `list`, `show`, `add`, `open`,
`untrack`, `favorite`, `unfavorite`, `edit`, `restore`, `purge`, `scan` and
`config` all run against the app's database, and a bare `edit` in a terminal
opens a full-screen form (`edit --full` for every field). `group` lists,
creates, changes and deletes groups; a project joins one with `edit --group`.
**The observer works for `mkdir`, `git init`/`clone`, `cargo new`/`init`, `go mod init`, `gh repo create`, and `init` for the package/project managers (`npm`, `pnpm`, `yarn`, `poetry`, `uv`)**:
`dexily mkdir thing` makes the folder and tracks it, and
`dexily git clone <repo>` does the same for a repository. The TUI still returns "not implemented". The command is `dexily`.

```
dexily list                     every project, as a table (--view, -s name|last-opened, -r to flip)
dexily show <query>             one project — exact name, id, parent/folder, or part of a name
dexily add [dir]                track a directory, or the current one
dexily open <query>             open a project in its application
dexily untrack <query>          forget a project, leaving its files alone (asks first; --yes)
dexily favorite <query>         mark a project as a favourite; unfavorite clears it
dexily edit <query> --name, --directory, --description, --notes, --add-tag/--remove-tag,
                     --set k=v/--unset k, --open-with, --group/--ungroup, --color, --icon   change a project's fields
dexily edit <query>             with no flags, in a terminal: a form for the description, tags and properties
dexily edit <query> --full      the same form with every field, as the app's edit view has them
dexily restore <query>          bring a project's record back from the bin
dexily purge <query>            delete a binned project's record for good (asks first; --yes)
dexily scan <dir>               find projects under a folder; --import registers them
dexily config folder-color ...  the folder colour in that table; header-color likewise
dexily group list               the groups, with their colour, icon and project count
dexily group create <name>      a new group (--color, --icon; cyan and briefcase by default)
dexily group edit <group>       rename it (--name) or change its --color or --icon
dexily group delete <group>     delete a group, keeping its projects ungrouped (asks first; --yes)
dexily config form-wrap on|off  whether Tab wraps around at the ends of edit's form
dexily config icons nerd|emoji|off   icons before names in tables (off by default)
dexily mkdir <folders…>         run the real mkdir, then track each folder it made (asks [Y/n/e])
dexily git init [dir]           run the real git init, then track the repository
dexily git clone <repo> [dir]   run the real git clone, then track the repository it made
dexily --add-tag rust --group Work mkdir <folder>   the same, set up in one go — flags go before the command
dexily <anything> --json        {"schema": 1, "data": …} on stdout, or {"schema": 1, "error": …} on stderr
```

### Observing

Put `dexily` in front of a command and it runs the real one — same output, same
prompts, same exit code — then tracks what the command created. **Only commands
dexily has a recognizer for run at all**; today that is `mkdir` (`md` too on
Windows), `git init`/`clone`, `cargo new`/`init`, `go mod init`, `gh repo create`
(with `--clone`, tracks the repo it clones), and `init` for the package/project
managers: `npm`/`pnpm`/`yarn`/`poetry` **bare** (which only write a manifest),
and `uv init [path]` (whose positional is a folder, like `cargo init`'s).
Anything else is refused before it starts — `npm init <initializer>`, which
downloads and runs a package (`npm init vite` is `npx create-vite`), and every
other subcommand — so a script or an agent given dexily can never use it to run
arbitrary programs:

```
$ dexily mkdir friction
dexily: track "friction" at /Users/me/code/friction? [Y/n/e]

# Set the new project up in the same breath — project flags go *before* the command:
$ dexily --add-tag rust --group Work --notes "client demo" git clone https://github.com/you/thing
dexily: track "thing" at /Users/me/code/thing? [Y/n/e]

# A command dexily doesn't recognise, or a git option it can't vet, is refused:
$ dexily git push
dexily: `git push` isn't a command dexily observes (it knows: mkdir, git init, git clone). Run it directly.

# …but at a terminal you can override: you see the reason, then confirm (and nothing is tracked):
$ dexily git clone --upload-pack=/opt/my-upload-pack https://host/repo
dexily: `--upload-pack` makes git run another program, so it is not passed on. Run git directly.
        run it anyway? [y/N] y
```

- **Overriding a refusal, at a terminal.** The refusal protects scripts and
  agents; a person who typed the command and can see why it was refused may run
  it anyway. dexily prints the reason, then asks `run it anyway? [y/N]` — a
  deliberate `y` runs the command untouched. A refused `git init` / `git clone`
  whose repo dexily already parsed is tracked, with the project flags applied;
  anything else **tracks nothing**, dexily having no recognizer for what it made.
  The default is no, `--yes` never reaches this
  prompt, and only a real terminal is ever asked: a pipe, a script, a redirect
  and `--json` cannot answer it and get the refusal as before. So the "no
  arbitrary programs" guarantee holds for everything but your own keyboard.
- **The prompt**, once per folder the command made: Enter or `y` tracks it, `n`
  leaves an ordinary folder, `e` tracks it and opens the full edit form. There
  is no prompt with `--yes`, `--json`, project flags, or when stdin or stderr is
  not a terminal — then dexily tracks and says
  `dexily: tracking "friction" at …` on stderr. `--quiet` (`-q`) hides that
  line, never a warning. `dexily config track-prompt off` silences the prompt
  for good — every make then auto-tracks, as `--yes` does. Ctrl+C belongs to
  the command while it runs, and at the prompt it ends dexily (130).
- **Project flags go before the command** — `--name`, `--description`,
  `--add-tag`, `--set k=v`, `--notes`, `--open-with`, `--group`, `--color`,
  `--icon`, with `edit`'s rules — and are checked before it runs: a group that
  does not exist stops everything, exit 2, and the command never starts.
  Everything after the command's name is the command's own. `--set` names
  match ignoring case, as `edit --set`.
- **What is tracked**, and only if the command exited 0. For `mkdir`: each
  named folder it really created — not `-p`'s parents, not `-m`'s value, not
  one that was already there; a folder named twice is one. For `git init`: the
  working folder, or the one named (`-C <path>` moves the folder git works
  in). For `git clone`: the folder named, or the one git names from the
  address. Git's proof is a `.git` in the folder, which may have been there
  already; `--bare` and `--mirror` run but track nothing. For `cargo new`: the
  new folder; for `cargo init` and `go mod init`: the folder that gains a
  `Cargo.toml` or `go.mod`. A failed command
  says `dexily: git clone failed (exit 128), nothing tracked`, and one that
  exits 0 without leaving a project says
  `dexily: mkdir made nothing new, nothing tracked`.
- **The name** is the one the app would give: a git project with a remote is
  named after the remote's repository, otherwise after its folder; a name
  already taken is qualified with its parent folder. `--name` overrides it.
- **A folder already tracked** is not asked about or created again:
  `dexily mkdir app`, then `dexily git init` inside it, is one project whose
  trackers are detected again
  (`dexily: "app" was already tracked; detected its trackers again`). Flags
  still apply to it, and add to its tags and properties, never replace them.
- **Trackers are detected, not set**, so a folder's kind is found from its
  files however it was made — no recognizer needed. Alongside **git** and
  **Unreal** (`.uproject`), a tracked folder is recognised as **Node**
  (`package.json`), **Python** (`pyproject.toml`/`setup.py`), **Go** (`go.mod`),
  **Godot** (`project.godot`), **Unity** (`ProjectSettings/ProjectVersion.txt`)
  or **Rust** (`Cargo.toml`). Detection runs wherever a folder is tracked, so
  `dexily git clone` of an Unreal or Godot project comes out with both its git
  and its engine tracker.
- **A git line runs only if dexily knows every word of it.** Only the options
  dexily lists are passed on, each in full and one at a time; anything else is
  refused before git starts, exit 2 — an abbreviation (`--dep 1`), short
  options run together (`-qb main`), a global option it does not know
  (`--git-dir`, `--shallow-file`), or one git adds later. Git takes more
  spellings than a list of bad options can hold, and each of those once ran
  another program. The options that do run one (`-c`/`--config`,
  `--config-env`, `--exec-path`, `-u`/`--upload-pack`, `--template`) and
  `ext::` / `fd::` addresses say so:
  `` `--upload-pack` makes git run another program, so it is not passed on.
  Run git directly. `` The full list of options passed on is in
  [`docs/cli/agents.md`](../../docs/cli/agents.md).
- **The command's exit code always wins.** If the folder could not be recorded
  at all, the code stays 0 and dexily prints why, with the
  `dexily add <folder>` that retries it. If it was tracked but its details —
  the flags, the form — could not be saved, dexily says
  `tracked "thing", but could not save its details: <why>`, with the
  `dexily edit "thing"` that sets them. An exit code above 255 (Windows) is
  reported as 1.
- **`--json`** prints one line on **stderr**, last — the command owns stdout.
  See [`agents.md`](../../docs/cli/agents.md#observed-commands).

```
src/
  main.rs        no arguments → TUI; a known command → run it; anything else → observe it
  paths.rs       the shared projects.db (pinned by src/tests/paths.rs)
  context.rs     opens the database and builds the core services
  confirm.rs     confirmation for destructive commands (--yes in a shell), and the observer's [Y/n/e]
  launcher.rs    AppLauncher over the system opener, Tauri-free
  commands/      one module per command: clap Args + run() -> Outcome
  output/        human rendering, and the --json envelope
  observe/       the observer: recognizers (before and after), spawn (the command itself), flags
  settings.rs    cli-settings.json beside the database: the CLI's own colours
  tui/           the TUI — keys and a `:` line over the same commands
  tests/         unit tests, mirroring the source tree
```

**Released independently of the desktop app** — its own version, its own
[changelog](CHANGELOG.md), and tags of the form `dexily-v<version>`.

| Document | What it is for |
|---|---|
| [`docs/superpowers/specs/2026-09-14-cli-design.md`](../../docs/superpowers/specs/2026-09-14-cli-design.md) | the design contract: structure, the command layer, releases, milestones |
| [`docs/cli/ROADMAP.md`](../../docs/cli/ROADMAP.md) | what is planned for the CLI, and why |
| [`docs/cli/agents.md`](../../docs/cli/agents.md) | the `--json` output, field by field, for scripts and LLM agents |
| [`docs/handoffs/2026-09-04-observer-cli.md`](../../docs/handoffs/2026-09-04-observer-cli.md) | the backend it builds on, and the shared-database details |

## Rules

- **Depends on `dexily-core`, never on `src-tauri`.** Anything the app and the
  CLI both need belongs in core.
- **Behaviour lives in core.** A command is argument parsing, a call or two into
  a core service, and rendering the result.
- **A command is defined once** and used by both the shell and the TUI's `:` line.
- **The database path must match the app's**, pinned by a test.

## Releasing dexily

The CLI is versioned apart from the app and released from `dexily-v<version>`
tags by [cargo-dist](https://opensource.axo.dev/cargo-dist/)
(`dist-workspace.toml`, `.github/workflows/dexily-release.yml`). The app's
`release.yml` and its `v*` tags are separate and unaffected.

1. Bump `version` in `crates/cli/Cargo.toml` and move the CHANGELOG's
   Unreleased entries under that version. If `crates/core` changed since it
   was last published, bump its version too, and the `dexily-core` version
   `crates/cli/Cargo.toml` asks for.
2. Merge to `main`.
3. Tag and push: `git tag dexily-v<version> && git push origin dexily-v<version>`.

The tag does the rest, in this order, and stops at the first step that fails:

1. **The checks** — the same jobs as CI (`release-checks.yml` calls `ci.yml`),
   on the tagged commit, and that the crates.io token is set.
2. **The builds** — macOS (Apple silicon and Intel), static Linux (x86_64 and
   arm64), Windows x86_64, with checksums and the installers, uploaded to a
   draft release.
3. **The crates** — `publish-crates.yml` publishes `dexily-core` and `dexily`
   to crates.io together, the core first; one whose version is already there
   is left out, so the core goes up only when it changed and a release can be
   re-run. If this fails the release stays a draft.
4. **The release** is published, and `restore-latest.yml` marks the newest app
   release "Latest" again, so the releases page keeps pointing app users at the
   app.

So the archives, the installers and `cargo install dexily` are always the same
code, and none of them can come from a commit that fails the checks.

**Once, before the first release from CI:** the repository needs a crates.io
API token as the `CARGO_REGISTRY_TOKEN` secret, with the *publish-update*
scope for `dexily` and `dexily-core`:

```sh
gh secret set CARGO_REGISTRY_TOKEN   # paste the token when asked
```

Users install it with any of:

```sh
cargo install dexily
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/HADO564/project-indexer/releases/download/dexily-v<version>/dexily-installer.sh | sh
```

```powershell
irm https://github.com/HADO564/project-indexer/releases/download/dexily-v<version>/dexily-installer.ps1 | iex
```
