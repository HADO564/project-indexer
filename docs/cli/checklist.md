# Project Indexer — CLI checklist

What's done and what's still open for the command-line tool, milestone by
milestone. The design contract is
[`../superpowers/specs/2026-09-14-cli-design.md`](../superpowers/specs/2026-09-14-cli-design.md),
and the plans and their reasoning are in [`ROADMAP.md`](ROADMAP.md). The shared
`indexer-core` features the CLI builds on are tracked in
[`../app/checklist.md`](../app/checklist.md), where they shipped.

## Organisation

- [x] Stays in this workspace as `crates/cli`, not a separate repository (spec → *Decisions locked*, 1)
- [x] Versioned and released independently: `0.1.0`, [`crates/cli/CHANGELOG.md`](../../crates/cli/CHANGELOG.md), `cli-v*` tags (`CONTRIBUTING.md` → *Versioning and releases*)
- [x] Every crate is `publish = false`
- [x] Binary named `indexer` — a placeholder, set in one place (`[[bin]]` in `crates/cli/Cargo.toml`)
- [x] Docs split per product: this checklist and the CLI roadmap, plus the crate's README and changelog
- [ ] Choose the final binary name

## 1. Foundations

- [x] `ensure_project` ignores binned projects in its directory check — the parked one-line `!p.is_deleted` filter, also listed in `../app/checklist.md`
- [ ] A test pinning it: bin a project, `ensure_project` the same directory, get a new live project
- [x] `indexer-core` added as a dependency of `crates/cli`
- [x] `paths.rs` resolves the shared `projects.db`, with a test pinning it to the app's path on the current OS (`src/tests/paths.rs` — also pins the identifier to `tauri.conf.json`)
- [x] The version-skew error from `SqliteRepository::open` is printed, never swallowed (`main` prints the full error chain to stderr and exits 1)
- [x] Module skeleton per the spec: `commands/`, `output/`, `observe/`, `tui/`, `context.rs`, `confirm.rs`, with each command stubbed to return "not implemented"

## 2. First slice

- [x] `indexer list` prints real rows from the real database — a project added in the GUI appears in `indexer list` and `list --json` with no restart (2026-09-14)

## 3. Plain subcommands

- [x] `Command` enum + `run(command, &Context) -> Outcome`, with rendering kept out of `run`
- [ ] Human-readable tables, and `--json` under the settled contract (`ROADMAP.md` → *The `--json` contract*)
- [ ] Trackers in `--json` as `{"kind": "git", …}` rather than serde's default `{"Git": {…}}` — `--json` currently serialises `Project` as-is, so the variant name leaks as the key. Fix with an output DTO in `output/json.rs` before anyone scripts against it
- [ ] `show`, `add`, `open`, `untrack`
  - [x] `list` — aligned name and directory columns, the folder name coloured in a terminal only (respects `NO_COLOR`)
  - [x] `show <id>` — basic human output: name, directory, id
  - [ ] `show` — a richer view: dates, favourite, tags, properties, group name, per-tracker details (git branch, dirty, remote)
  - [x] `show <query>` finds a project by part of its name, ignoring case (`domain::matching::resolve` in core); no match is an error, exit 1
  - [ ] **Check the several-matches case by hand.** Only one project was tracked when this landed, so `show` with a query matching two or more projects has never been run. Add a second project whose name shares a word with the first and confirm `show <word>` reports the count and exits 1
  - [ ] List several matches with enough to tell them apart — short id, name, last opened, path, and **every detected tracker** on the project (not only git) — instead of only a count. One short label per tracker from `project.trackers`: git as its branch plus `*` when dirty, Unreal as its engine version, and any kind without a specific label as its `Tracker::kind()` name, so a new detector shows up without a CLI change
  - [ ] Exact rules before name matching: exact name, full id and 8-character short id, a directory path (`show .`), several words matched against the path in order (zoxide's rule); `show <id>` stopped working when `show` switched to name matching
  - [ ] Rank several matches best first, ties to the most recently opened
  - [ ] Tests for `domain::matching` in `crates/core/src/tests/domain/matching.rs`
- [x] `config folder-color [COLOR] [--reset]` — 50 colours (16 standard + 34 named 24-bit); saved in `cli-settings.json` beside `projects.db`, never in the database; `--folder-color` overrides it for one run
- [ ] Groups, the bin (restore and purge), scan
- [ ] Destructive commands confirm through a confirmer (`--yes` in a shell)
- [ ] A Tauri-free `AppLauncher` for `open` — in `core::platform` or the CLI, decided at this milestone
- [ ] `views.ts` view, count and search logic moved into core, with the GUI switched over in the same change
- [ ] `scanSettings.ts` validation moved into core, likewise

## 4. The observer

- [ ] Spawn the wrapped command with inherited stdio; its exit code always wins
- [ ] First recognizers chosen, each with its project-directory rule written down
- [ ] Records through core services (`ensure_project`, `refresh_trackers`); a recording failure never changes the exit code
- [ ] Human-path output decided (silent, or one line on stderr), and whether `--quiet` exists

## 5. The TUI

- [ ] `indexer` with no arguments opens it
- [ ] Read-only panes: sidebar views with counts, the project list, project detail
- [ ] Keybinds for movement and search
- [ ] A `:` command line parsed into the same `Command` as the shell, defaulting to the selected project
- [ ] `y/n` confirmation on the command line for destructive commands
- [ ] Refreshes when another process writes, via `PRAGMA data_version`

## Later — packaging

- [ ] A CLI release workflow: `-p indexer-cli` per target, none of the app's system dependencies, on `cli-v*` tags
- [ ] Homebrew tap, winget manifest, Scoop bucket; Linux channels decided
- [ ] Anything asking GitHub for "the latest release" filters by tag prefix
