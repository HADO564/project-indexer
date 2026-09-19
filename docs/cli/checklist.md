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
- [x] Human-readable tables, and `--json` under the settled contract (`ROADMAP.md` → *The `--json` contract*)
- [x] Trackers in `--json` as `{"kind": "git", …}` rather than serde's default `{"Git": {…}}` — an output DTO in `output/json.rs` (`ProjectJson`, `TrackerJson`), so a rename in core can't change the documented shape by accident
- [x] Failures under `--json` as `{"schema": 1, "error": {"kind", "message", …}}` on stderr — `not_found` and `ambiguous` (with the candidates) come from `commands::Failure`; anything else is `error`
- [x] [`agents.md`](agents.md): every command's `--json` output, the project and tracker shapes, errors and exit codes, for scripts and LLM agents
- [ ] `show`, `add`, `open`, `untrack`
  - [x] `list` — aligned name and directory columns, the folder name coloured in a terminal only (respects `NO_COLOR`)
  - [x] `list` prints the same table as `show`'s several matches (`output::human::project_table`, the folder coloured in a terminal with padding worked out on the plain text; tests in `src/tests/output/human.rs`) — `NAME  DIRECTORY  TRACKERS  LAST OPENED` — as the CLI's default view, like the GUI's main list. `list` is the table and `show` is one project's details (docker `ps`/`inspect`, kubectl `get`/`describe`), rather than a `-s` flag switching `show` between the two
  - [x] The table is bordered (rounded box-drawing corners) with a coloured header row. In a terminal it grows to at least 70% of the width, sharing the extra space between columns, and is centred (`terminal_size`); piped, or inside `show`'s error, it is only as wide as its cells and not indented. Tests pin the plain layout, the stretch and centring, and that colour moves no column
  - [x] `resolve`'s four rules split into private helpers in `domain::matching` (`matches_exact`, `matches_id`, `matches_path`, `matches_name`), with `resolve` keeping the order between them. A public `matches` returning every match was tried and folded back in: nothing but `resolve` needed it, and `Resolution::Ambiguous` already carries the list
  - [x] `list [query]` filters the table — a name containing the query, or a path ending with it when the query has a `/` (rules 3–4 of `show`, not all four, so `list app` still shows `app-gateway`). A core `filter(projects, query) -> Vec<&Project>`, ranked exact name, then starts with, then contains, then most recently opened; `resolve` ends with `pick(filter(…))`, so the two can't disagree. Zero matches is an empty table, exit 0, with `no projects match "…"` rather than "no projects tracked yet"
  - [x] `show <id>` — basic human output: name, directory, id
  - [ ] `show` — a richer view: dates, favourite, tags, properties, group name (per-tracker details come with `--tracker`, below)
  - [x] `show <query>` finds a project by part of its name, ignoring case (`domain::matching::resolve` in core); no match is an error, exit 1
  - [x] **Check the several-matches case by hand** (2026-09-15, against a throwaway database with `app` in `work/` and `play/` plus `app-gateway`): `show app` and `show ap` print the table and exit 1, `show work/app`, `show 6c5d931e` and a full id in capitals find one. It caught an exact name shared by two projects silently picking the first
  - [x] An exact name wins before the name search: `show api` picks the project named `api` even when `api-gateway` also exists. Several projects can share a name (`~/work/app`, `~/play/app`), so an exact name that several share lists them all rather than picking the first
  - [x] A query containing `/` matches the end of the path — `show work/app` finds `~/work/app` and not `~/play/app`. Whole folder names only: the path must end with `/` + the query, so `rk/app` doesn't match. Zoxide's space-separated words were considered and dropped: typing back the `parent/folder` the list prints is simpler and more predictable
  - [x] List several matches as a table instead of only a count — headers `NAME  DIRECTORY  TRACKERS  LAST OPENED`. DIRECTORY is `parent/folder`, so it can be typed back; TRACKERS is every detected tracker's `Tracker::kind()` joined with `, ` (`-` when none); LAST OPENED is `YYYY-MM-DD` or `never`. Built by `output::human::project_table` and carried in the error, which suggests adding a folder from the path
  - [x] Rank several matches best first — name starts with the query, then name contains it — ties to the most recently opened
  - [x] Tests for `domain::matching` in `crates/core/src/tests/domain/matching.rs`
  - [x] `show <id>` works again, for scripts and copy-paste — a full id, or an id prefix of at least 8 characters (`SHORT_ID_LEN`), checked after the exact name and only when the query is hex digits and dashes, so a name like `cafe` is still searched by name
  - [ ] Print a short id in the table, so the several-matches list can be copied from by id as well as by `parent/folder`
  - [ ] *Next branch:* `...` in a path query stands for any number of folders — `show work/.../app` finds `~/work/client/acme/app` and `~/work/client/ea/app`. Several matches produce the same list as any other query. Matched by splitting the query and the path on `/` and comparing from the end, no regex. `...` rather than `*` because the shell expands `*` before `indexer` ever runs (zsh stops with `no matches found`); `*` could be accepted too for users who quote it or alias `noglob indexer`
  - [x] `--tracker <kind>` (short `-t`) on `list` and `show`, e.g. `show app --tracker git`. A clap `ValueEnum` (`git`, `unreal`), so `--help` lists the kinds and typos are rejected — one flag with a value rather than `--git`, `--unreal`, … so a new detector adds a value, not a flag. `commands::with_tracker` keeps only projects with that tracker (`Tracker::is`) *before* `resolve`/`filter`, which don't change, so `show app -t git` finds the git `app` instead of reporting an ambiguity
  - [x] Per-tracker columns and details. The table's TRACKERS column becomes each named tracker's own columns (git: BRANCH, CHANGES; Unreal: ENGINE), in the order given, `-` where a project lacks one; `show`'s detail view gains a section per kind (git: branch, changes, remote; Unreal: project, engine, vcs). `project_table` is no longer fixed at four columns — `headers`/`middle_cells` derive them from the kinds. `kind_cells` and `kind_details` name every `Tracker` variant with no `_` arm, so a new detector fails the build until it has columns
  - [x] Several kinds at once: `-t git -t unreal` or `-t git,unreal`, keeping a project that carries any of them, de-duplicated and in the order given
  - [ ] *Later:* pick from several matches interactively (arrow keys or a number) when stdin is a terminal, instead of printing the list and exiting 1. Scripts and pipes keep the list and the exit code
  - [ ] *Later, not this branch:* `show .` resolves the project in the current directory
