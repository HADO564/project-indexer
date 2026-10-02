# Handoff — the full-screen `edit` form

**Date:** 2026-09-27
**Status:** in progress on `feat/cli-edit-form` — part 1 (the compact form,
steps 1–7) done 2026-10-02; part 2 (`--full`) next. Scope widened
2026-09-29 to a compact form and a `--full` one (§2); nothing is blocked.
**Read first:** the write-commands brief
([`2026-09-23-cli-write-commands.md`](2026-09-23-cli-write-commands.md)), which
built the `edit` flags this form sits on, and spec decision 5 in
[`../superpowers/specs/2026-09-14-cli-design.md`](../superpowers/specs/2026-09-14-cli-design.md),
revised 2026-09-27 for this form.

---

## 0. Cold start

Everything in §0 of the write-commands brief still applies — `run` prints
nothing, stdout is data, doc comments are `--help`, commits carry no Claude
attribution. Two more for this piece:

- **This is the first `ratatui` code in the crate.** The TUI (milestone 5) is
  built on whatever this establishes — module layout, how state is kept apart
  from drawing, how the terminal is restored — so it is worth doing plainly.
- **The user writes the feature code** and the assistant explains, reviews,
  and owns comments, tests, docs and git — as before. The user may hand over a
  named part explicitly.

## 1. The goal

`indexer edit app` with no field flags opens a full-screen form of the
project's fields, the way the GUI's edit form shows them. Enter saves; Esc
cancels. Saving makes the same single `ProjectService::update` the flags make —
the form is a way of typing an `edit`, not a second path to the database.

