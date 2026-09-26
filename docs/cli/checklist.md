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
- [x] `show`, `add`, `open`, `untrack`
  - [x] `list` — aligned name and directory columns, the folder name coloured in a terminal only (respects `NO_COLOR`)
  - [x] `list` prints the same table as `show`'s several matches (`output::human::project_table`, the folder coloured in a terminal with padding worked out on the plain text; tests in `src/tests/output/human.rs`) — `NAME  DIRECTORY  TRACKERS  LAST OPENED` — as the CLI's default view, like the GUI's main list. `list` is the table and `show` is one project's details (docker `ps`/`inspect`, kubectl `get`/`describe`), rather than a `-s` flag switching `show` between the two
  - [x] The table is bordered (rounded box-drawing corners) with a coloured header row. In a terminal it grows to at least 70% of the width, sharing the extra space between columns, and is centred (`terminal_size`); piped, or inside `show`'s error, it is only as wide as its cells and not indented. Tests pin the plain layout, the stretch and centring, and that colour moves no column
  - [x] `resolve`'s four rules split into private helpers in `domain::matching` (`matches_exact`, `matches_id`, `matches_path`, `matches_name`), with `resolve` keeping the order between them. A public `matches` returning every match was tried and folded back in: nothing but `resolve` needed it, and `Resolution::Ambiguous` already carries the list
  - [x] `list [query]` filters the table — a name containing the query, or a path ending with it when the query has a `/` (rules 3–4 of `show`, not all four, so `list app` still shows `app-gateway`). A core `filter(projects, query) -> Vec<&Project>`, ranked exact name, then starts with, then contains, then most recently opened; `resolve` ends with `pick(filter(…))`, so the two can't disagree. Zero matches is an empty table, exit 0, with `no projects match "…"` rather than "no projects tracked yet"
  - [x] `show <id>` — basic human output: name, directory, id
  - [ ] `show` — a richer view: dates, favourite, tags, properties (group name done; per-tracker details come with `--tracker`, below)
  - [x] `show` names the project's group. `Project` stores only a `group_id`, and a renderer has no database, so `show::run` resolves it and carries a `GroupLabel` on the `Outcome` — the first CLI caller of `ctx.groups`. The label also carries the group's colour and icon, unrendered for now, so painting them later needs no change to the command layer. A `group_id` whose group has gone is treated as ungrouped, not as an error
  - [x] `list --sort <field>` with `-r/--reverse` — `name` (A to Z) or `last-opened` (most recent first), mapping onto core's `SortOptions`. The two fields have opposite natural directions, so `--reverse` means "the opposite of this field" rather than "descending"; leaving it at `SortDirection::default()` would make `--sort last-opened` answer with the stalest projects first
  - [x] `show <query>` finds a project by part of its name, ignoring case (`domain::matching::resolve` in core); no match is an error, exit 1
  - [x] **Check the several-matches case by hand** (2026-09-15, against a throwaway database with `app` in `work/` and `play/` plus `app-gateway`): `show app` and `show ap` print the table and exit 1, `show work/app`, `show 6c5d931e` and a full id in capitals find one. It caught an exact name shared by two projects silently picking the first
  - [x] An exact name wins before the name search: `show api` picks the project named `api` even when `api-gateway` also exists. Several projects can share a name (`~/work/app`, `~/play/app`), so an exact name that several share lists them all rather than picking the first
  - [x] A query containing `/` matches the end of the path — `show work/app` finds `~/work/app` and not `~/play/app`. Whole folder names only: the path must end with `/` + the query, so `rk/app` doesn't match. Zoxide's space-separated words were considered and dropped: typing back the `parent/folder` the list prints is simpler and more predictable
  - [x] List several matches as a table instead of only a count — headers `NAME  DIRECTORY  TRACKERS  LAST OPENED`. DIRECTORY is `parent/folder`, so it can be typed back; TRACKERS is every detected tracker's `Tracker::kind()` joined with `, ` (`-` when none); LAST OPENED is `YYYY-MM-DD` or `never`. Built by `output::human::project_table` and carried in the error, which suggests adding a folder from the path
  - [x] Rank several matches best first — name starts with the query, then name contains it — ties to the most recently opened
  - [x] Tests for `domain::matching` in `crates/core/src/tests/domain/matching.rs`
  - [x] `show <id>` works again, for scripts and copy-paste — a full id, or an id prefix of at least 8 characters (`SHORT_ID_LEN`), checked after the exact name and only when the query is hex digits and dashes, so a name like `cafe` is still searched by name
  - [x] Print a short id in the table, so the several-matches list can be copied from by id as well as by `parent/folder`. An ID column third, after NAME and DIRECTORY — the identity columns grouped, with TRACKERS and LAST OPENED as the facts after them. It prints `SHORT_ID_LEN` characters read from core, not a literal 8, so the table can never show an id `show` would refuse
  - [x] `list --view <set>` — `all`, `favorites`, `binned`, onto core's `list`/`list_favorites`/`list_deleted`. One flag with a value, not two booleans: the sets are alternatives, and `views.ts` already models them as one `View` with groups as a further case carrying an id. Favourite and binned are independent flags on a project, not groups, so binning a favourite keeps the flag and only the views disagree about showing it. `Outcome::Projects` carries the view so an empty one says which emptiness it is. `ungrouped` deliberately left out: the only view with no core method behind it, and inventing its rule CLI-side is what the `views.ts` migration exists to prevent
  - [ ] *Later:* `list --pick` — choose a project from the table, filtering as you type, and print only the choice to stdout so it composes: `indexer open "$(indexer list --pick)"`. The third caller of the shared picker (`ROADMAP.md` → *Interactive picking*)
  - [ ] *Next branch:* `...` in a path query stands for any number of folders — `show work/.../app` finds `~/work/client/acme/app` and `~/work/client/ea/app`. Several matches produce the same list as any other query. Matched by splitting the query and the path on `/` and comparing from the end, no regex. `...` rather than `*` because the shell expands `*` before `indexer` ever runs (zsh stops with `no matches found`); `*` could be accepted too for users who quote it or alias `noglob indexer`
  - [x] `--tracker <kind>` (short `-t`) on `list` and `show`, e.g. `show app --tracker git`. A clap `ValueEnum` (`git`, `unreal`), so `--help` lists the kinds and typos are rejected — one flag with a value rather than `--git`, `--unreal`, … so a new detector adds a value, not a flag. `commands::with_tracker` keeps only projects with that tracker (`Tracker::is`) *before* `resolve`/`filter`, which don't change, so `show app -t git` finds the git `app` instead of reporting an ambiguity
  - [x] Per-tracker columns and details. The table's TRACKERS column becomes each named tracker's own columns (git: BRANCH, CHANGES; Unreal: ENGINE), in the order given, `-` where a project lacks one; `show`'s detail view gains a section per kind (git: branch, changes, remote; Unreal: project, engine, vcs). `project_table` is no longer fixed at four columns — `headers`/`middle_cells` derive them from the kinds. `kind_cells` and `kind_details` name every `Tracker` variant with no `_` arm, so a new detector fails the build until it has columns
  - [x] Several kinds at once: `-t git -t unreal` or `-t git,unreal`, keeping a project that carries any of them, de-duplicated and in the order given
  - [ ] *Later:* pick from several matches interactively (arrow keys or a number) when stdin is a terminal, instead of printing the list and exiting 1. Scripts and pipes keep the list and the exit code — one of the three callers of the shared picker (`ROADMAP.md` → *Interactive picking*)
  - [ ] *Later, not this branch:* `show .` resolves the project in the current directory
