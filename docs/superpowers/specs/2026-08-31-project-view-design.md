# Project view — design

**Date:** 2026-08-31
**Status:** approved, ready for implementation planning

## Goal

Replace the cramped `ProjectDetailModal` with a dedicated route,
`/project/[id]`, that shows a project's identity plus one tab per detected
tracker (git, unreal, …), each rendered richly — links open externally, paths
reveal in the file explorer, ids/urls copy — and a strip showing every
registered detector's live outcome (matched / found nothing / errored).

## Non-goals (YAGNI)

- Bespoke per-tracker Svelte components. One generic `TrackerPanel` renders
  everything, type-driven.
- Persisted per-detector error history. Detection status is computed live on
  page load; only `refresh` persists trackers, exactly as today.
- A frontend service layer / stores refactor. `+page.svelte` keeps its shape;
  the new route owns its own small state.
- `GitInfo.contributors`, migration of stored `web_url` (re-derived on next
  refresh), a command-layer test harness (tracked separately in
  `architecture.md`).
- Keyboard tab navigation beyond what the existing tab markup already gives.

## Decisions locked during brainstorming

| Area | Decision |
|---|---|
| Container | Client-side route `/project/[id]`, replaces `ProjectDetailModal`. Back-nav to the list. |
| Page layout | Single column, stacked: back-nav → header → identity block → detection-status strip → tracker tabs → panel. |
| Detection on open | Live, read-only re-detect via a new `inspect_project` command. Does not persist. |
| Refresh | Existing `refresh_project_trackers` (all-or-nothing, persists) + re-inspect to repaint. |
| Panel style | One generic `TrackerPanel`, no per-tracker components. Field affordances driven by an inferred field *type*. |
| Field typing | Convention over the raw `Tracker` payload (see "Field-type convention"). No per-kind code in the frontend. |
| Remote URL | Backend-derived `GitInfo.web_url` (SSH→HTTPS, strip `.git`). Works for any project in git regardless of tracker kind. |
| Actions | Open links externally · copy to clipboard · per-tab re-detect · jump to Edit. |

## Backend (`src-tauri/src/`)

### `Detector::kind()`

New required trait method:

```rust
pub trait Detector: Send + Sync {
    /// Stable identity, lowercase, e.g. "git". Used to tag detection
    /// outcomes and to target a single detector on re-detect.
    fn kind(&self) -> &'static str;
    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError>;
}
```

`Gitector::kind` → `"git"`, `UnrealDetector::kind` → `"unreal"`. Trivial.

### `Detection` refactor (`detectors/runner.rs`)

Today: `Detection { trackers: Vec<Tracker>, errors: Vec<DetectorError> }`.

New:

```rust
pub struct Detection {
    /// One entry per registered detector, in registration order.
    pub outcomes: Vec<DetectorOutcome>,
}

pub enum DetectorOutcome {
    Detected   { kind: &'static str, tracker: Tracker },
    NotDetected { kind: &'static str },
    Failed     { kind: &'static str, error: DetectorError },
}

impl Detection {
    /// Trackers from detectors that matched, cloned, in registration order.
    pub fn trackers(&self) -> Vec<Tracker>;
    /// Errors from detectors that failed.
    pub fn errors(&self) -> Vec<&DetectorError>;
    /// All-or-nothing view — semantics UNCHANGED from today: `Ok(trackers)`
    /// only if nothing failed, else `Err` with the first failure and the
    /// partial trackers discarded. Still the documented "recorded decision".
    pub fn into_result(self) -> Result<Vec<Tracker>, DetectorError>;
}
```

`DetectorRunner::detect_project(path)` builds `outcomes` by running every
detector. Add:

```rust
/// Run one detector by kind (for per-tab re-detect), or all if `only` is None.
pub fn inspect(&self, path: &Path, only: Option<&str>) -> Detection;
```

`detect_project` becomes `self.inspect(path, None)`.

Call-site changes:
- `create_project`: `project.trackers = detection.trackers();` then
  `for e in detection.errors() { eprintln!(...) }`
