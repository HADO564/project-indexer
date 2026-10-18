# TUI design

**Date:** 2026-10-15
**Status:** draft for review — no code yet
**Roadmap:** `docs/cli/ROADMAP.md` → *The TUI*; checklist §5
**Prerequisite:** done — prose-through-a-writer (PR #33, 2026-10-15)

Bare `dexily` opens a full-screen interface for browsing and acting on tracked
projects. This spec fixes the layout, the state model, the key map and the
phase boundaries so the build is a series of small, reviewable PRs rather than
one milestone-sized drop.

---

## 1. Principles

- **Every change is a command.** The TUI never writes to the database except by
  running a `Command` — the same enum the shell parses. The `:` line parses into
  it, letter shortcuts build it, and the action menu is a list of them. One code
  path for mutations, in the TUI and the shell alike; the TUI is a *way of typing
  commands*, not a second door to the data. (This is the vim-style-but-command-
  driven direction: shortcuts and a which-key menu on the surface, a command
  underneath every one.)
- **Never the only way.** The CLI stays whole. The TUI is for a person at a
  terminal; a script always has the subcommands and `--json`.
- **Reuse what exists.** `tui::terminal::Session` (raw mode + alternate screen on
  stderr + Drop/panic teardown), the edit form (`tui/form.rs`, `ui/form.rs`), the
  `Command` enum, `appearance::glyph` for icons, and the new prose writer for the
  message line.
- **Pure state, testable without a terminal.** Model the app the way `FormState`
  is modelled — keys in, state out, side effects named and run by the loop — so
  every transition is a unit test with no PTY.

## 2. Layout

Three panes, a status line and a hint line:

```
┌─ dexily ─────────────────────────────────────────────────────┐
│ VIEWS           │ PROJECTS                 │ DETAIL           │
│  All       (42) │ > Acme_Admin_Panel   Git │ Acme_Admin_Panel │
│  Favourites (3) │   Acme_Storefront    Git │ directory  …     │
│  Ungrouped (12) │   build_at_12:30         │ id         …     │
│  Bin        (1) │   devmon         Git,Node│ tags       …     │
│ GROUPS          │   …                      │ group      Work  │
│  Work       (5) │                          │ trackers   …     │
│  Clients    (2) │                          │          ⏎ expand│
│  … 8 more  ↓    │                          │                  │
├─────────────────┴──────────────────────────┴──────────────────┤
│ 42 projects · All                                              │ ← status line; ':' / '/' turn it into an input
│ j/k move · o open · e edit · / search · : cmd · ? help · q quit│ ← hint line
└────────────────────────────────────────────────────────────────┘
```

- **Sidebar** — the fixed views **All · Favourites · Ungrouped · Bin** (the core
  `View` set, counts included), then the groups in sidebar order, each with its
  live count. When the groups overflow, the sidebar **scrolls and keeps the
  selection in view — it never collapses into foldable sections.** That is a
  deliberate echo of the GUI: `viewState.ts` records that a sectioned/collapsible
  sidebar was *rejected* because persisting a selection is only safe when every
  view is always on screen. Icons via `appearance::glyph`, colours as `list`
  draws them.
- **List** — the projects for the selected view/group, columns as `dexily list`,
  the selection highlighted with a reverse/bold modifier so it reads under
  `NO_COLOR`. List is the default; a grid/card mode mirrors the GUI's
  `ViewMode = "list" | "grid"` and is a later phase (§8, §11 Q4).
- **Detail** — the selected project as `dexily show` renders it; `Enter` expands
  it full-screen for a project with many fields or trackers (§11 Q6). A pane, not
  a tab.
- **Status line** — normally the last prose (fed by the PR #33 `err` writer) or a
  count summary. It is **modal**: `:` turns it into a command input, `/` into a
  search input; `Esc` restores it unchanged, `Enter` runs/applies (§4, §11 Q7).
  There is no always-visible command line.
- **Hint line** — the active keys; `?` replaces the body with a full help overlay.

## 3. State model (`tui/app.rs`)

One `App` struct, the single source of truth for a frame:

| Field | Meaning |
|---|---|
| `projects: Vec<Project>`, `groups: Vec<Group>` | the last database read |
| `view: View` + `group_id: Option<Uuid>` | the sidebar selection (set vs. group) |
| `focus: Focus` (`Sidebar` \| `List`) | which pane movement drives |
| `selected: usize` + `selected_id: Option<Uuid>` | the cursor in the filtered list; id kept so a refresh can restore it |
| `mode: Mode` | `Normal` \| `Command(String)` \| `Search(String)` \| `Confirm { prompt, pending: Command }` \| `ActionMenu` \| `Help` |
| `message: String` | the last prose line |
| `data_version: i64` | last seen `PRAGMA data_version`, for refresh |
| `should_quit: bool` | set by `q` / `Ctrl+C` |

Transitions are pure: `App::handle(key) -> Vec<Effect>`, where `Effect` is
`RunCommand(Command)`, `Reload`, `OpenForm(project)`, or `Quit`. The event loop
executes effects (they touch `ctx` and the terminal); the state itself never
does I/O. This is exactly the `FormState::handle` → `Action` shape, scaled up.

## 4. Command dispatch — the core

- **`:` line.** The typed text is split into argv and fed to the **same clap
  `Command` parser the shell uses**. Parse error → the usage message on the
  message line, exit nothing. Success → `commands::run(command, ctx)`, render the
  `Outcome` with `output::human::write(out_sink, &mut message_buf, …)` so the
  prose lands on the message line, then `Reload`.
- **Letter shortcuts** build a `Command` for the selection and run it the same
  way: `o` → `Command::Open { query: selected_id }`, etc. A shortcut is sugar
  over a command, never a separate mutation path.
- **Action menu** (`Space`) lists the commands that apply to the selection, built
  from the `Command` definitions; choosing one runs it, or — if it needs a value
  — opens the `:` line prefilled (e.g. `:edit <id> --group `).
- **Confirmation.** Destructive commands (`untrack`, `purge`, `group delete`)
  enter `Confirm` mode and take a `y/n` on the command line before running. The
  TUI injects its **own `Confirmer`** into `Context` that answers from this mode —
  which is why `StdinConfirmer`/`TerminalGate` stayed shell-only in PR #33; the
  TUI substitutes its own, as the trait was built to allow.

## 5. Key map (defaults)

Every key resolves to a **named action** (`move-down`, `open`, `command-line`, …)
before anything runs, even in phase 1 — so phase 5 (configurable keys) is a
config layer over this table, not a rewrite.

| Keys | Action |
|---|---|
| `j` / `↓`, `k` / `↑` | move the selection |
| `g` / `G` | top / bottom |
| `Tab` / `h` / `l` | switch focus sidebar ↔ list |
| `[` / `]` | (sidebar) jump to the previous / next section — Views ↔ Groups ↔ Saved |
| `/` | **filter the focused pane** — view/group entries in the sidebar, or projects in the list (the `name: value` / free-text query engine); incremental, `Esc` clears, `Enter` commits |
| `o` | open · `e` edit (opens the existing form) · `f` favourite toggle (list focus) |
| `d` | untrack (confirm) · in the bin: `r` restore · `x` purge (confirm) |
| `:` | command line · `n` → `:add ` prefilled (Tab completes paths) |
| `Space` | action menu · `?` help overlay |
| `q` / `Ctrl+C` | quit |

## 6. Live refresh

The loop polls with a timeout (`event::poll(250ms)`); each idle tick reads
`PRAGMA data_version`, and on a change re-reads projects/groups, restoring the
selection by `selected_id`. It also reloads after any command. This is the
"GUI and CLI both write the one SQLite file" case from the roadmap — the TUI is
a third reader that must notice the others' writes.

## 7. Teardown & non-terminal

Reuse `terminal::Session` (Drop restores the screen) and the panic hook. All
drawing is on stderr; stdout stays untouched. Bare `dexily` with no terminal, or
under `--json`, keeps today's behaviour — it cannot draw a TUI to a pipe, so it
errors cleanly rather than hanging.

## 8. Phases (one PR each)

1. **Skeleton (render-only) — shipped 2026-10-15, PR #34.** Enter the alt-screen,
   draw the three panes from a one-time read, `j/k`/arrows and `g`/`G` move,
   `Tab`/`h`/`l` switch pane, `[`/`]` jump sidebar sections, `q`/`Esc`/`Ctrl+C`
   quit cleanly. Icons and mark colours drawn from `appearance` (Q5), an empty
   view says so, no terminal errors cleanly. No mutations. Proved the loop,
   layout and teardown.
2. **The `:` command line — shipped (phase 2).** `:` opens a
   modal status line; the typed text is tokenized (quote-aware) and parsed by a
   `no_binary_name` clap wrapper into the shell's `Command`, run with
   `commands::run`, its prose shown on the status line, then the panes reload.
   The TUI injects its own confirmer/editor (`tui::host`) that refuse the two
   interactive flows — a destructive command's `y/n`, the bare-`edit` form —
   until phases 4/3, so every non-interactive command works now. The heart of
   "every change is a command". (Defaulting an omitted target to the selected
   project is a phase-3 nicety, with the letter shortcuts.)
3. **Shortcuts + action menu + `?` help + hint line.** Letter keys dispatch
   commands on the selection; `Space` menu from the `Command` definitions.
4. **Live refresh + search + `n` add + `y/n` confirm.** `data_version` polling,
   `/` filter, `n` → prefilled `:add ` with Tab completion, the TUI `Confirmer`.
5. **Configurable keys.** The shared form+TUI keymap and `config` rebinding,
   saved in `cli-settings.json`. Largest and most independent; last.

## 9. Non-goals / deferred

- Mouse / right-click menu — the checklist mentions right-click; fold into phase
  3 only if cheap, else defer.
- Configurable keys — phase 5 (the shared form+TUI keymap, `config` rebinding).
- **Theming / TUI colours** (asked 2026-10-16) — folds into the planned
  **Theming** feature (checklist → *Theming*: `config theme <scheme>` /
  `config theme set <role> <colour>`, LazyVim-style named schemes with per-role
  overrides, saved in `cli-settings.json`). The TUI reads the same roles —
  `folder`/`header` already are two — so there is no separate TUI theming effort;
  it arrives when Theming does.
- **Fonts** (asked 2026-10-16) — not possible: a terminal app cannot choose the
  font, which the emulator owns; a program may only toggle bold, italic,
  underline, dim and reversed. The one font-adjacent lever is the icon style
  (`config icons nerd|emoji|off`), already wired.
- Project linking / the graph view — deferred past the whole TUI (app roadmap).

## 10. Decisions — resolved (2026-10-15)

- **Movement:** both `hjkl` and arrows (the hint shows both).
- **`e` edit:** reuses the existing full-screen form.
- **Action-menu trigger:** `Space` (which-key style).
- **Project search scope (`/` in the list):** the current view only.
- **Sidebar/pane filter key:** `/` (one rule — "filter the focused pane"); `f`
  stays favourite.
- **Section jump:** `[` / `]` between sidebar sections.
- **Grid/card mode:** out of this milestone; list-only, grid revisited later.
- **Saved custom views:** deferred to a joint TUI + GUI feature (§11 Q2).

## 11. Review questions (2026-10-15) — resolutions

**Q1 — Large Groups (or Views) section.** A large section is handled three ways,
never by collapsing:
- **Scroll.** The fixed views (All/Favourites/Ungrouped/Bin) stay pinned at the
  top; the groups list below scrolls and keeps the selection in view, with a
  `… N more ↓` affordance.
- **Filter (`/`).** With the sidebar focused, `/` starts an incremental
  find-as-you-type over the entries (vim-style), narrowing to those whose name
  matches; `Esc` clears it, `Enter` selects. Same `/` gesture as the list, which
  filters projects — one rule: *`/` filters the focused pane.*
- **Section jump (`[` / `]`).** Jump the cursor to the first entry of the
  previous / next section (Views ↔ Groups ↔ a future Saved block), so reaching
  Groups from Views is one key, not a scroll.

Rationale and consistency: the GUI rejected a sectioned/collapsible sidebar
(`viewState.ts`) because legibility — every view on screen — is what makes
persisting a selection safe. The filter here is **transient and never persisted**,
so committing a selection clears it and the full sidebar returns with the
selection visible; that is precisely the property the rejection protected, so a
find-as-you-type does not reopen the problem collapsing would have. *Open key
choice:* the user suggested `f` for the filter; `f` is favourite in the list, so
the spec uses `/` (uniform "filter the focused pane"). `f` could instead be the
sidebar filter when the sidebar is focused (favourite stays `f` in the list) —
confirm which.

**Q2 — Custom views from tags/properties.** Core already has the *query* engine
(`domain::views`): `name: value` property queries, free text over
name/directory/tags/property-values, and `property_keys` for a hint. What does
not exist is a *saved* custom view — the `View` set is fixed. Decision:
- **Now:** the TUI's `/` search uses that engine directly, so `tag: rust` or
  `client: acme` filters ad-hoc from day one — most of the value, no new storage.
- **Later (own feature, after the TUI core):** *saved* custom views. Because what
  a view *means* lives in core so the CLI and GUI cannot drift, a saved view is a
  core concept (name + query) in its own table/meta — **not** the project blob —
  exposed through `dexily` + `--json` + the GUI. Cross-cutting, so out of the TUI
  milestone. When they land they sit in a "Saved" sidebar section below Groups,
  and Q1's scroll rule covers a long view list too.
- **Decided (2026-10-15): deferred.** Saved custom views are built for the **TUI
  and the GUI together**, in one pass, as their own cross-cutting feature later —
  not in this milestone. The reason they go together: the view definition lives
  in core precisely so the two frontends cannot drift, so adding a *saved* view
  is one change surfaced in both, not a TUI-only bolt-on. The TUI's `/` still does
  ad-hoc `tag:` / property filtering now.

**Q3 — Ungrouped.** Fixed: Ungrouped is a first-class core `View` (as is Bin) and
is now in the sidebar — **All · Favourites · Ungrouped · Bin**. My first sketch
dropped it; corrected in §2.

**Q4 — Cards vs list.** The GUI already offers both (`ViewMode = "list" | "grid"`,
`ProjectRow` vs `ProjectTile`), defaulting to list. A tile shows mark + name +
directory + tracker badges + favourite + actions. Decision: the TUI **defaults to
a list** (dense rows, best for keyboard nav, matches `dexily list`); a
**grid/card mode** is an optional later phase mirroring `ProjectTile`. In a
terminal a card costs 4–5 rows, so a list shows far more at once — list earns the
default. *Your call:* include grid mode in the TUI milestone, or list-only for now?

**Q5 — Icons and colours.** Reuse the CLI `appearance` module exactly as
`dexily list`/`show` already do, rendered with ratatui styles:
- **Icons** — `appearance::glyph`, honouring `config icons` (`nerd` Font Awesome
  glyphs · `emoji` · `off`, the default); `custom:*` and unknown names fall back
  to the folder glyph.
- **Colour** — `appearance::rgb`/`mark_rgb`, true-colour: a project's own colour
  else its group's, as its mark in the GUI. The row highlight / card edge takes
  the mark colour; `NO_COLOR` drops colour (icons stay) and the selection still
  reads via a reverse/bold modifier.

No new rendering model — the mark logic is already shared with the shell.
**Implemented in phase 1 (2026-10-15):** glyphs from `appearance::glyph`, names
in `appearance::mark_rgb` / `rgb` colour, `NO_COLOR` honoured, favourite starred
in gold.

**Q6 — Detail tab, and the graph.** Detail is a **pane**, not a tab; `Enter`
expands it full-screen for a project with many fields/trackers, then collapses
back. On the **graph**: project linking is drawn in the GUI with **Svelte Flow**
and is **deferred past the whole TUI** (app roadmap → *Project linking*; its own
open questions — typed/directed links, where edges live — are unresolved). A
node-graph canvas does not translate to a terminal, so when linking lands the TUI
surfaces it **not as a drawn graph but as a "Links" section in the detail pane**:
a navigable list of related projects (typed/directed if links are), each jumped
to with `Enter` — navigation *is* the traversal. The canvas stays GUI-only.
Nothing graph-related is in this milestone.

**Q7 — Command line hidden until `:`.** Adopted (reflected in §2). There is no
always-visible command line: the bottom row is the **status line** until `:`
turns it into a command input (or `/` into search). **Esc** cancels back to the
status line unchanged; **Enter** runs the command (renders the `Outcome` on the
status line and reloads) or applies the search. This is vim's model, and it is
already the `Mode` enum in §3 (`Command(String)`, `Search(String)`).