**What the form is for (the user's words, 2026-09-29):** it is entirely human
centric — so a person can view their project at a glance and change the fields
they want to. It is never the only way to change something: every field it
shows is also a command-line flag (or, for favourite, a verb), so scripts,
agents and the TUI's `:` line can make every change a person can.

## 2. Decided

| Question | Decision |
|---|---|
| Two forms | **Compact by default, `--full` for everything** (decided 2026-09-29). `indexer edit app` opens the compact form: description, tags, properties. `indexer edit app --full` opens the full form, every field the GUI's edit view has — name, directory, description, tags, favourite, notes, open with, group, colour, icon, properties. `--full` is `--full` rather than `--all` because `edit app --all` reads as "edit all projects". It only chooses a form, so combining it with a field flag is a usage error. |
| Every field is also a flag | A field is never editable **only** in the form. Each field the full form adds gets its `edit` flag **in the same step** (`--name`, `--notes`, `--directory`, `--open-with`, `--group`, `--color`, `--icon`), so the form and the command line cannot drift apart. The rule is not "the form shows only what has a flag" but "nothing the form changes lacks a command-line way to change it". |
| Favourite | **A checkbox in the full form,** as in the GUI (decided 2026-09-29). It saves through the same `update`; on the command line it stays the `favorite` / `unfavorite` verbs, which is its command-line way. |
| Enter and Esc | **Enter saves, except in a property's name box** (revised 2026-10-02, found by using the form: after Ctrl+N and a name, Enter saved before a value could be typed). In a name box, on any row, Enter moves to that row's value; from the value, Enter saves. Every input is one line, so Enter is never needed for a newline. Tab / Shift+Tab and ↓ / ↑ move between fields. **Esc on a property row leaves the properties** (focus to the tags; an untouched row is removed as when leaving any empty row); **Esc anywhere else cancels**, so one stray Esc mid-row never throws the edit away. While Ctrl+D asks, Esc answers no. |
| Properties | **One row per property,** a name input and a value input, like the GUI's property editor. Ctrl+N adds an empty row and focuses its name; Ctrl+D deletes the row under the cursor, **after asking** (decided 2026-10-01): the form waits for one key, `y` deletes and any other key keeps the row (Ctrl+C still cancels the whole form); a row with both boxes empty is deleted without asking, since nothing is lost. Step 4 draws the question, e.g. `Delete "Client"? y/n`. |
| Empty rows | **A row needs a name; leaving an empty row removes it** (decided 2026-10-01). When focus leaves a row whose name *and* value are both empty — a Ctrl+N row never filled in, or a loaded property with both boxes cleared — the row is removed; Ctrl+N brings a new one. Ctrl+N does nothing while the focused row is empty, so blank rows cannot stack up. Saving drops an empty row still under focus. A row with a value but no name is kept and sent, and core refuses it with "a property name cannot be empty", as `edit --set =x` is refused: nothing typed is thrown away silently. Two names equal ignoring case: **the lower row wins**, as a later `--set` replaces an earlier name. |
| Tags | One line, comma-separated, as the GUI's tag box. |
| Focus at the ends | **Wraps by default, a setting turns it off** (decided 2026-09-30). Tab on the last field goes to the first and Shift+Tab on the first to the last, as the GUI's Tab order does. `indexer config form-wrap on\|off [--reset]` saves the choice in `cli-settings.json` (`Settings.form_wrap: Option<bool>`, missing = on); off, focus stops at the ends. A setting rather than a flag because it is a preference, not a per-run choice. `FormState::new` takes it as a plain `bool`, so the form stays pure and both behaviours are unit-tested. |
| When the form opens | Only when **no field flag** was given, stdin and stderr are both terminals, and `--json` is off. Otherwise a bare `edit` (or `edit --full`) stays a **usage error, exit 2**, as today. |

## 3. Shape

```
crates/cli/src/tui/
  form.rs        FormState: the fields, the focus, key handling, and the
                 changes to save. Pure — no terminal — so it is unit-tested.
  terminal.rs    enter/leave the alternate screen and raw mode on stderr,
                 restoring the terminal on every exit path, panics included
  ui/form.rs     draws a FormState with ratatui; tested with TestBackend
```

**State is separate from drawing.** `FormState::handle(key) -> Action`
(`Continue`, `Save`, `Cancel`) decides everything; `ui::form::draw(frame,
&state)` only paints. That split is what lets every rule be tested without a
terminal, and it is the split the TUI proper will want.

**Only changed fields are sent.** `FormState::changes()` compares each field
with the project as it was loaded and fills an `UpdateProject` with just the
ones that differ. Saving an untouched form writes nothing. This also narrows
the lost-update problem: the GUI's form overwrites every field; this one only
the fields actually edited (the full fix is still the optimistic `updated_at`
check in `../architecture.md` → *Quality backlog*).

**The form is an input method on `Context`, like the confirmer.** A
`ProjectEditor` trait with one method — given the project, return the changes
or `None` for cancelled — and a terminal implementation. `edit::run` calls it
when no flag was given. Tests supply a stand-in, the way
`src/tests/commands/bin.rs` supplies a stand-in confirmer, so `edit`'s wiring
is tested without a screen.

**Drawn on stderr, not stdout** — the split `../cli/ROADMAP.md` → *Interactive
picking* already sets for anything interactive, so stdout stays data. That
means building the `Terminal` over `std::io::stderr()` rather than using
`ratatui::init()`, which assumes stdout, and installing the panic hook that
restores the terminal ourselves.

**One dependency:** `ratatui = "0.30"`. It re-exports crossterm as
`ratatui::crossterm`, so crossterm is not added separately.

## 4. Steps

Each is small enough to review on its own, in this order. Part 1 is the
compact form, end to end; part 2 grows it into the full one. The boundary
between them is a natural point for a pull request.

**Part 1 — the compact form**

1. ~~**The non-interactive path.**~~ *(done 2026-09-28, `e7f9af7`.)* The
   `change` group no longer requires a member; `edit::run` asks
   `ctx.editor` — a `ProjectEditor` on `Context` — when no field flag was
   given, and `TerminalEditor` refuses without a terminal or under `--json`
   with `Failure::Usage`, which `main` turns into prose and exit 2.
2. ~~**`FormState` for the three fields**~~ *(done 2026-10-01; properties
   moved to step 3 with their rows.)* — load from a `Project`, a single-line
   text input (insert, backspace, ←/→), focus movement (wrapping or stopping
   at the ends, as a `bool` given to `FormState::new`), Enter → `Save`, Esc →
   `Cancel`, and `changes()`. No terminal; unit tests only.
3. ~~**Property rows**~~ *(done 2026-10-02.)* — name and value per row, Ctrl+N / Ctrl+D, an empty
   row removed when focus leaves it (§2, *Empty rows*), Ctrl+D asking first
   (§2, *Properties*), and `changes()` for
   the map, reusing `edit::edited_properties`' rules (names ignoring case,
   the lower of two equal names wins, values as typed).