- `refresh_project_trackers`: `detection.into_result()` (unchanged)
- `detect_project_trackers`: `detection.trackers()`
- Tests migrate to constructing `Detection { outcomes: vec![…] }`; the
  resilience test and both `into_result` tests keep their assertions.

### `GitInfo.web_url`

```rust
pub struct GitInfo {
    // … existing fields …
    /// Browser-openable form of `repo_url`, or None if it isn't a
    /// recognizable http/ssh git remote.
    pub web_url: Option<String>,
}
```

Derived in `Gitector::detect` via a private helper:

```rust
fn web_url(remote: &str) -> Option<String>
```

| input | output |
|---|---|
| `git@github.com:acme/repo.git` | `https://github.com/acme/repo` |
| `ssh://git@gitlab.com/acme/repo.git` | `https://gitlab.com/acme/repo` |
| `https://github.com/acme/repo.git` | `https://github.com/acme/repo` |
| `https://github.com/acme/repo` | `https://github.com/acme/repo` |
| `/srv/git/repo.git` (local path) | `None` |
| `` (empty) | `None` |

`Option<String>` → old stored `Tracker::Git` records without the key
deserialize as `None`; re-derived on next detect/refresh.

### `inspect_project` command (`commands/projects.rs`)

```rust
#[tauri::command]
pub fn inspect_project(
    app: AppHandle,
    detectors: State<'_, DetectorRunner>,
    id: String,
    only: Option<String>,      // re-detect a single kind
) -> Result<ProjectInspection, ProjectError>
```

- load project → `ProjectError::NotFound` if absent
- `Project::check_directory_health` — on failure, **do not** return `Err`;
  set `directory_status` and return an otherwise-empty inspection so the page
  can still render identity + a banner
- `detectors.inspect(dir, only.as_deref())`
- map `Detection.outcomes` → `results`

```rust
pub struct ProjectInspection {
    pub project: Project,
    pub directory_status: DirectoryStatusDto,   // Ok | Unavailable(String)
    pub results: Vec<DetectorResultDto>,
}

pub struct DetectorResultDto {
    pub kind: String,
    pub status: DetectorStatusDto,              // "detected" | "not_detected" | "failed"
    pub tracker: Option<Tracker>,               // Some iff detected
    pub error: Option<String>,                  // Some iff failed
}
```

