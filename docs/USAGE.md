# Using Project Indexer

A walkthrough of the app as you actually meet it. For what it is and how to
install it, see the [README](../README.md); for how to work on it, see
[CONTRIBUTING](../CONTRIBUTING.md).

## Contents

- [The main window](#the-main-window)
- [Adding a project](#adding-a-project)
- [What gets detected](#what-gets-detected)
- [The project view](#the-project-view)
- [Opening a project](#opening-a-project)
- [Organising: favourites, tags, sorting](#organising-favourites-tags-sorting)
- [Editing a project](#editing-a-project)
- [Removing a project](#removing-a-project)
- [When a directory goes missing](#when-a-directory-goes-missing)
- [The system tray](#the-system-tray)
- [Where your data lives](#where-your-data-lives)

## The main window

Three regions: a **New project** form at the top, sort controls, and the list of
everything you track. Two icons sit in the header — a **★** opening your
favourites and a **bin** opening deleted projects.

## Adding a project

Fill in the form and press **Create project**.

- **Name** — required, and must be unique.
- **Directory** — required. **Browse…** opens a native folder picker.
- **Description** — optional.
- **Tags** — optional, comma separated.

**The name fills itself in.** When you browse to a directory and the Name field
is still empty, the app suggests one: the repository name from the git remote if
there is one, otherwise the folder name. It only ever fills a blank field, so it
will not overwrite something you typed.

Detection runs when the project is created. If a detector fails, the project is
still created — the failure is reported rather than silently swallowed.

## What gets detected

| Tracker | Found by | Reports |
|---|---|---|
| **Git** | a repository at or above the directory | current branch, dirty state, detached HEAD, branch list, current commit, remote URL and a browser-openable form of it |
| **Unreal Engine** | a `.uproject` file in the directory | project name, engine association, category, description, modules, enabled plugins, configured source-control provider |

Detectors are independent. A directory can be both at once, and each is reported
separately. Note the asymmetry: git is discovered *upwards* — a subdirectory of a
repository still counts — while the `.uproject` file must be in the directory
itself.

A detector that fails is shown as **failed**, never as "nothing found". A
malformed `.uproject` is not quietly reported as "not an Unreal project".

Detected trackers appear as coloured badges on the project card. Each tracker
kind gets its own consistent colour.

## The project view

**Details** on a project's `···` menu opens its own page: identity at the top,
then a status strip showing every detector that was consulted and what it
concluded, then one tab per detected tracker.

- Detection here is **live and read-only** — it runs when you open the page and
  does not change what is stored.
- **Refresh** is the action that persists a new detection result.
- Each tab can be re-detected on its own.
- Detectors that found nothing collapse into a **Not detected (N)** disclosure so
  they do not crowd out the ones that did.
- **Edit** opens over the page.

Fields inside a tracker tab are rendered according to what they are. A
`https://` remote becomes a link; an `ssh://` or `git@` remote becomes copyable
text rather than a broken link; paths get open and reveal buttons; commit hashes
and other identifiers are copyable; lists render as chips.

## Opening a project

**Open** on the `···` menu launches the project. What that does depends on the
project's **Open with** setting, chosen in Edit:

- **An application** — picked from a list of what you actually have installed.
  On Windows that comes from Start Menu shortcuts and registry App Paths; on
  Linux from `.desktop` entries in the standard XDG directories, so it matches
  what your desktop launcher shows, Flatpak and Snap included.
- **Nothing set** — the project opens in your system file manager.

If the configured application has since been uninstalled or moved, opening tells
you that specifically rather than failing generically.

Opening a project updates its last-opened time, which is what the default sort
orders by.

## Organising: favourites, tags, sorting

- **Favourites** — a checkbox in Edit. Favourites show a **★** on the card, and
  the header's star icon lists them on their own.
- **Tags** — comma separated, free-form.
- **Sorting** — by **Name** or **Last opened**, in either direction. The arrow
  button beside the dropdown flips it.

## Editing a project

**Edit** on the `···` menu. Beyond the creation fields, editing exposes:

- **Favorite** — the star.
- **Client** — free text, for work done for someone else.
- **Notes** — free text.
- **Open with** — the application picker described above.

## Removing a project

**Delete** opens a dialog with two clearly different outcomes. Read it before
confirming, because the default is the destructive one.

- **Delete the directory from disk** — *selected by default*. This removes the
  actual folder and cannot be undone. By default the project record itself is
  kept in the bin, so the entry is recoverable even though the files are not.
  A further checkbox, **Also permanently forget this project**, drops the record
  too.
- **Just remove it from this app** — untracks it. The folder on disk is left
  completely untouched, and you can re-add it later by creating a project
  pointed at the same path.

The **bin** icon in the header lists soft-deleted projects and offers
**Restore**.

## When a directory goes missing

If a project's folder is deleted or moved out from under the app, its card shows
an amber marker and the path struck through. The project is not removed — the
record stays so you can fix the path in Edit or untrack it deliberately.

A directory that merely cannot be read right now — an unmounted drive, a
permissions problem — is not treated as gone.

## The system tray

Closing the window does not quit. The app hides to the system tray and keeps
running.

- **Left-click** the tray icon to bring the window back.
- **Right-click** for **Show** and **Quit**. Quit is the real exit.
- Launching the app again while it is hidden also brings the window forward
  rather than starting a second copy.

**On Linux the tray needs an appindicator library.** If one is not installed the
app still runs and prints a message naming the package — and in that case closing
the window genuinely quits, since there would be no tray to restore it from. The
per-distribution package is in the README's
[Linux notes](../README.md#linux-notes).

## Where your data lives

One SQLite file:

| Platform | Path |
|---|---|
| Windows | `%APPDATA%\com.shaer.project-indexer\projects.db` |
| macOS | `~/Library/Application Support/com.shaer.project-indexer/projects.db` |
| Linux | `~/.config/com.shaer.project-indexer/projects.db` |

It is per-machine and not synced, so projects tracked on one computer do not
appear on another. Writes are synchronous — there is nothing buffered to lose if
the app exits unexpectedly.

The file is deliberately a plain, readable SQLite database rather than an opaque
store, so other tools can read it. It holds paths, names, and labels — not
credentials.
