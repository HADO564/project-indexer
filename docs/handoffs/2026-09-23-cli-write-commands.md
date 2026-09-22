# Handoff — the CLI's write commands

**Date:** 2026-09-23
**Status:** ready to start. The command shape is decided (§4); nothing is
blocked.
**Runs in parallel with:**
[`2026-09-23-views-to-core.md`](2026-09-23-views-to-core.md) — see §8 there and
§7 here for the conflict surface.
**Read first:** [`../cli/checklist.md`](../cli/checklist.md) → milestone 3, and
[`../cli/ROADMAP.md`](../cli/ROADMAP.md) → *The `--json` contract*.

---

## 0. Cold start

Read these first; this handoff assumes them rather than repeating them.

| Where | For |
|---|---|
| [`../../CONTRIBUTING.md`](../../CONTRIBUTING.md) | *The checks*, *Project layout*, *Rules the codebase enforces*, *Commits and pull requests* |
| [`../architecture.md`](../architecture.md) | *Invariants worth protecting* — invariant 9 (core never depends on Tauri) is the one this work must not break |
| [`../cli/ROADMAP.md`](../cli/ROADMAP.md) | *The `--json` contract*, settled and additive-only |
| [`../cli/agents.md`](../cli/agents.md) | every command's existing `--json` shape — new commands join this document |

Conventions a fresh reader will otherwise trip on:

- **`run` prints nothing.** A command returns an `Outcome`; `output/human.rs`
  and `output/json.rs` render it. This is what lets the shell, `--json` and the
  planned TUI share one implementation, and it is why anything a renderer needs
  must be resolved *in* `run` — a renderer has no `Context` and no database.
- **stdout is data, stderr is everything else.** Under `--json`, stdout carries
  the document and nothing else; prose and failures go to stderr.
- **Doc comments on clap types are the `--help` text.** A missing one is a
  blank space in the user's help output, not a style nit.
- **No `_` arm in a `match` over `Tracker`.** Naming every variant is what makes
  a new detector fail the build until someone decides what it shows.
- **Comments and doc comments are written as part of the change**, not left as
  a follow-up. So are tests, changelog and checklist updates.
- Commits are conventional and carry **no Claude attribution**.

Build and check:

```bash
cargo build -p indexer-cli
cargo clippy --workspace --all-targets
cargo fmt --all -- --check
cargo test --workspace
```

Manual checks run against a throwaway database — set `HOME` to a scratch
directory and the CLI resolves a fresh `projects.db` under it.

---

## 1. The goal in one paragraph

The CLI reads well and writes almost nothing. `add` registers a directory and
`untrack` forgets one; everything in between — a description, tags, properties,
favouriting, the bin — can only be done in the GUI. This is the gap between
"reads the database" and "replaces the GUI for everyday use", and it is the
last large piece of milestone 3.

---

## 2. Working arrangement

How this has been run, unless the user says otherwise at the start of a
session: **the user writes the feature code.** The assistant explains what is
needed and why, reviews each step before the next begins, and owns comments,
doc comments, tests, docs, branches, commits and PRs. It writes feature code
only when the user says so explicitly, in those words. Corrections and small
follow-ups found in review are the assistant's to apply rather than hand back.

If the user instead wants this executed autonomously, they will say so — in
which case §6's order still holds, but surface each step's diff for review
rather than running to the end.

---

## 3. What core already provides

Nothing here needs new core work.

```rust
ProjectService::update(id, UpdateProject) -> Project
ProjectService::restore(id) -> Project       // clears is_deleted
ProjectService::delete(id)                   // purge; refuses unless already binned
```

`UpdateProject` — every field is `Option`, and `None` means "leave alone":

```rust
name, directory, description,
tags: Option<Vec<String>>,
favorite: Option<bool>,
open_with: Option<Option<String>>,   // double option: absent vs. explicitly cleared
notes:     Option<Option<String>>,
properties: Option<BTreeMap<String, String>>,
group_id, color, icon
```

---

## 4. Command shape — decided 2026-09-23: the hybrid

**Discrete verbs for state changes; one `edit` for the fields that take values.**

```
indexer favorite app          indexer unfavorite app
indexer restore app           indexer purge app

indexer edit app --description "The gateway" \
                 --add-tag rust --remove-tag web \
                 --set client=acme --unset priority
```

**The rule for anything added later:** *if it takes a value, it is an `edit`
flag; if it is a state change with no argument, it is a verb.* Write that down
rather than judging case by case, or the split drifts.

Why this rather than one `edit` with every flag, or a verb per operation:

- The TUI plan binds letter shortcuts to commands (`o` → `:open`,
  `d` → `:untrack`) and builds its action menu **from the `Command`
  definitions**. Favouriting needs to be a command for `f` to bind to it and
  for it to appear in that menu at all.
- `purge` must never read as an edit. As its own verb it cannot be mistaken
  for one; as `edit --purge` it would sit in `--help` beside `--description`
  as though they were comparable.
- The field flags batch. `edit app --description "…" --add-tag rust` is one
  read-modify-write, where a verb per operation would be three — three
  separate lost-update windows (5.1).