Serialization: `DetectorStatusDto` as a lowercase string tag;
`DirectoryStatusDto` as `{ ok: true }` / `{ ok: false, message }` (or serde
enum — implementer's call, mirror in `types.ts`).

Register in `lib.rs` `invoke_handler`.

No new `ProjectError` variants. Detector failures live in `results`, never in
the command's `Err`.

## Frontend (`src/`)

### Route `src/routes/project/[id]/+page.svelte` (+ `+page.ts`)

- `+page.ts`: `export const prerender = false;` (dynamic param under
  adapter-static + SPA fallback). `ssr` already false globally via
  `+layout.ts`.
- Read the id: `import { page } from "$app/stores"` → `$page.params.id`
  (`@sveltejs/kit ^2.9` — `$app/state` may not be available; use the store).
- On mount / when id changes: `inspectProject(id)`; hold
  `inspection = $state<ProjectInspection | null>(null)`, `error`, `loading`.
- **Header:** `← All projects` (`<a href="/">`), project name, buttons
  `Open` (`openProjectDirectory(id)`, reuse missing-app handling from
  `opener.ts`), `Edit`, `Refresh`.
- **Identity block:** lift the `<dl>` markup out of `ProjectDetailModal`
  before deleting it.
- **Detection-status strip:** map `inspection.results`:
  - `detected` → `● {kind}` green
  - `not_detected` → `○ {kind} — not detected` dim
  - `failed` → `▲ {kind} — {error}` red
- **Tabs:** `results.filter(status === "detected")`; tab bar + `TrackerPanel`
  for the active one; each tab has a small `re-detect` button →
  `inspectProject(id, { only: kind })`, splice the returned result in.
- **Edit overlay:** render `EditProjectForm` (existing, self-contained) in a
  modal container on this page; `onSaved` → re-inspect, close.
- **Refresh:** `refreshProjectTrackers(id)`; then `inspectProject(id)` to
  repaint regardless of outcome; a thrown refresh error → `ErrorBanner`.
- **Directory unavailable** (`directory_status` not ok): identity block +
  banner, no tabs.
- **Not found** (`inspectProject` throws NotFound): a small "project not
  found" block + `← All projects`.

### `src/lib/components/TrackerPanel.svelte` (new)

Props: `{ tracker: Tracker }`. Renders `trackerFields(tracker)` as a
`<dl>`-style list; per row, switch on `field.type`:

| type | rendering |
|---|---|
| `text` | value as text |
| `code` | `<code>` monospace + `⧉ copy` button |
| `link` | `<a>` (label = value) + `↗ open` (→ `openExternalUrl`) + `⧉ copy` |
| `path` | value + `📂 reveal` (→ `revealPath`) + `⧉ copy` |
| `chips` | inline chip row; row omitted if the array is empty |
| `flag` | a warning-toned badge showing the humanized key; **rendered only when the value is `true`**, omitted when `false` |

`copy` → `navigator.clipboard.writeText(value)`, failures swallowed (console
only). If the webview blocks `navigator.clipboard`, fall back to adding
`@tauri-apps/plugin-clipboard-manager` (note for implementer; not assumed).

### `src/lib/trackers.ts` — upgrade `trackerFields`

Return `Array<{ label: string; type: FieldType; value: string | string[] }>`
where `FieldType = "text" | "code" | "link" | "path" | "chips" | "flag"`.

Inference, applied per `[key, value]` of the tracker payload, first match wins:

1. `typeof value === "boolean"` → `flag`
2. `Array.isArray(value)` → `chips`
3. key matches `/(^url$|_url$)/i` **or** string value matches
   `/^(https?:\/\/|git@|ssh:\/\/)/` → `link`
4. key matches `/(^path$|_path$|_root$|_dir$|directory)/i` → `path`
5. key matches `/(hash|commit)/i` → `code`
6. otherwise → `text`

Empty string / null values: row omitted (as today). Empty arrays: omitted.
`trackerKind()` is unchanged. This is the **only** place field semantics
live, and it has **no per-tracker-kind branching**.

`web_url` (a `link`) and `repo_url` (also a `link` — matches rule 3 by key
and/or value) both render as rows: one clickable (`web_url` resolves to
`https://`), one effectively copy-only if it's an `ssh` form. No special
casing; acceptable minor redundancy.

### `src/lib/api/`

- `projects.ts`: `inspectProject(id: string, opts?: { only?: string }): Promise<ProjectInspection>`
- `opener.ts`: `openExternalUrl(url: string): Promise<void>` and
  `revealPath(path: string): Promise<void>` wrapping
  `@tauri-apps/plugin-opener` (`openUrl`, `revealItemInDir`).
- `types.ts`: `GitInfo.web_url: string | null`; new `ProjectInspection`,
  `DetectorResult`, `DetectorStatus`, `DirectoryStatus`.

### Tauri capabilities

`src-tauri/capabilities/*.json` already grants `opener:default` (covers
`open-url`/`open-path`). Add `opener:allow-reveal-item-in-dir` if
`revealItemInDir` isn't in the default set (verify during implementation).

### Removed / changed

- `ProjectDetailModal.svelte` — **deleted** (identity `<dl>` moved into the
  route first).
- `+page.svelte` — drop `detailTargetId`, `detailTarget`, `handleShowDetails`,
  `handleCloseDetails`, the `{#if detailTarget}` block, the `ProjectDetailModal`
  import.
- `ProjectCard.svelte` — `onShowDetails` prop removed; the "Details" button
  becomes `<a href={\`/project/${project.id}\`}>` styled as a button.
- `ProjectList.svelte` — drop the `onShowDetails` pass-through.
- `FavoritesModal` / `BinModal` — if they render `ProjectCard`, the Details
  link still works (navigation closes the modal implicitly on route change);
  verify no `onShowDetails` prop is still required.

## Data flow

```
/project/[id] mount
  → inspectProject(id)                          [inspect_project]
      load Project (NotFound → error state)
      check_directory_health → directory_status
      DetectorRunner.inspect(dir, None)         → Detection { outcomes }
      → ProjectInspection { project, directory_status, results }
  → render identity + status strip + tabs(detected) + TrackerPanel

Refresh
  → refreshProjectTrackers(id)                  [persist, all-or-nothing]
  → inspectProject(id)                          [repaint, always]
  → thrown error → ErrorBanner

Per-tab re-detect
  → inspectProject(id, { only: kind })          → replace that one result

Edit → EditProjectForm overlay → onSaved → inspectProject(id) → close
Open → openProjectDirectory(id)                 [existing, missing-app aware]

row "↗ open"  → openExternalUrl(value)
row "📂 reveal" → revealPath(value)
row "⧉ copy"   → navigator.clipboard.writeText(value)
```

## Error handling

| Situation | Behaviour |
|---|---|
| project id not found | route shows "project not found" + back link |
| directory deleted/moved | `directory_status: unavailable(msg)`; identity block + banner, no tabs |
| a detector errors during inspect | shown in the status strip (`▲ kind — msg`); that tab is absent |
| `refresh_project_trackers` fails (all-or-nothing) | `ErrorBanner` on the page; live inspect view unchanged |
| `navigator.clipboard` throws | swallowed, console warning |
| `openExternalUrl` / `revealPath` throws | `ErrorBanner` |

## Testing

**Rust unit:**
- `web_url()` — the table above (~6 cases incl. `None` paths)
- `Detection` — `trackers()` / `errors()` / `into_result()` after the
  `outcomes` refactor; migrate `one_detector_failing_keeps_the_others_results`,
  `into_result_returns_every_tracker_when_no_detector_failed`,
  `into_result_discards_partial_trackers_on_any_error`
- `DetectorRunner::inspect` with `only = Some("git")` runs just that detector;
  `only = None` runs all; `only = Some("nonsense")` → empty `outcomes`
- `Detector::kind()` for both detectors

**No** command-layer test for `inspect_project` (harness doesn't exist;
`architecture.md` backlog).

**Frontend:** `svelte-check` clean. Manual via the `run` skill: open a project
that is git + (fake) unreal, verify tabs, status strip, copy, open-link,
reveal-path, per-tab re-detect, Refresh (success + a forced failure), Edit
round-trip, back-nav, a project whose directory was deleted, a bad id in the
URL.

## Doc & invariant updates (part of this work)

- `architecture.md`
  - invariant #1 reworded: a new detector needs `kind()`, a `Tracker` variant,
    an `*Info` model **with fields named per the display convention**, one
    `registry.rs` line — and **zero frontend code**. Spell out the convention
    (`*_url`→link, `*_path`/`*_root`→path, `*hash*`→code, bool→flag,
    array→chips).
  - check off "explicit tracker/detector identity" and "per-detector status
    to the UI" in the backlog
  - `Detection` "recorded decision" doc-comment reference: `{trackers,errors}`
    → `outcomes`
- `knowledgebase.md` — "Tracker detection, end to end" and frontend sections:
  `/project/[id]` route replaces `ProjectDetailModal`; `inspect_project`;
  `web_url`; the `TrackerPanel` / field-type convention
- `checklist.md` — new "Project view" section
- `accomplishments.md` — dated entry

## Risks / notes

- **Field-type convention is implicit.** Mitigated: small, documented,
  degrades to `text`. A detector author who names a URL field `remote` instead
  of `remote_url` loses the link affordance until renamed — acceptable.
- **`$app/stores` vs `$app/state`** — using the store form for
  `@sveltejs/kit ^2.9`. If Kit is bumped later, migrate.
- **`navigator.clipboard` in the Tauri webview** — expected to work; plugin
  fallback identified if not.
- **adapter-static + dynamic route** — `prerender = false` on `+page.ts`
  should satisfy the build; verify `npm run build` early.
- **Detection refactor touches freshly-committed code** — the `into_result`
  recorded decision and its tests are preserved by design; re-run them.