- [x] `config folder-color [COLOR] [--reset]` — 50 colours (16 standard + 34 named 24-bit); saved in `cli-settings.json` beside `projects.db`, never in the database; `--folder-color` overrides it for one run
- [x] `config header-color [COLOR] [--reset]` — the same colours for a table's header row, default magenta; `--header-color` overrides it for one run. Both settings share one `Outcome::Color { setting, color }`, so `--json` reports `{"header_color": …}` or `{"folder_color": …}`
- [x] `scan <dir>` — **first-release work, not "later with groups"**: it is how a corpus gets populated at all. Someone arriving with 40 project folders cannot `add` them one at a time
  - [x] `ScanService::scan` then `import`, the same two steps the GUI uses: the walk registers nothing, so the review step stays deliberate
  - [x] Review by default: the candidates as a table (directory relative to the scanned folder, suggested name with `(renamed)` when disambiguation changed it, kinds, already-tracked), then the summary and the ready-made `--import` command. `--json` emits the `ScanReport`
  - [x] `--depth <n>` maps to `ScanMode` (1 = `Quick`), `--include-ignored` widens the prune list, `--tracker <kinds>` restricts `ScanRequest::detectors` — matched case-insensitively against `detector_kinds()`, since a detector spells its own kind (`Git`) and the scan compares exactly
  - [x] `stopped_early` (the `MAX_DIRECTORIES` bound) is surfaced, not swallowed, so an incomplete scan never reads as "nothing more to find"
  - [x] The `ImportReport`'s `skipped` and `failures` are reported per row; a failed row never fails the whole command
  - [ ] *Later:* pick which candidates to import when stdin is a terminal, instead of all-or-nothing. Same interaction as picking from `show`'s several matches