- A verb per operation would also mean `indexer property set app client acme`,
  which is a lot of typing for something naturally written inline.

---

## 5. Four traps, in order of how much they will cost

### 5.1 `tags` and `properties` replace; they do not merge

`UpdateProject.tags` is `Option<Vec<String>>` and replaces the whole list. Core
has no "add one tag". So `--add-tag rust` is a **read-modify-write**: fetch the
project, append, send the whole list back.

That is fine, and it is also a lost-update window. If the GUI writes between
the read and the write, one change vanishes with no error. A single user with
a terminal and the app open is a realistic scenario, so decide deliberately
whether to accept it (probably yes, for now) and say so in a comment.

### 5.2 `find_one` cannot see binned projects

`commands::find_one` resolves through `ctx.projects.list(SortOptions::default())`,
which filters `!is_deleted`. So `indexer restore zulu` on a binned project
reports **"no project matches"** — the row is right there, in the wrong corpus.

`restore` and `purge` need to resolve against `list_deleted` instead. Options:
give `find_one` a parameter for which corpus to search, or give the bin
commands their own small resolver. Either way it is a deliberate change, not
something to discover at the keyboard.

### 5.3 The bin is not what `untrack` produces

Worth knowing before designing `restore`:

```rust
// untrack — a HARD delete. The row is gone. Files untouched.
pub fn untrack(&self, id: &str) { self.load(id)?; self.repo.delete(id)?; }

// the ONLY path into the bin — and it removes the directory from disk first
pub fn delete_directory(&self, id: &str, delete_metadata: bool) {
    remove_directory(&project.directory)?;
    if delete_metadata { self.repo.delete(id)? } else { project.mark_deleted(); … }
}
```

So a binned project is one whose **folder has been deleted from disk** and
whose metadata was kept as a record. `restore` puts the entry back, pointing at
a directory that no longer exists — `open` on it will fail its health check,
correctly.

Two consequences:

- `restore` is about recovering *the record*, not the files. Say so in `--help`.
- **Whether the CLI should be able to put things in the bin at all is a
  separate, weightier decision.** It means deleting a user's folder from a
  terminal, where there is no undo. The ROADMAP's storage section already
  argues for a review step; treat it as out of scope here unless decided
  otherwise.

### 5.4 Destructive commands go through the confirmer

`purge` is permanent and irreversible — it must call `ctx.confirmer.confirm`,
as `untrack` does. Piped stdin without `--yes` refuses rather than reading a
script's input as consent, and answering no is `Outcome::Cancelled`, exit 0.

`restore` and `edit` are reversible and need no prompt.

---

## 6. Suggested order

Each step is small enough to review on its own.

1. **`favorite` / `unfavorite`** — the smallest possible write. One
   `UpdateProject { favorite: Some(true), ..Default::default() }`. Proves the
   whole path end to end and pairs immediately with `list --view favorites`.
2. **`edit --description`** — one more field, introduces the `edit` command and
   its argument shape.
3. **`edit --add-tag` / `--remove-tag`** — the first read-modify-write (5.1).
4. **`edit --set k=v` / `--unset k`** — parsing `k=v`, and a `BTreeMap` rather
   than a `Vec`.
5. **`restore`** — the first command that has to resolve against the bin (5.2).
6. **`purge`** — the same resolution, plus the confirmer (5.4).

Each needs an `Outcome` variant (or reuses `Outcome::Project`), a human
rendering, and a `--json` document — the same three places every command has
touched.

---

## 7. Definition of done

- [ ] Commands built in the order of §6, each reviewed before the next
- [ ] Every one has: `--json` output, a human message on stderr, an entry in
      [`../cli/agents.md`](../cli/agents.md), and tests
- [ ] `cargo clippy --workspace --all-targets`, `cargo fmt --all -- --check`,
      `cargo test --workspace`
- [ ] **By hand,** against a throwaway `HOME`: favourite a project and see it
      under `--view favorites`; add and remove a tag; set and unset a property;
      restore a binned project and confirm `show` finds it again; `purge`
      refusing without `--yes` on piped stdin
- [ ] `docs/cli/checklist.md` and `crates/cli/CHANGELOG.md` updated

---

## 8. Conflict surface with the `views.ts` branch

The two branches are close to disjoint in code. Expected overlap:

| File | Risk | Advice |
|---|---|---|
| `docs/cli/checklist.md` | **high** — both tick milestone-3 lines | Touch only your own lines; resolve by keeping both. |
| `crates/cli/CHANGELOG.md` | **high** — both append under `### Added` | Append-only; keep both entries. |
| `docs/cli/agents.md` | low | This branch adds command rows; the other should not touch it. |
| `crates/cli/src/**` | **none expected** | The `views.ts` branch has no reason to edit the CLI crate. |
| `src/**`, `src-tauri/**` | **none expected** | This branch has no reason to touch the frontend. |

One thing to leave alone: **`commands::View` in the CLI is not core's `View`.**
The CLI's is a closed `ValueEnum` (`All`, `Favorites`, `Binned`) so clap can
validate it; core's carries a group id. They should not be merged without a
decision — see §4 of the other handoff.

Rebase on `main` before opening the PR rather than at merge time.
