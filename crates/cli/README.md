# dexily

The command-line tool for Project Indexer. It records projects as you create them
(`dexily git init` runs the real `git init`, then registers the repository),
exposes most of what the desktop app does as commands, and offers a
keyboard-driven TUI. It opens the same database as the app, so the two stay in
step with no pairing — and it works without the app installed.

**Status: every plain subcommand works.** `list`, `show`, `add`, `open`,
`untrack`, `favorite`, `unfavorite`, `edit`, `restore`, `purge`, `scan` and
`config` all run against the app's database, and a bare `edit` in a terminal
opens a full-screen form (`edit --full` for every field); the observer and
the TUI still return "not implemented". `group` lists, creates, changes and
deletes groups; a project joins one with `edit --group`. The command is
`dexily`.

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
dexily <anything> --json        {"schema": 1, "data": …} on stdout, or {"schema": 1, "error": …} on stderr
```

```
src/
  main.rs        no arguments → TUI; a known command → run it; anything else → observe it
  paths.rs       the shared projects.db (pinned by src/tests/paths.rs)
  context.rs     opens the database and builds the core services
  confirm.rs     confirmation for destructive commands (--yes in a shell)
  launcher.rs    AppLauncher over the system opener, Tauri-free
  commands/      one module per command: clap Args + run() -> Outcome
  output/        human rendering, and the --json envelope
  observe/       the observer
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
