# Project Indexer — Known Issues

_Written 2026-08-28 from a Linux build-and-run pass. Reflects `main` at `9761e80` plus the fix on `fix/select-color-scheme` (`761d848`)._

Four issues were found while getting the Windows-developed app compiling and
running on Linux. They are labelled `PI-001`–`PI-004` and carry deliberately
different dispositions: one was a real user-visible defect, one is cosmetic
log noise, one is a linter false positive, and one is an inaccurate comment.

| ID | Issue | Severity | Status |
|----|-------|----------|--------|
| PI-001 | Sort dropdown unreadable on Linux | Medium — user-visible | **Fixed** |
| PI-002 | Stale `filesystem.ts` 404 in dev log | Trivial — cosmetic | No action needed |
| PI-003 | `state_referenced_locally` warnings ×8 | None — false positive | Not a defect |
| PI-004 | NVIDIA comment understates its own scope | Trivial — comment accuracy | Open |

Nothing here blocks the Linux build. `cargo check`, `cargo test` (61 passed),
`pnpm build` and `pnpm tauri build` all complete cleanly, and the app runs.

---

## PI-001 — Sort dropdown renders unreadable on Linux

**Severity:** Medium (user-visible) · **Status:** Fixed in `761d848` · **Platform:** Linux; latent on Windows

The sort `<select>` in the default view painted as a solid white box with
near-white text, making the selected option invisible. The `↓` direction
toggle beside it was fine, as were every `<input>` in the same form.

**Cause.** The page never declared `color-scheme`. The engine therefore
assumed `light` and painted the `<select>` as a *native light menulist*,
overriding the CSS background — while Tailwind's `dark:text-gray-100` still
applied near-white text to it. The dark dropdown arrow was the giveaway: that
glyph is drawn by the engine's native theme, not by page CSS.

**Why only the select.** `<input>` is not drawn as a native menulist, so
`dark:bg-gray-800` applied to it normally. That is why the Name, Directory,
Description and Tags fields themed correctly in the very same form.

**Why Windows looked fine.** That desktop is light-themed, so
`prefers-color-scheme: light` selected `bg-white` + `text-gray-900`, which
happened to agree with the light native widget. Coincidence, not correctness —
the bug reproduces on Windows switched to dark mode.

**Fix** — `src/app.css`:

```css
html {
  color-scheme: light dark;
}
```

Chosen over `appearance: none` plus a hand-drawn arrow because it addresses the
root cause and covers every native widget at once — scrollbars, spinners, and
the `<select>`s in `BinModal.svelte:104` and `FavoritesModal.svelte:103` — rather
than patching one control and leaving the next to be rediscovered.

**Verified** by rebuilding the release binary and screenshotting the running
app: the control now reads "Last opened" on a dark ground.

**Update (GUI v1, `25eda04`):** the app is now a single committed dark theme
(`color-scheme: dark`, semantic `@theme` tokens, all `<select>`s styled
explicitly). There's no light-mode path left to regress into, so this class
of bug is retired rather than just patched.

---

## PI-002 — Stale `filesystem.ts` 404 in the dev log

**Severity:** Trivial (cosmetic) · **Status:** No action needed · **Scope:** dev server only

```
[404] GET /src/lib/api/filesystem.ts
```

`src/lib/api/filesystem.ts` was deleted in `7f8d8ae`. Nothing references it any
more — a sweep of `.ts`, `.js`, `.svelte`, `.json` and sourcemaps returns only
the unrelated Rust file `crates/core/src/platform/filesystem.rs` (moved there
from `src-tauri/src/utils/` in the frontend-agnostic-core refactor).

The request comes from the WebKit webview's cached module graph, left over from
a dev session predating the deletion. It appears only in the Vite dev log, never
in a production build — which is why `pnpm build` succeeds cleanly. It clears
itself once that cache is evicted.

---

## PI-003 — `state_referenced_locally` warnings in EditProjectForm

**Severity:** None (linter false positive) · **Status:** Not a defect · **Location:** `EditProjectForm.svelte:20-27`

Eight warnings of the form:

```
This reference only captures the initial value of `project`.
Did you mean to reference it inside a derived instead?
```

raised against the field seeds:

```js
let name = $state(project.name);
```

Svelte flags this in case `$derived` was intended. For a form the one-time seed
is correct — fields must not snap back while someone is typing. Three things
confirm it is safe here:

1. **Switching projects gives fresh values.** The form is mounted at
   `ProjectList.svelte:46` under `{#if editingId === project.id}` inside a keyed
   `{#each ... (project.id)}`. Editing a different project destroys the old
   component and constructs a new one, re-running every initializer.
2. **The form closes before any reload.** `handleSaved` (`+page.svelte:66`) and
   `handleCancelEdit` (`:62`) both clear `editingId` first; deleting the edited
   project clears it at `:90`. There is no polling or `setInterval`.
3. **A stale capture cannot clobber backend data.** The save payload
   (`EditProjectForm.svelte:41-50`) carries only user-editable fields — name,
   directory, description, tags, favorite, notes, client, open_with. Fields the
   backend manages, such as `tracker` and `last_opened_at`, are not in it.

The one case where `project` updates while the form stays mounted — another
card triggering `loadProjects()` — is precisely when preserving in-progress
typing is the desired behaviour.

**If the noise is unwanted,** state the intent explicitly rather than
restructuring:

```js
import { untrack } from "svelte";
let name = $state(untrack(() => project.name));
```

This silences the warning without changing behaviour. Purely cosmetic.

---

## PI-004 — NVIDIA workaround comment understates its own scope

**Severity:** Trivial (comment accuracy) · **Status:** Open · **Location:** `src-tauri/src/lib.rs:32`

The comment reads:

> Both paths are created by the proprietary kernel module only, so this is
> distro-independent — no package or driver-version probing needed.

The **open** kernel module creates `/proc/driver/nvidia/version` and
`/sys/module/nvidia/version` too. The test machine runs
`NVIDIA UNIX Open Kernel Module 610.57.04` and both paths exist, so
`disable_dmabuf_renderer_on_nvidia()` engages there.

That is the right outcome — the open module still uses the proprietary
userspace GL stack that has the GBM allocation failure — but the comment
describes a narrower trigger than the code actually has, which could mislead
someone later into "fixing" the detection. The code needs no change; the
wording does.

---

## Verification environment

| | |
|---|---|
| OS | Arch Linux, kernel 7.1.9 |
| Session | Hyprland / Wayland |
| GTK theme | `Adwaita-dark`, `color-scheme: prefer-dark` |
| GPU | NVIDIA UNIX Open Kernel Module 610.57.04 |
| WebKitGTK | 2.52.6 |
| Rust | 1.94.0 |
| Node / pnpm | 26.7.0 / 10.32.1 |

The GTK theme matters: `prefer-dark` is what puts the page into dark mode and
exposes PI-001. On a light-themed desktop the app looks correct and the defect
stays hidden.
