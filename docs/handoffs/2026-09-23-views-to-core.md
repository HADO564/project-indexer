# Handoff — `views.ts` into `indexer-core`

**Date:** 2026-09-23
**Status:** ready to start. The *what* is settled; §5 is the one design decision
left, and it is the reason this is its own branch.
**Runs in parallel with:** the CLI's write commands (`edit`, tags, properties,
favourite, `restore`, `purge`). See §8 for the conflict surface.
**Read first:** [`../cli/checklist.md`](../cli/checklist.md) → milestone 3, and
[`../../src/lib/views.ts`](../../src/lib/views.ts) in full — it is 168 lines and
its comments carry most of the reasoning.

---

## 1. The goal in one paragraph

The GUI decides what "Favourites", "Ungrouped" and `client: acme` mean, in
TypeScript. The CLI and the planned TUI need the same answers. Three
implementations of `matchesQuery` would be three subtly different search
behaviours with nothing checking they agree — so the rules move into
`indexer-core`, and **the GUI switches over in the same change**. That last
clause is the whole point: porting the logic without switching the GUI creates
exactly the duplication this exists to prevent.

`views.ts` itself argues the case, about sorting:

> Re-implementing `core::domain::sorting` in TypeScript would be a second
> unchecked cross-language mirror, the drift hazard `api/types.ts` is already
> flagged for.

Same argument, now applied to the module that wrote it.

---

## 2. What moves, what stays

| Stays in `src/lib/views.ts` | Moves to core |
|---|---|
| `viewKey`, `parseViewKey` — localStorage encoding, a browser concern | `View` (the type itself) |
| `viewLabel` — UI strings ("Favourites", "Unknown group") | `isPropertyQuery`, the `name: value` parse |
| | `propertyKeys` |
| | `matchesProperty`, `matchesQuery` |
| | `resolveView` |
| | `ViewCounts`, `viewCounts` |

`viewLabel` needs `groups` and returns display text; `viewKey` is a storage
format. Neither is a rule about *which projects match*, so neither belongs in
core.

---

## 3. Rules to port exactly

These are behaviours, not implementation details. The existing tests in
[`../../src/tests/lib/views.test.ts`](../../src/tests/lib/views.test.ts) (273
lines) pin most of them — **read them before porting, and port them too.**

- **`name: value` is a property query, everything else is free text.** The name
  must be non-empty, hold no whitespace, and be **at least two characters**.
  That is what keeps `D:\Games` and `build at 12:30` as ordinary text searches
  rather than queries for a property called `D` or `build at 12`.
- **A property query matches nothing else.** A project without that property
  never matches, however its name or path reads.
- **An empty value asks "does this project have the property at all?"** —
  the natural reading of typing `client:` and pausing.
- **Free text searches name, directory, tags and property *values*** — not
  property names. Case-insensitive throughout.
- **An empty query matches everything**, so callers need no special case.
- **`resolveView` does not sort.** Filtering is order-preserving, so whatever
  order the backend returned still holds inside every view.
- **Favourites is derived from the live list, not fetched.** Every comparator
  in `sorting.rs` ends in the unique id, so filtering a sorted list and sorting
  a filtered list give the same list — and deriving it means the sidebar count
  and the list cannot disagree.
- **Counts ignore the query.** The sidebar reports what each view holds, not
  what a transient filter leaves of it.
- **An empty group shows `0`, it does not vanish** — hence seeding the count
  map from every group before counting. The spec's invariant is that no view
  hides projects without saying so.
- **A project naming a group that no longer exists counts towards neither** its
  group nor Ungrouped. Transient; the next refetch resolves it.

**No regex.** `domain::matching` already sets the house style — *"Matched by
splitting the query and the path on `/` and comparing from the end, no regex."*
Splitting once on `:` and checking the name is 2+ characters with no whitespace
does the same job and takes no new dependency.

---

## 4. Where it lands in core

Suggested: `crates/core/src/domain/views.rs`, exported from `domain/mod.rs`,
with tests in `crates/core/src/tests/domain/views.rs`. It is pure domain logic
over `&[Project]` and `&[Group]` — no repository, no service, no I/O.

`View` becomes a Rust enum with `Group(String)` carrying the id, mirroring the
TypeScript union exactly.

