# Handoff — the full-screen `edit` form

**Date:** 2026-09-27
**Status:** ready to start on `feat/cli-edit-form`. Scope and keys decided (§2);
nothing is blocked.
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
project's editable fields, the way the GUI's edit form shows them. Enter saves;
Esc cancels. Saving makes the same single `ProjectService::update` the flags
make — the form is a way of typing an `edit`, not a second path to the
database.

## 2. Decided

| Question | Decision |
|---|---|
| Which fields | **Exactly the fields `edit` has flags for:** description, tags, properties. A field joins the form when its flag joins `edit` (`--name`, `--notes`, `--group`, …), so the form can never change something the shell cannot. `favorite` stays a verb and is not in the form. |
| Enter | **Saves, from any field.** Every input is one line, so Enter is never needed for a newline. Tab / Shift+Tab and ↓ / ↑ move between fields. Esc cancels. |
| Properties | **One row per property,** a name input and a value input, like the GUI's property editor. Ctrl+N adds an empty row; Ctrl+D deletes the row under the cursor. |
| Tags | One line, comma-separated, as the GUI's tag box. |
| When the form opens | Only when **no field flag** was given, stdin and stderr are both terminals, and `--json` is off. Otherwise a bare `edit` stays a **usage error, exit 2**, as today. |

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

Each is small enough to review on its own, in this order.

1. **The non-interactive path.** Relax the `change` group from
   `required(true)`; in `run`, no flags and not interactive (stdin or stderr
   not a terminal, or `--json`) is a usage error. Keeping **exit 2** needs a
   way for `run` to say "usage error" to `main` — a `Failure` variant `main`
   maps to exit code 2. No form yet: behaviour is unchanged, reached by a new
   route. Tests: bare `edit` parses; bare `edit` without a terminal is the
   usage error.
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
   draw → read key → `handle` loop, and the `ProjectEditor` on `Context` that
   `edit::run` calls. Tests with a stand-in editor.
6. **By hand, then docs and the PR** — a real terminal: open, edit each field,
   save; cancel; Ctrl+C; resize while open; a panic leaves the terminal usable.

## 5. Definition of done

- [ ] Steps built in order, each reviewed before the next
- [ ] `cargo clippy --workspace --all-targets`, `cargo fmt --all -- --check`,
      `cargo test --workspace`
- [ ] By hand in a real terminal, per step 6; and `edit app` piped or under
      `--json` still exits 2
- [ ] `docs/cli/checklist.md`, `docs/cli/agents.md` (bare `edit` in a
      terminal), `crates/cli/CHANGELOG.md`, `crates/cli/README.md`
