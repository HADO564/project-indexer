# Handoff — the full-screen `edit` form

**Date:** 2026-09-27
**Status:** in progress on `feat/cli-edit-form` — step 1 done. Scope widened
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
| Enter | **Saves, from any field.** Every input is one line, so Enter is never needed for a newline. Tab / Shift+Tab and ↓ / ↑ move between fields. Esc cancels. |
| Properties | **One row per property,** a name input and a value input, like the GUI's property editor. Ctrl+N adds an empty row; Ctrl+D deletes the row under the cursor. |
| Tags | One line, comma-separated, as the GUI's tag box. |
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
2. **`FormState` for the three fields** — load from a `Project`, a single-line
   text input (insert, backspace, ←/→), focus movement, Enter → `Save`, Esc →
   `Cancel`, and `changes()`. No terminal; unit tests only.
3. **Property rows** — name and value per row, Ctrl+N / Ctrl+D, and
   `changes()` for the map, reusing `edit::edited_properties`' rules
   (names ignoring case, values as typed).
4. **Drawing** — `ui::form::draw`: a bordered block titled with the project's
   name, labelled inputs, the focused one highlighted with the cursor in it,
   and a hint line (`Enter save · Tab next · Esc cancel`). Tested with
   ratatui's `TestBackend`.
5. **The terminal and the loop** — `terminal.rs` (alternate screen and raw
   mode on stderr, restored on every exit including a panic), the
   draw → read key → `handle` loop, replacing `TerminalEditor`'s placeholder.
6. **By hand, then docs.**

**Part 2 — `--full`, one field and its flag at a time**

7. **`--full` and the favourite checkbox.** The flag (a usage error beside a
   field flag, or without a terminal), `FormState` holding which form it is,
   and a checkbox input toggled with Space. Favourite's command-line way is
   the `favorite` / `unfavorite` verbs, so it needs no new flag.
8. **Name and notes** — `--name`, `--notes` and their text fields. `--notes ""`
   clears the notes (`UpdateProject.notes` is `Some(None)`).
9. **Directory** — `--directory` and a text field. Core validates the path;
   folder browsing can come later.
10. **Open with** — `--open-with` and a text field (`""` clears it); a picker
    over `platform::list_installed_apps` can come later.
11. **Group** — `--group <name>`, resolved to an id through `ctx.groups`
    (a name no group has, or one several groups share, is an error), and
    `--group ""` to ungroup; in the form, a choice cycled with ←/→ rather than
    typed.
12. **Colour and icon** — `--color` (a palette name or `#rrggbb`, core's rule)
    and `--icon` (a bundled name or an existing `custom:<name>`; uploading a
    custom icon stays in the GUI). Text fields first; a swatch row and an icon
    list can come later.
13. **By hand, then docs and the PR.**

## 5. Definition of done

- [ ] Steps built in order, each reviewed before the next
- [ ] `cargo clippy --workspace --all-targets`, `cargo fmt --all -- --check`,
      `cargo test --workspace`
- [ ] By hand in a real terminal, per steps 6 and 13: open each form, edit
      each field, save; cancel; Ctrl+C; resize while open; a panic leaves the
      terminal usable; and `edit app` / `edit app --full` piped or under
      `--json` still exit 2
- [ ] Every field in the full form has its command-line way, and `agents.md`
      documents each new flag
- [ ] `docs/cli/checklist.md`, `docs/cli/agents.md` (bare `edit` in a
      terminal), `crates/cli/CHANGELOG.md`, `crates/cli/README.md`