- [x] `config folder-color [COLOR] [--reset]` — 50 colours (16 standard + 34 named 24-bit); saved in `cli-settings.json` beside `projects.db`, never in the database; `--folder-color` overrides it for one run
- [x] `config header-color [COLOR] [--reset]` — the same colours for a table's header row, default magenta; `--header-color` overrides it for one run. Both settings share one `Outcome::Color { setting, color }`, so `--json` reports `{"header_color": …}` or `{"folder_color": …}`
- [ ] `scan <dir>` — **first-release work, not "later with groups"**: it is how a corpus gets populated at all. Someone arriving with 40 project folders cannot `add` them one at a time, and the GUI's scan is the only way to bulk-register today
  - [ ] `ScanService::scan` then `import`, the same two steps the GUI uses: the walk registers nothing, so the review step stays deliberate
  - [ ] Review by default: print the candidates as a table (directory, suggested name, kinds, already-tracked) and register nothing. `--import` (or `--yes`) performs the import; `--json` emits the report
  - [ ] `--depth <n>` maps to `ScanMode` (1 = `Quick`), `--include-ignored` widens the prune list, `--tracker <kinds>` restricts `ScanRequest::detectors` (empty finds nothing, deliberately — so the CLI defaults to every kind `detector_kinds()` reports)
  - [ ] `stopped_early` (the `MAX_DIRECTORIES` bound) is surfaced, not swallowed, so an incomplete scan never reads as "nothing more to find"
  - [ ] The `ImportReport`'s `skipped` and `failures` are reported per row; a failed row never fails the whole command
- [ ] Groups, the bin (restore and purge)
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
- [ ] Panes: sidebar views with counts, the project list, project detail
- [ ] Keybinds for movement and search
- [ ] A `:` command line parsed into the same `Command` as the shell, defaulting to the selected project
- [ ] Letter shortcuts that run commands on the selected project (`o` open, `d` untrack, …)
- [ ] `n` opens the `:` line with `:add ` pre-filled — the current directory, `Tab` completing paths
- [ ] A per-project action menu (LazyVim / which-key style) on `Space` and on right-click, built from the `Command` definitions; every entry also reachable by key
- [ ] A hint line or `?` help overlay showing the keys
- [ ] `y/n` confirmation on the command line for destructive commands
- [ ] Refreshes when another process writes, via `PRAGMA data_version`

## Later — packaging

- [ ] A CLI release workflow: `-p indexer-cli` per target, none of the app's system dependencies, on `cli-v*` tags
- [ ] **The desktop app provides `indexer` from its own binary** — brief: [`../handoffs/2026-09-15-gui-provides-indexer.md`](../handoffs/2026-09-15-gui-provides-indexer.md)
  - [ ] `indexer-cli` gains a library entry point; the TUI goes behind a default-on `tui` feature
  - [ ] The app's `main` runs the CLI when invoked as `indexer`, before Tauri starts — never opens a window
  - [ ] "Install command-line tool" in the app: puts `indexer` on `PATH` (macOS symlink, Windows, deb/rpm), detects an existing `indexer` rather than overwriting it, and can remove it
  - [ ] Windows console output decided and working (release builds use the `windows` subsystem)
  - [ ] `indexer list` through the app's binary starts within a measured budget (~100 ms)
- [ ] Homebrew tap, winget manifest, Scoop bucket; Linux channels decided
- [ ] Anything asking GitHub for "the latest release" filters by tag prefix
