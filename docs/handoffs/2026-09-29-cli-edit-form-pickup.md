# Handoff — picking up the `edit` form at step 2

**Date:** 2026-09-29
**Branch:** `feat/cli-edit-form`, off `main` at `5ee4b63` (PR #17 merged).
Pushed; nothing uncommitted. `main` has not moved since the branch was cut —
check with `git fetch && git log HEAD..origin/main` before starting.
**The task brief:** [`2026-09-27-cli-edit-form.md`](2026-09-27-cli-edit-form.md)
— decisions (§2), shape (§3), the thirteen steps (§4), definition of done
(§5). This file does not repeat it; it says where the work stands and how to
carry on.

---

## 0. Cold start — read in this order

| Where | For |
|---|---|
| This file | where things stand, how to work with the user, step 2 in detail |
| [`2026-09-27-cli-edit-form.md`](2026-09-27-cli-edit-form.md) | the form's design; **§2 changed on 2026-09-29** (compact + `--full`) |
| [`2026-09-23-cli-write-commands.md`](2026-09-23-cli-write-commands.md) §0 | the codebase conventions every CLI change follows — still binding |
| [`../../CONTRIBUTING.md`](../../CONTRIBUTING.md) | *The checks*, *Project layout* (tests beside the tree), *Commits and pull requests* |
| [`../cli/checklist.md`](../cli/checklist.md) → milestone 3 | the live status; the form's item has sub-items to tick |
| `crates/cli/src/commands/edit.rs`, `editor.rs`, `tests/commands/support.rs` | the code step 2 builds on |

Build and check, as ever:

```bash
cargo build -p indexer-cli
cargo clippy --workspace --all-targets   # clean — zero warnings is the baseline
cargo fmt --all -- --check
cargo test --workspace                   # CLI 99, core 274, migrations 8
```

## 1. How to work with this user — read before writing any code

These come from the user, over several sessions. They are not optional.

- **The user writes the feature code.** The project exists for them to learn
  Rust by building it. Explain what is needed and why, give the *shape* (a
  signature, a skeleton, an analogous example on a different problem) and
  review what they write. **Write feature code only when they explicitly hand
  a part over** — "you do this part", "can you make changes 3, 4 and 5", "can
  you apply the fixes", "do the final 2 parts". A handover covers exactly what
  was named; the next part is theirs again. Never offer to write it as a menu
  option.
- **Yours without asking:** comments and doc comments (never hand them back as
  a to-do), tests, docs, changelog and checklist ticks, small fixes found in
  review (a stale doc line, a user-facing message, an unused import), branches,
  commits, pushes when asked, pull requests.
- **They are new to Rust.** Explain in plain words with comparisons: `Option`
  as a box that may be empty, `match` arms, `Box`, closures in `retain`,
  shadowing, partial moves have all come up. Short, concrete, one idea at a
  time; walk through code piece by piece when asked.
- **Ground reviews in the compiler.** Run `cargo build` / `cargo clippy`
  rather than reading by eye, and try the result against a throwaway
  database (§4) before calling it done.
- **Report what was wrong and why** when fixing their code, as a numbered
  list — they read these to learn.
- **Never add Claude attribution** to commits or pull requests.
- **Decisions are theirs.** When a choice genuinely belongs to them, lay out
  the options with a recommendation and ask; then write the answer into the
  docs (brief, checklist, and spec/roadmap if it revises them) the same day.

## 2. Where things stand

**Done on this branch:**

| Commit | What |
|---|---|
| `251dad4` | the brief |
| `e7f9af7` | step 1 — a bare `edit` reaches a `ProjectEditor` on `Context` |
| `548a674` | the 2026-09-29 decisions: compact form by default, `--full` for every field, favourite a checkbox |

**How step 1 works** (the pieces step 2 plugs into):

- `crates/cli/src/editor.rs` — `trait ProjectEditor { fn edit(&self, &Project)
  -> anyhow::Result<Option<UpdateProject>> }`; `Some` = save these changes,
  `None` = cancelled. `TerminalEditor { json }` refuses with
  `Failure::Usage` under `--json` or when stdin or stderr is not a terminal,
  and otherwise `bail!`s "the edit form is not built yet" — **step 5 replaces
  that `bail!` with the form**, and renames `_project` back to `project`.
- `Context.editor: Box<dyn ProjectEditor>`, passed to `Context::open` beside
  the confirmer; `main` builds `TerminalEditor::new(cli.json)`.
- `commands/edit.rs` — `has_field_flag(&args)` is checked **before**
  `find_one` (which moves `args.project` out, after which `&args` can't be
  lent whole). No flag → `ctx.editor.edit(&project)?`; `None` →
  `Outcome::Cancelled`. Flags → `from_flags(…)` as before. The `change`
  `ArgGroup` no longer has `required(true)`.
- `Failure::Usage { message }` — `main` prints it as prose and returns exit
  code 2, even under `--json` (the contract keeps usage errors prose). Its
  `json.rs` arm exists only because the match must be exhaustive and reports
  kind `error`, never a fourth kind.
- `tests/commands/support.rs` — an in-memory `Context` (`context(consent,
  editor)`, projects `app` live and `old` binned), `Answer` (stand-in
  confirmer), `Scripted(fn(&Project) -> Option<UpdateProject>)` (stand-in
  editor), `never_asked` (panics if the editor is asked), and `run(ctx, argv)`
  which parses and runs a real command. Use these for any test that runs a
  command.

**The TUI module is stubs.** `crates/cli/src/tui/` has `mod.rs` (`run` bails),
and `app.rs`, `cmdline.rs`, `keys.rs`, `ui/mod.rs` holding doc comments only.
`tui/mod.rs`'s module doc still says "view-only" — stale since 2026-09-15; fix
it when the first real code lands there.

**Not on this branch, recorded for later:** an optimistic `updated_at` check
against lost updates between the GUI and the CLI (`../architecture.md` →
*Quality backlog* → *Later*). The form narrows the problem by sending only
changed fields; it does not solve it.

## 3. Step 2 — `FormState` for the compact form

The user has not started it. Give them this breakdown (adapted to how they
ask), let them write it, review with the compiler, then write the tests and
docs.

**Dependency.** Add `ratatui = "0.30"` to `crates/cli/Cargo.toml` now, for its
key types; it re-exports crossterm as `ratatui::crossterm`, so crossterm is
not added separately. (`cargo search ratatui` showed 0.30.2 on 2026-09-27.)

**Files.** `crates/cli/src/tui/form.rs`, declared in `tui/mod.rs`; tests in
`crates/cli/src/tests/tui/form.rs` (a new `tests/tui/mod.rs`, declared in
`tests/mod.rs`) — tests mirror the source tree.

**The shape** — no terminal anywhere in this step:

```rust
pub enum Action { Continue, Save, Cancel }

/// One line of editable text and where the cursor is in it.
pub struct TextInput { value: String, cursor: usize }   // cursor counts chars

pub struct FormState { /* the original values, one TextInput per field, the focus */ }

impl FormState {
    pub fn new(project: &Project) -> Self;
    pub fn handle(&mut self, key: KeyEvent) -> Action;
    pub fn changes(&self) -> UpdateProject;   // only the fields that differ
}
```

- **Fields for step 2:** description and tags as `TextInput`s. Properties can
  be held as rows of two `TextInput`s already, but adding and deleting rows
  (Ctrl+N / Ctrl+D) is step 3.
- **Keys:** printable characters insert at the cursor; Backspace deletes before
  it; ←/→ move it; Tab / ↓ next field, Shift+Tab (`KeyCode::BackTab`) / ↑
  previous; **Enter → `Save`**, **Esc → `Cancel`**.
- **`changes()`** fills an `UpdateProject` with only the fields that differ
  from the project as loaded, everything else `None`, so saving an untouched
  form writes nothing.

**Traps worth raising before they hit them:**

1. **The cursor is a character count, but `String::insert` and `remove` take a
   byte index.** A tag like `café` or an emoji breaks naive indexing with a
   panic. Convert with `char_indices()` (or keep the value as `Vec<char>`).
2. **Only react to `KeyEventKind::Press`.** Windows reports key releases as
   separate events; without the check every key types twice there. CI runs on
   Windows.
3. **Ctrl+C must cancel.** In raw mode (step 5) the terminal no longer turns
   Ctrl+C into a signal — it arrives as a key event
   (`KeyCode::Char('c')` with `KeyModifiers::CONTROL`), and a form that ignores
   it cannot be quit that way. Make it `Cancel` here, and make sure a
   Ctrl-modified character is not also *typed*.
4. **Tags compare as core stores them.** The field shows `Rust, Web`; decide
   "changed" by parsing on commas, trimming, dropping empties and comparing
   with `indexer_core::domain::normalize::normalize_tags` applied — otherwise
   retyping `rust` over `Rust` counts as a change. Send the parsed list; core
   normalizes on save.
5. **Description is a plain `String` in core** (`""` = none), so an emptied
   field is `Some(String::new())`, not `None`.

**Tests (yours):** loading a project fills the fields; typing, Backspace and
cursor movement including non-ASCII text; focus wraps or stops at the ends
(decide which with the user — the GUI's Tab order wraps); Enter / Esc /
Ctrl+C give their `Action`; a key release does nothing; `changes()` is empty
for an untouched form, carries only the edited field, and ignores a tag
retyped in another case.

**Then:** steps 3–6 per the brief. Step 4 (drawing) is where the user first
sees `ratatui`; step 5 needs the stderr-backed `Terminal` and a panic hook
that restores the terminal, because `ratatui::init()` assumes stdout.

## 4. Testing by hand without touching real data

- Build **before** overriding `HOME` — cargo breaks under a changed `HOME`.
- `HOME=<scratch dir> target/debug/indexer …` resolves a fresh `projects.db`
  under it. Use the session's scratchpad directory, not `/tmp`.
- A real terminal for the form: `script -q /dev/null target/debug/indexer edit
  app` gives the binary a pseudo-terminal from a non-interactive shell.
- To put a project in the bin (no CLI command does it — binning deletes the
  folder), set `is_deleted` in both the `data` JSON blob and the
  `is_deleted` column of `projects`, e.g. with Python's `sqlite3`. The same
  trick stores a GUI-style value (a property named `Client`) to test against.

## 5. Things learned on the write-commands branch that still apply

- `UpdateProject` derives `Default` (every field `None` = leave alone) —
  build updates with `..Default::default()`.
- Core title-cases tags (`rust` → `Rust`, `CLI` → `Cli`) and de-duplicates
  them on save; property **names** are stored as typed but the search matches
  them ignoring case, so the CLI matches them ignoring case too
  (`edit::edited_properties`); property **values** are kept exactly as typed.
- `human.rs` writes prose with `eprintln!`, so tests cannot capture it — a
  known limitation (checklist → *Prose leaves `eprintln!`*), and one the TUI
  must fix before it draws anything that coexists with prose.
- Pull requests merge with a merge commit (`gh pr merge --merge`) after CI
  (Rust on Ubuntu and Windows, frontend, CLA) passes; branches are kept.