- [x] `add [dir]` registers a directory through `ensure_project`, defaulting to the current one — `indexer add` in a project folder is the common case, and `add .` still works. The path is canonicalised before it is stored, because the database outlives the shell that wrote to it and the GUI reads it from a different working directory; a missing path or a file is refused. Already tracked is reported, not an error: `already_tracked` in `--json`, a different sentence in human output
- [x] `open <query>` hands the directory to the system opener, or to the project's `open_with` app, and stamps `last_opened_at`
- [x] `untrack <query>` forgets a project's metadata, leaving its directory alone
- [x] Destructive commands confirm through a confirmer (`--yes` in a shell) — `untrack` is the first caller. Piped stdin without `--yes` refuses rather than reading a script's input as consent; answering no is `Outcome::Cancelled`, exit 0, because nothing went wrong
- [x] A Tauri-free `AppLauncher` for `open` — **decided: the CLI** (`src/launcher.rs`, `SystemLauncher`). Core already holds the platform-specific halves (`open_with_app_available`, and `open_with_command` for a Linux `.desktop` command line); what was left is handing a path to the system opener, which the `open` crate — the one `tauri-plugin-opener` wraps — does everywhere. Core would take that dependency for one frontend, and the app cannot drop its plugin without being retested on three platforms. Move this into `core::platform` when it does
- [ ] The write commands — favourite, description, tags, properties — and the bin (restore, purge). Branch `feat/cli-write-commands`, one step at a time in the handoff's order:
  - [x] `favorite` / `unfavorite <query>` — one `update` with only `favorite` set, through `find_one` like `open`. One `favorite::run` takes the flag to store, so the two verbs share every line; one `Outcome::Favorited { project, favorite }` for both, `--json` is the saved project. Needed `#[derive(Default)]` on core's `UpdateProject`, which the handoff assumed existed — every field `None`, pinned by `a_default_update_leaves_every_field_alone`
  - [x] `edit --description` — introduces `edit` and its argument shape. Every field flag joins a clap `ArgGroup` named `change` with `required(true).multiple(true)`: at least one, any combination, so a bare `edit app` is a usage error (exit 2) and each later flag needs only `group = "change"`. `description` passes straight into `UpdateProject`, since both are `Option<String>` meaning the same thing; `""` clears it. Its own `Outcome::Edited`, and `--json` is the whole saved project
  - [ ] `edit --add-tag` / `--remove-tag` — the first read-modify-write. Its lost-update window is accepted for now and said so in a comment; the fix, for this and for the GUI's edit form, is the optimistic `updated_at` check in `../architecture.md` → *Quality backlog*
  - [ ] `edit --set k=v` / `--unset k`
  - [ ] `restore`
  - [ ] `purge`
  - [ ] *After the flags:* `edit <query>` with no field flags opens a full-screen form of the project's fields (decided 2026-09-27; spec decision 5, `ROADMAP.md` → *The TUI*). Saving makes the same one `update` the flags make. Until it exists, and whenever stdin is not a terminal or under `--json`, a bare `edit` is a usage error (exit 2)

  The design, recorded: **Shape decided 2026-09-23: the hybrid** — discrete verbs for state changes (`favorite`, `unfavorite`, `restore`, `purge`), one `edit` for the fields that take values (`--description`, `--add-tag`, `--set k=v`). The rule for anything added later: if it takes a value it is an `edit` flag, if it is a state change with no argument it is a verb. Favouriting has to be a command in its own right for the TUI's `f` to bind to it and for it to appear in the action menu built from the `Command` definitions; `purge` as its own verb can never read as an edit; and the field flags batch into one read-modify-write where a verb each would be three. Briefed in [`../handoffs/2026-09-23-cli-write-commands.md`](../handoffs/2026-09-23-cli-write-commands.md). Two traps recorded there: `UpdateProject.tags` and `.properties` **replace** rather than merge, so adding one tag is a read-modify-write; and `find_one` resolves through `list`, which filters `!is_deleted`, so `restore` and `purge` cannot find the very projects they act on until they resolve against `list_deleted`
