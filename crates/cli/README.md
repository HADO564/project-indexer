# indexer-cli

The command-line tool for Project Indexer. It records projects as you create them
(`indexer git init` runs the real `git init`, then registers the repository),
exposes most of what the desktop app does as commands, and offers a
keyboard-driven TUI. It opens the same database as the app, so the two stay in
step with no pairing — and it works without the app installed.

**Status: every plain subcommand works.** `list`, `show`, `add`, `open`,
`untrack`, `scan` and `config` all run against the app's database; the observer
and the TUI still return "not implemented". Groups and the bin have no commands
yet. The binary name `indexer` is a placeholder.

```
indexer list                     every project, as a table
indexer show <query>             one project — exact name, id, parent/folder, or part of a name
indexer add [dir]                track a directory, or the current one
indexer open <query>             open a project in its application
indexer untrack <query>          forget a project, leaving its files alone (asks first; --yes)
indexer scan <dir>               find projects under a folder; --import registers them
indexer config folder-color ...  the folder colour in that table; header-color likewise
indexer <anything> --json        {"schema": 1, "data": …} on stdout, or {"schema": 1, "error": …} on stderr
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
[changelog](CHANGELOG.md), and tags of the form `cli-v<version>`.

| Document | What it is for |
|---|---|
| [`docs/superpowers/specs/2026-09-14-cli-design.md`](../../docs/superpowers/specs/2026-09-14-cli-design.md) | the design contract: structure, the command layer, releases, milestones |
| [`docs/cli/ROADMAP.md`](../../docs/cli/ROADMAP.md) | what is planned for the CLI, and why |
| [`docs/cli/agents.md`](../../docs/cli/agents.md) | the `--json` output, field by field, for scripts and LLM agents |
| [`docs/handoffs/2026-09-04-observer-cli.md`](../../docs/handoffs/2026-09-04-observer-cli.md) | the backend it builds on, and the shared-database details |

## Rules

- **Depends on `indexer-core`, never on `src-tauri`.** Anything the app and the
  CLI both need belongs in core.
- **Behaviour lives in core.** A command is argument parsing, a call or two into
  a core service, and rendering the result.
- **A command is defined once** and used by both the shell and the TUI's `:` line.
- **The database path must match the app's**, pinned by a test.