**Note for the CLI side:** `commands::View` in the CLI is a *different, smaller*
type — `All`, `Favorites`, `Binned` — deliberately closed so clap can validate
it. It is not the same as core's `View` and should not be merged with it
without a decision: the CLI's is a `ValueEnum` (fixed set, no payload), and
core's carries a group id. When the CLI grows group views, the CLI type becomes
a `String` parsed into core's `View`, the same shape as `parseViewKey`.

---

## 5. The one hard part: the GUI is synchronous, IPC is not

This is why the task is bigger than it looks, and it is the decision to make
before writing frontend code.

`src/routes/+page.svelte` uses three `$derived`:

```svelte
const counts            = $derived(viewCounts(projects, deletedProjects, groups));
const knownPropertyKeys = $derived(propertyKeys([...projects, ...deletedProjects]));
const visibleProjects   = $derived(resolveView(selectedView, projects, deletedProjects, query));
```

Svelte 5 `$derived` is **synchronous**. Tauri `invoke()` returns a **Promise**.
A Promise cannot go in a `$derived`, so this is not a swap — the reactivity has
to change shape.

`visibleProjects` is the hot one: `query` changes on every keystroke.
`counts` and `knownPropertyKeys` only recompute when the data changes, so they
are far cheaper to move.

Options, none obviously right:

1. **`$effect` + async state, with race guarding.** Keep a request id and drop
   responses that arrive out of order — type fast enough and an older result
   can land after a newer one. Add a short debounce. Accept one stale frame
   between keystroke and result.
2. **Send ids, not projects.** The command takes the query and returns matching
   ids; the frontend filters its in-memory list. Smaller payload, same async
   problem.
3. **Move only the cold paths** (`viewCounts`, `propertyKeys`) and leave
   `matchesQuery` in TypeScript. Rejected as the end state — it leaves the
   duplication in the one function that matters — but a defensible first commit
   if the branch wants to land in stages.

Whatever is chosen, **the search box's feel is what is at risk**. Verify by
hand: type quickly into the search field and confirm results do not flicker,
lag visibly, or land out of order. That cannot be checked by CI.

---

## 6. Tauri commands

New commands in `src-tauri/src/commands/` (a `views.rs` alongside `groups.rs`
and `scan.rs`), registered in `mod.rs` and the `invoke_handler`. Keep them thin
— parse arguments, call core, return. The existing commands are the template.

Whatever shape they take, `src/lib/api/types.ts` gains matching types. That
file is already flagged as a hand-maintained mirror; keep it in step.

---

## 7. Definition of done

- [ ] Core module with the rules of §3, and Rust tests ported from
      `src/tests/lib/views.test.ts` — every case, not a subset
- [ ] Tauri commands, registered
- [ ] `src/lib/views.ts` reduced to `viewKey`, `parseViewKey`, `viewLabel`
- [ ] `+page.svelte`, `Sidebar.svelte`, `ViewControls.svelte` switched over
- [ ] `src/tests/lib/views.test.ts` trimmed to what still lives in TS
- [ ] `cargo test --workspace`, `cargo clippy --workspace --all-targets`,
      `cargo fmt --all -- --check`, and the frontend's typecheck and tests
- [ ] **By hand:** search box under fast typing; sidebar counts including an
      empty group showing `0`; `client:` with an empty value; a `D:\Games`-style
      query staying a text search
- [ ] `docs/cli/checklist.md` — tick the `views.ts` line in milestone 3

---

## 8. Conflict surface with the parallel CLI branch

The CLI branch is adding write commands (`edit`, tags, properties, favourite,
`restore`, `purge`). Expected overlap, worst first:

| File | Risk | Advice |
|---|---|---|
| `docs/cli/checklist.md` | **high** — both branches tick items in milestone 3 | Both sides: touch only your own line. Resolve by keeping both. |
| `crates/cli/CHANGELOG.md` | **high** — both add under `### Added` | Append-only; resolve by keeping both entries. |
| `crates/core/src/domain/mod.rs` | low | One added `pub mod` line each at most. |
| `docs/cli/agents.md` | low | The CLI branch adds command rows; this branch should not touch it. |
| `crates/cli/src/**` | **none expected** | This branch has no reason to edit the CLI crate. If it does, say so early. |
| `src/**`, `src-tauri/**` | **none expected** | The CLI branch has no reason to touch the frontend. |

The two branches are close to disjoint in code and overlap mainly in
documentation, which resolves by keeping both sides. Rebase on `main` before
opening the PR rather than at merge time.