- [ ] Groups
- [ ] **Prose leaves `eprintln!` and goes through a writer.** Every message that is not data — `no projects match "…"`, `the bin is empty`, `tracking "app" at …`, the scan summary, `cancelled` — is written straight to the process's stderr from inside `output::human::write`. Two consequences: `src/tests/output/human.rs` cannot assert on any of them, because they never reach the `out` it captures; and **a TUI cannot use them at all** — a full-screen app draws to an alternate screen buffer, so a stray `eprintln!` lands on top of the interface and stays there until a redraw. The TUI needs these on its message line, which means `write` taking a second writer for prose (or an `Outcome` carrying the notice), decided when the TUI starts. Until then the CLI behaviour is correct and only the tests are poorer for it
- [ ] `text_width` counts display columns, not `chars()` (`unicode-width`) — an emoji or a CJK project name is one char and two columns, so it under-pads its column and shifts every column to its right. Latent today, and the blocker for icons in any table (`ROADMAP.md` → *Group colour and icons in a terminal*)
- [ ] Group colour in the CLI: `Color::from_swatch` for the seven shared names, a `Rust` variant for the eighth, and `#rrggbb` into the existing `Code::Rgb`
- [ ] `indexer config icons nerd|emoji|off` and an icon renderer with a fallback chain, for the TUI sidebar — `custom:*` icons are uploaded images and always fall back
- [x] `views.ts` view, count and search logic moved into core, with the GUI switched over in the same change — briefed in [`../handoffs/2026-09-23-views-to-core.md`](../handoffs/2026-09-23-views-to-core.md). Now `core::domain::views` (`View`, `resolve_view`, `matches_query`, `is_property_query`, `property_keys`, `view_counts`), behind the `resolve_view`, `view_counts` and `property_keys` Tauri commands. The GUI's three `$derived` became effects: the search returns ids, which the page filters its sorted list by, debounced while typing and guarded against out-of-order replies. `views.ts` keeps only `viewKey`, `parseViewKey` and `viewLabel`. `View::Group` is `Group { id }`, not `Group(String)` — an internally tagged enum cannot carry an unnamed string beside `kind`
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
- [ ] Prose on the message line, not stderr — see the item in milestone 3; a stray `eprintln!` lands on top of the alternate screen buffer and stays there
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