4. ~~**Drawing**~~ *(done 2026-10-02; modifiers only, no colours, so it
   reads the same under `NO_COLOR`.)* — `ui::form::draw`: a bordered block titled with the project's
   name, labelled inputs, the focused one highlighted with the cursor in it,
   and a hint line (`Enter save · Tab next · Esc cancel`). Tested with
   ratatui's `TestBackend`.
5. ~~**The terminal and the loop**~~ *(done 2026-10-02.)* — `terminal.rs` (alternate screen and raw
   mode on stderr, restored on every exit including a panic), the
   draw → read key → `handle` loop, replacing `TerminalEditor`'s placeholder.
6. ~~**The `form-wrap` setting**~~ *(done 2026-10-02.)* — `Settings.form_wrap: Option<bool>` (missing =
   on), `indexer config form-wrap on|off [--reset]` beside the colour
   settings, and `main` passing it to `TerminalEditor::new`, which hands it to
   `FormState::new`. A broken settings file falls back to wrapping, as the
   colours fall back to their defaults.
7. ~~**By hand, then docs.**~~ *(done 2026-10-02.)* Found by hand: Enter
   on an untouched form still called `update`, moving `updated_at` and
   printing `updated`. Fixed: core's `UpdateProject::is_empty`, and `edit`
   returns `Outcome::Unchanged` without writing.

**Part 2 — `--full`, one field and its flag at a time**

8. **`--full` and the favourite checkbox.** The flag (a usage error beside a
   field flag, or without a terminal), `FormState` holding which form it is,
   and a checkbox input toggled with Space. Favourite's command-line way is
   the `favorite` / `unfavorite` verbs, so it needs no new flag.
9. **Name and notes** — `--name`, `--notes` and their text fields. `--notes ""`
   clears the notes (`UpdateProject.notes` is `Some(None)`).
10. **Directory** — `--directory` and a text field. Core validates the path;
    folder browsing can come later.
11. **Open with** — `--open-with` and a text field (`""` clears it); a picker
    over `platform::list_installed_apps` can come later.
12. **Group** — `--group <name>`, resolved to an id through `ctx.groups`
    (a name no group has, or one several groups share, is an error), and
    `--group ""` to ungroup; in the form, a choice cycled with ←/→ rather than
    typed.
13. **Colour and icon** — `--color` (a palette name or `#rrggbb`, core's rule)
    and `--icon` (a bundled name or an existing `custom:<name>`; uploading a
    custom icon stays in the GUI). Text fields first; a swatch row and an icon
    list can come later.
14. **By hand, then docs and the PR.**

## 5. Definition of done

- [ ] Steps built in order, each reviewed before the next
- [ ] `cargo clippy --workspace --all-targets`, `cargo fmt --all -- --check`,
      `cargo test --workspace`
- [ ] By hand in a real terminal, per steps 7 and 14: open each form, edit
      each field, save; cancel; Ctrl+C; resize while open; a panic leaves the
      terminal usable; and `edit app` / `edit app --full` piped or under
      `--json` still exit 2
- [ ] Every field in the full form has its command-line way, and `agents.md`
      documents each new flag
- [ ] `docs/cli/checklist.md`, `docs/cli/agents.md` (bare `edit` in a
      terminal), `crates/cli/CHANGELOG.md`, `crates/cli/README.md`
