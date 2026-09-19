# Handoff — where the CLI stands, and what to pick up

**Date:** 2026-09-16
**Status:** `main` is green; nothing in flight, no open pull requests, no local
branches beside `main`.
**Read first:** [`../cli/checklist.md`](../cli/checklist.md) — the live list.
The contract is
[`../superpowers/specs/2026-09-14-cli-design.md`](../superpowers/specs/2026-09-14-cli-design.md),
the reasoning [`../cli/ROADMAP.md`](../cli/ROADMAP.md).

## Where we are

Milestones 1–2 are done and milestone 3 (plain subcommands) is part-way:

| Command | State |
|---|---|
| `list` | bordered table: NAME, DIRECTORY (`parent/folder`), TRACKERS, LAST OPENED; fills 70% of the terminal and centres; folder and header colours |
| `show <query>` | exact name → full id or 8-char prefix → path ending (`work/app`) → part of a name, ranked; several matches print the table and exit 1 |
| `config folder-color` / `config header-color` | 50 colours, saved in `cli-settings.json` beside the database |
| `--json` | `{"schema": 1, "data": …}` for every command |
| `add`, `open`, `untrack`, observer, TUI | stubs returning "not implemented" |

Tests: core 228 + 8, CLI 16. `cargo clippy --workspace --all-targets` is clean.
Merged as PRs #6 and #7 on 2026-09-15.

## Pick up here, in this order

1. ~~**`list [query]`**~~ *(done 2026-09-17, branch `feat/cli-list-query`)* — filter the table: a name containing the query, or a
   path ending with it when the query has a `/`, exact names ranked first.
   Needs a core `filter(projects, query) -> Vec<&Project>`, which `resolve`
   then ends with, so `list` and `show` share rules 3–4; zero matches is an
   empty table and exit 0. *(Revised 2026-09-17: an earlier plan had `list`
   use all four of `show`'s rules through a public `matches`, but then
   `list app` hid `app-gateway`.)*
2. ~~**`--tracker <kind>` / `-t`**~~ *(done: the flag and filtering on
   2026-09-18 (`feat/cli-tracker-filter`), the columns, detail sections and
   several-kinds support on 2026-09-19 (`feat/cli-tracker-columns`).)* The
   table's columns and `show`'s sections now come from the kinds named;
   `kind_cells` and `kind_details` in `output/human.rs` match every `Tracker`
   variant with no `_` arm, so a new detector fails the build there first.
3. **`...` in a path query** — `show work/.../app` spans any number of folders.
   Split query and path on `/` and compare from the end; no regex. `...` rather
   than `*`, which the shell expands before `indexer` ever runs.
4. ~~**`scan <dir>`**~~ *(done 2026-09-20, branch `feat/cli-scan`.)* Review by
   default, `--import` to register, `--json` for both. What is left is picking
   *which* candidates to import when stdin is a terminal — the same
   interaction as picking from `show`'s several matches, so worth doing once,
   for both.
5. **The rest of milestone 3** — `add`, `open`, `untrack`, groups, the bin;
   the confirmer for destructive commands; a Tauri-free `AppLauncher`;
   `views.ts` and `scanSettings.ts` logic into core.

Two things are decided but unwritten, and are worth doing before the TUI:
**`indexer pick`** (opens the TUI and prints the chosen path) with
**`indexer init <shell>`** (a `cd` function, as zoxide does), because a terminal
tool whose main use is "go to that project" cannot change the parent shell's
directory by itself; and **help** — `?` and `:help` built from the `Command`
definitions. Neither is in the checklist yet; settle them when the TUI starts.

Parked with briefs of their own: the app providing `indexer`
([`2026-09-15-gui-provides-indexer.md`](2026-09-15-gui-provides-indexer.md)) and
the observer ([`2026-09-04-observer-cli.md`](2026-09-04-observer-cli.md)).

## How to work here

- **The user writes the feature code.** Explain the step, hand over a small
  snippet or a skeleton, then review what they wrote and say what is wrong and
  why. They ask for the compiler's view often; run `cargo clippy` rather than
  reading by eye. Docs, branching, commits and pull requests are yours.
- **This is their first CLI in Rust.** Lifetimes, `?`, iterators and `match` as
  a tail expression have all been explained once; keep explanations short and
  concrete, one step at a time.
- **Never add attribution lines** to commits or pull requests.
- **Testing against real data without touching it:** build first, then run the
  binary with a temporary `HOME`, which moves the config directory and so the
  database (`HOME=<tmp> target/debug/indexer list`). `cargo` itself breaks under
  a changed `HOME`, so build before overriding it. Rows can be inserted straight
  into `projects` (`id`, `data` JSON, `is_deleted`, `directory_normalized`,
  `updated_at`). Rendering in a fake terminal: `script -q /dev/null sh -c 'stty
  cols 100; …'`.
- **Sizes, if the packaging question comes back** (measured 2026-09-15, release,
  macOS arm64): `indexer` 3.64 MB stripped / 1.73 MB gzipped; a ratatui +
  crossterm TUI adds ~0.37 MB / ~0.15 MB.
