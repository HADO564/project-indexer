# Groups, migration and icon sanitizing — backend implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the entire backend half of the project views/grouping feature — the `Group` entity, the project's first schema migration with fixtures, and a trust-boundary SVG sanitizer with an icon store — so the frontend half can be built against a finished API.

**Architecture:** Everything lands in `indexer-core`, the Tauri-free library crate, with `src-tauri` as a thin command adapter. Groups follow the existing ports pattern (`GroupReader`/`GroupRepository` mirroring `ProjectReader`/`ProjectRepository`), and storage follows the house rule that the JSON blob is the truth and columns are queryable mirrors. The sanitizer is pure logic in core with no filesystem or Tauri knowledge.

**Tech Stack:** Rust 2021, `rusqlite` 0.32 (bundled SQLite), `serde`/`serde_json`, `chrono`, `uuid`, `thiserror`, `quick-xml` (new), Tauri 2.

**Spec:** [`docs/superpowers/specs/2026-09-05-project-views-grouping-design.md`](../specs/2026-09-05-project-views-grouping-design.md)

## Global Constraints

- **`indexer-core` must never `use tauri`.** The compiler enforces it. Anything that knows about webviews, `invoke` or `AppHandle` belongs in `src-tauri`.
- **The JSON blob is the truth; columns are queryable mirrors.** `data` holds the whole `Project`; `is_deleted`, `directory_normalized`, `updated_at` and now `group_id` are duplicated into columns for querying. Both are written in the same statement.
- **A new `Project` field must be `Option<T>` or `#[serde(default)]`,** or every stored record fails to load. Guarded by `loads_a_record_missing_every_absorbable_field`.
- **One public application error:** `ProjectError`. Port-level errors (`RepositoryError`, the new `IconError`) map into it via `From`. Do not create a parallel hierarchy.
- **The app is the only writer of `projects.db`.** devmon attaches it read-only.
- **`CURRENT_SCHEMA_VERSION` ends this plan at `2`.** `open` must still refuse a database written by a newer binary.
- **Commit trailer:** every commit ends with `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>` (name whichever model you are).
- **Pre-commit hook:** already installed on this clone (`core.hooksPath = .githooks`). It runs `cargo fmt --check`, `cargo clippy --workspace --all-targets` and `cargo test --workspace` for Rust changes. Clippy has a **two-warning baseline**: `module-inception`, and `unnecessary_sort_by` at `crates/core/src/platform/app_discovery.rs:116`. Both pre-date this plan and neither is in a file it touches — leave them alone and do not add a third. (Older docs, including the 2026-09-04 plugins handoff, still say one warning; that is stale.)
- **Test counts today:** 105 Rust tests written, 102 run on Linux, 94 on Windows. Quote the number for the platform you are on.

---

### Task 1: `Group` domain type, `UpdateGroup`, and the swatch palette

**Files:**
- Create: `crates/core/src/domain/palette.rs`
- Create: `crates/core/src/domain/group.rs`
- Create: `crates/core/src/domain/update_group.rs`
- Modify: `crates/core/src/domain/mod.rs`
- Modify: `crates/core/src/error/project_error.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: nothing (first task).
- Produces: `Group { id: String, name: String, color: String, icon: String, position: i64, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`; `Group::new(name: String, color: String, icon: String, position: i64) -> Result<Group, ProjectError>`; `Group::check_for_duplicate_name(name: &str, existing: &[Group]) -> Result<(), ProjectError>`; `Group::update(&mut self, update: UpdateGroup) -> Result<(), ProjectError>`; `UpdateGroup { name: Option<String>, color: Option<String>, icon: Option<String> }`; `palette::SWATCHES: [&str; 8]`; `palette::is_known_swatch(name: &str) -> bool`; new `ProjectError` variants `InvalidGroupName`, `InvalidGroupIcon`, `UnknownSwatch(String)`, `DuplicateGroupName(String)`, `GroupNotFound(String)`.

**Note on validation scope.** The spec says group `colour and icon must be known names`. Colour is validated in core, which owns the palette list. **Icon is validated only as non-empty**, because the bundled icon set lives in the frontend and duplicating 24 names in Rust would be a silent drift hazard; an unknown icon name falls back to a default glyph at render, matching the ignore-what-you-do-not-know rule used everywhere else. Step 8 amends the spec so the two documents agree.

- [ ] **Step 1: Write the failing tests for the palette**

Create `crates/core/src/domain/palette.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_swatches_are_recognized() {
        assert!(is_known_swatch("cyan"));
        assert!(is_known_swatch("pink"));
    }

    #[test]
    fn unknown_swatch_is_rejected() {
        assert!(!is_known_swatch("chartreuse"));
        assert!(!is_known_swatch("#e7b64e"));
        assert!(!is_known_swatch(""));
    }

    #[test]
    fn there_are_eight_swatches_and_they_are_unique() {
        let mut sorted = SWATCHES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 8);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p indexer-core palette`
Expected: FAIL — `cannot find value SWATCHES in this scope`.

- [ ] **Step 3: Implement the palette**

At the top of `crates/core/src/domain/palette.rs`, above the test module:

```rust
/// The swatch names a project or group colour may take.
///
/// Stored values are these bare names, never literal colours — `src/lib/palette.ts`
/// resolves each to a `var(--color-swatch-*)` token at render, so a theme swap
/// recolours every project and group coherently. That file is a hand-maintained
/// mirror of this list; change one, change the other.
pub const SWATCHES: [&str; 8] = [
    "cyan", "gold", "amber", "rust", "violet", "green", "blue", "pink",
];

pub fn is_known_swatch(name: &str) -> bool {
    SWATCHES.contains(&name)
}
```

Register the module in `crates/core/src/domain/mod.rs` by adding `pub mod palette;` alongside the existing `pub mod naming;`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p indexer-core palette`
Expected: PASS (3 tests).

- [ ] **Step 5: Add the new error variants**

In `crates/core/src/error/project_error.rs`, add these variants inside `enum ProjectError`, after `Detection`:

```rust
    #[error("Group name cannot be empty")]
    InvalidGroupName,

    #[error("Group icon cannot be empty")]
    InvalidGroupIcon,

    #[error("Not a known colour: {0}")]
    UnknownSwatch(String),

    #[error("A group with this name already exists: {0}")]
    DuplicateGroupName(String),

    #[error("Group with id '{0}' not found")]
    GroupNotFound(String),
```

- [ ] **Step 6: Write the failing tests for `Group`**

Create `crates/core/src/domain/group.rs` containing only this test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Group {
        Group::new("Client work".into(), "cyan".into(), "briefcase".into(), 0)
            .expect("valid group")
    }

    #[test]
    fn new_group_trims_its_name_and_stamps_timestamps() {
        let g = Group::new("  Client work  ".into(), "cyan".into(), "briefcase".into(), 3)
            .expect("valid group");
        assert_eq!(g.name, "Client work");
        assert_eq!(g.position, 3);
        assert_eq!(g.created_at, g.updated_at);
        assert!(!g.id.is_empty());
    }

    #[test]
    fn rejects_an_empty_name() {
        let result = Group::new("   ".into(), "cyan".into(), "briefcase".into(), 0);
        assert!(matches!(result, Err(ProjectError::InvalidGroupName)));
    }

    #[test]
    fn rejects_an_unknown_colour() {
        let result = Group::new("Personal".into(), "#ff0000".into(), "briefcase".into(), 0);
        assert!(matches!(result, Err(ProjectError::UnknownSwatch(_))));
    }

    #[test]
    fn rejects_an_empty_icon() {
        let result = Group::new("Personal".into(), "cyan".into(), "  ".into(), 0);
        assert!(matches!(result, Err(ProjectError::InvalidGroupIcon)));
    }

    #[test]
    fn duplicate_name_check_ignores_case_and_surrounding_space() {
        let existing = vec![sample()];
        let result = Group::check_for_duplicate_name("  CLIENT WORK ", &existing);
        assert!(matches!(result, Err(ProjectError::DuplicateGroupName(_))));
    }

    #[test]
    fn duplicate_name_check_allows_a_distinct_name() {
        let existing = vec![sample()];
        assert!(Group::check_for_duplicate_name("Personal", &existing).is_ok());
    }

    #[test]
    fn update_applies_only_the_fields_present() {
        let mut g = sample();
        let before = g.color.clone();
        g.update(UpdateGroup {
            name: Some("Renamed".into()),
            color: None,
            icon: None,
        })
        .expect("valid update");
        assert_eq!(g.name, "Renamed");
        assert_eq!(g.color, before);
    }

    #[test]
    fn update_rejects_an_unknown_colour_and_changes_nothing() {
        let mut g = sample();
        let result = g.update(UpdateGroup {
            name: Some("Renamed".into()),
            color: Some("chartreuse".into()),
            icon: None,
        });
        assert!(matches!(result, Err(ProjectError::UnknownSwatch(_))));
        assert_eq!(g.name, "Client work");
    }
}
```

- [ ] **Step 7: Run the tests to verify they fail**

Run: `cargo test -p indexer-core group`
Expected: FAIL — `cannot find struct Group`.

- [ ] **Step 8: Implement `Group`, `UpdateGroup`, and amend the spec**

Create `crates/core/src/domain/update_group.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Partial update: a field left `None` is unchanged.
///
/// Unlike `UpdateProject`, no field here is nullable — a group always has a
/// name, a colour and an icon — so these are plain `Option<T>` rather than the
/// double-option "absent vs explicit null" shape.
///
/// `position` is deliberately absent: reordering goes through
/// `GroupRepository::set_group_positions`, which rewrites the whole ordering,
/// so there is exactly one path that renumbers.
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateGroup {
    pub name: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
}
```

At the top of `crates/core/src/domain/group.rs`, above the test module:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::palette::is_known_swatch;
use crate::domain::update_group::UpdateGroup;
use crate::error::ProjectError;

/// A user-defined band of projects, surfaced as one entry in the sidebar.
///
/// Membership is exclusive — a project has zero or one group — which is what
/// makes every project appear under exactly one entry. Tags remain the
/// non-exclusive mechanism.
///
/// `icon` names a glyph from the frontend's bundled set. It is validated only
/// as non-empty here: core does not own that list, and an unknown name falls
/// back to a default glyph at render rather than failing, the same
/// forward-compatibility rule the `--json` contract uses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub position: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Group {
    fn validate_name(name: &str) -> Result<(), ProjectError> {
        if name.trim().is_empty() {
            return Err(ProjectError::InvalidGroupName);
        }
        Ok(())
    }

    fn validate_color(color: &str) -> Result<(), ProjectError> {
        if !is_known_swatch(color) {
            return Err(ProjectError::UnknownSwatch(color.to_string()));
        }
        Ok(())
    }

    fn validate_icon(icon: &str) -> Result<(), ProjectError> {
        if icon.trim().is_empty() {
            return Err(ProjectError::InvalidGroupIcon);
        }
        Ok(())
    }

    pub fn new(
        name: String,
        color: String,
        icon: String,
        position: i64,
    ) -> Result<Self, ProjectError> {
        Self::validate_name(&name)?;
        Self::validate_color(&color)?;
        Self::validate_icon(&icon)?;

        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            name: name.trim().to_string(),
            color,
            icon: icon.trim().to_string(),
            position,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn check_for_duplicate_name(name: &str, existing: &[Group]) -> Result<(), ProjectError> {
        if existing
            .iter()
            .any(|g| g.name.trim().eq_ignore_ascii_case(name.trim()))
        {
            return Err(ProjectError::DuplicateGroupName(name.trim().to_string()));
        }
        Ok(())
    }

    /// Validates every present field *before* assigning any of them, so a
    /// rejected update leaves the group exactly as it was.
    pub fn update(&mut self, update: UpdateGroup) -> Result<(), ProjectError> {
        if let Some(name) = &update.name {
            Self::validate_name(name)?;
        }
        if let Some(color) = &update.color {
            Self::validate_color(color)?;
        }
        if let Some(icon) = &update.icon {
            Self::validate_icon(icon)?;
        }

        if let Some(name) = update.name {
            self.name = name.trim().to_string();
        }
        if let Some(color) = update.color {
            self.color = color;
        }
        if let Some(icon) = update.icon {
            self.icon = icon.trim().to_string();
        }
        self.updated_at = Utc::now();
        Ok(())
    }
}
```

In `crates/core/src/domain/mod.rs`, add `pub mod group;` and `pub mod update_group;` to the module list, and `pub use group::Group;` plus `pub use update_group::UpdateGroup;` to the re-exports.

In `crates/core/src/lib.rs`, extend the domain re-export line to include the two new types:

```rust
pub use domain::{
    GitInfo, Group, InstalledApp, Project, Tracker, UnrealInfo, UpdateGroup, UpdateProject,
};
```

Then amend the spec so it matches the validation decision. In `docs/superpowers/specs/2026-09-05-project-views-grouping-design.md`, replace `colour and icon must be known names` with:

```
colour must be a known palette name, and icon must be non-empty — core does
not own the bundled icon list, so an unknown icon name falls back to a default
glyph at render rather than failing
```

- [ ] **Step 9: Run the tests to verify they pass**

Run: `cargo test -p indexer-core group`
Expected: PASS (8 tests).

- [ ] **Step 10: Run the full gate and commit**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets && cargo test --workspace`
Expected: clippy shows only the known `module-inception` warning; all tests pass.

```bash
git add crates/core/src/domain/palette.rs crates/core/src/domain/group.rs \
        crates/core/src/domain/update_group.rs crates/core/src/domain/mod.rs \
        crates/core/src/error/project_error.rs crates/core/src/lib.rs \
        docs/superpowers/specs/2026-09-05-project-views-grouping-design.md
git commit -m "feat(core): add the Group domain type and swatch palette

A group is a user-defined band of projects with a name, a palette colour,
a bundled-icon name and a sidebar position. Membership is exclusive, so
colour and icon exist to make one entry distinguishable at a glance.

Colours are stored as bare swatch names rather than literals so a theme
swap recolours everything coherently; the eight names are mirrored in
src/lib/palette.ts. Icons are validated only as non-empty, because core
does not own the bundled icon list and an unknown name should fall back
to a glyph rather than fail — the spec is amended to match.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 2: Three new `Project` fields and the `UpdateProject` surface

**Files:**
- Modify: `crates/core/src/domain/project.rs`
- Modify: `crates/core/src/domain/update_project.rs`
- Modify: `src/lib/api/types.ts`

**Interfaces:**
- Consumes: `palette::is_known_swatch`, `ProjectError::UnknownSwatch` (Task 1).
- Produces: `Project.group_id: Option<String>`, `Project.color: Option<String>`, `Project.icon: Option<String>`; `UpdateProject.group_id/color/icon: Option<Option<String>>`.

- [ ] **Step 1: Write the failing tests**

In `crates/core/src/domain/project.rs`, inside the existing `mod tests`, add:

```rust
    #[test]
    fn new_project_has_no_group_colour_or_icon() {
        let p = Project::new("Name".into(), std::env::temp_dir().to_string_lossy().into_owned(), None, None)
            .expect("temp dir exists");
        assert!(p.group_id.is_none());
        assert!(p.color.is_none());
        assert!(p.icon.is_none());
    }

    #[test]
    fn update_sets_and_clears_group_colour_and_icon() {
        let mut p = project_with_dir(&std::env::temp_dir().to_string_lossy());

        p.update(UpdateProject {
            name: None,
            directory: None,
            description: None,
            tags: None,
            favorite: None,
            open_with: None,
            notes: None,
            client: None,
            group_id: Some(Some("group-1".into())),
            color: Some(Some("gold".into())),
            icon: Some(Some("gamepad".into())),
        })
        .expect("valid update");
        assert_eq!(p.group_id.as_deref(), Some("group-1"));
        assert_eq!(p.color.as_deref(), Some("gold"));
        assert_eq!(p.icon.as_deref(), Some("gamepad"));

        // An explicit null clears; an absent key leaves it alone.
        p.update(UpdateProject {
            name: None,
            directory: None,
            description: None,
            tags: None,
            favorite: None,
            open_with: None,
            notes: None,
            client: None,
            group_id: Some(None),
            color: None,
            icon: None,
        })
        .expect("valid update");
        assert!(p.group_id.is_none());
        assert_eq!(p.color.as_deref(), Some("gold"));
    }

    #[test]
    fn update_rejects_an_unknown_colour() {
        let mut p = project_with_dir(&std::env::temp_dir().to_string_lossy());
        let result = p.update(UpdateProject {
            name: None,
            directory: None,
            description: None,
            tags: None,
            favorite: None,
            open_with: None,
            notes: None,
            client: None,
            group_id: None,
            color: Some(Some("chartreuse".into())),
            icon: None,
        });
        assert!(matches!(result, Err(ProjectError::UnknownSwatch(_))));
        assert!(p.color.is_none());
    }
```

Extend the existing `loads_a_record_missing_every_absorbable_field` test with three more assertions, after the `trackers` line:

```rust
        assert!(project.group_id.is_none());
        assert!(project.color.is_none());
        assert!(project.icon.is_none());
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p indexer-core project`
Expected: FAIL — `struct UpdateProject has no field named group_id`, and `no field group_id on type Project`.

- [ ] **Step 3: Add the fields to `Project`**

In `crates/core/src/domain/project.rs`, add to the `struct Project` definition, after `trackers`:

```rust
    /// The group this project belongs to, or `None` for Ungrouped.
    /// Mirrored into the `projects.group_id` column for querying.
    #[serde(default)]
    pub group_id: Option<String>,
    /// Secondary palette colour — distinguishes this project from others.
    #[serde(default)]
    pub color: Option<String>,
    /// Bundled icon name (`"gamepad"`) or a custom one (`"custom:my-logo"`).
    #[serde(default)]
    pub icon: Option<String>,
```

Add `group_id: None, color: None, icon: None,` to the struct literal in `Project::new`, and to the `project_with_dir` helper in the test module.

Add the colour check to `Project::update`, immediately after the existing directory validation and before any assignment:

```rust
        if let Some(Some(color)) = &update.color {
            if !crate::domain::palette::is_known_swatch(color) {
                return Err(ProjectError::UnknownSwatch(color.clone()));
            }
        }
```

Extend the `apply_if_present!` invocation in `Project::update` to include the three new fields:

```rust
        apply_if_present!(
            self,
            update,
            description,
            favorite,
            open_with,
            notes,
            client,
            group_id,
            color,
            icon
        );
```

The macro already handles the double-option shape: for `open_with`-style fields the matched value is itself an `Option<String>`, which is exactly the field's type.

- [ ] **Step 4: Add the fields to `UpdateProject`**

In `crates/core/src/domain/update_project.rs`, add after `client`:

```rust
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_some"
    )]
    pub group_id: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_some"
    )]
    pub color: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_some"
    )]
    pub icon: Option<Option<String>>,
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p indexer-core project`
Expected: PASS.

- [ ] **Step 6: Mirror the change into the TypeScript types**

`src/lib/api/types.ts` is a hand-maintained mirror of the Rust models and drifts silently. In `interface Project`, add after `trackers`:

```ts
  group_id: string | null;
  color: string | null;
  icon: string | null;
```

In `interface UpdateProject`, add after `client`:

```ts
  group_id?: string | null;
  color?: string | null;
  icon?: string | null;
```

- [ ] **Step 7: Run the web gate and commit**

Run: `pnpm run check && pnpm test`
Expected: 0 errors; the 8 known `state_referenced_locally` warnings in `EditProjectForm.svelte` (PI-003, a documented false positive) remain and are fine.

```bash
git add crates/core/src/domain/project.rs crates/core/src/domain/update_project.rs src/lib/api/types.ts
git commit -m "feat(core): give a project a group, a colour and an icon

All three are Option<T>, so records written by older builds still load —
the absorbable-field contract at the top of project.rs, with the legacy
record test extended to cover them.

On UpdateProject they are double options, matching open_with/notes/client,
because all three are clearable and 'key absent' must stay distinguishable
from 'key present but null'. Colour is validated against the palette before
anything is assigned, so a rejected update changes nothing.

types.ts is a hand-maintained mirror of the Rust models, so it moves in the
same commit rather than drifting.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 3: Migration fixtures scaffold

This is the deferred `ROADMAP.md` item whose trigger is `CURRENT_SCHEMA_VERSION` going to 2. It is built **before** the migration so the migration lands against a harness that already works.

**Files:**
- Create: `crates/core/tests/migrations.rs`

**Interfaces:**
- Consumes: `SqliteRepository::open`, `CURRENT_SCHEMA_VERSION` (existing).
- Produces: test helpers `seed_v1(path: &Path)` and `user_version(path: &Path) -> i64`, used again in Task 4.

- [ ] **Step 1: Write the fixture harness and its first assertions**

Create `crates/core/tests/migrations.rs`:

```rust
//! Migration fixtures: seed a database at a known `user_version`, run `open`,
//! and assert the result of each step.
//!
//! `ROADMAP.md` gates this scaffold on `CURRENT_SCHEMA_VERSION` reaching 2.
//! The point is that a *new* binary opening an *old* database is routine — so
//! every step gets a fixture asserting the shape it produces and that existing
//! rows survive it.

use std::path::Path;

use indexer_core::infra::{SqliteRepository, CURRENT_SCHEMA_VERSION};
use rusqlite::Connection;

/// The v1 schema, verbatim as the `from < 1` migration step wrote it. Copied
/// rather than referenced on purpose: a fixture must keep describing the old
/// shape even after the production code has moved on.
const V1_SCHEMA: &str = "
    CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
    INSERT INTO meta (key, value) VALUES
      ('app', 'project-indexer'), ('schema_version', '1');
    CREATE TABLE projects (
      id                   TEXT PRIMARY KEY,
      data                 TEXT NOT NULL,
      is_deleted           INTEGER NOT NULL,
      directory_normalized TEXT NOT NULL,
      updated_at           TEXT NOT NULL
    );
    CREATE INDEX idx_projects_is_deleted ON projects(is_deleted);
    CREATE INDEX idx_projects_directory_normalized ON projects(directory_normalized);
    CREATE TABLE project_tags (
      project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
      tag        TEXT NOT NULL,
      PRIMARY KEY (project_id, tag)
    );
    CREATE INDEX idx_project_tags_tag ON project_tags(tag);
";

/// One project as a v1 binary would have written it — no group, colour or icon.
const V1_PROJECT: &str = r#"{
    "is_deleted": false,
    "id": "seeded-1",
    "name": "Seeded",
    "description": "",
    "directory": "/tmp/seeded",
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-01-01T00:00:00Z",
    "last_opened_at": null,
    "tags": ["rust"],
    "favorite": false,
    "open_with": null,
    "notes": null,
    "client": null,
    "trackers": []
}"#;

fn seed_v1(path: &Path) {
    let conn = Connection::open(path).expect("open fixture db");
    conn.execute_batch(V1_SCHEMA).expect("v1 schema");
    conn.execute(
        "INSERT INTO projects (id, data, is_deleted, directory_normalized, updated_at)
         VALUES ('seeded-1', ?1, 0, '/tmp/seeded', '2024-01-01T00:00:00Z')",
        [V1_PROJECT],
    )
    .expect("seed project");
    conn.execute("INSERT INTO project_tags (project_id, tag) VALUES ('seeded-1', 'rust')", [])
        .expect("seed tag");
    conn.pragma_update(None, "user_version", 1)
        .expect("stamp v1");
}

fn user_version(path: &Path) -> i64 {
    Connection::open(path)
        .expect("reopen")
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .expect("read user_version")
}

fn meta_schema_version(path: &Path) -> String {
    Connection::open(path)
        .expect("reopen")
        .query_row("SELECT value FROM meta WHERE key = 'schema_version'", [], |r| r.get(0))
        .expect("read meta schema_version")
}

#[test]
fn a_v1_database_opens_and_keeps_its_rows() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("projects.db");
    seed_v1(&path);

    let repo = SqliteRepository::open(&path).expect("v1 database must open");

    let project = repo
        .get("seeded-1")
        .expect("read")
        .expect("the seeded project must survive migration");
    assert_eq!(project.name, "Seeded");
    assert_eq!(project.tags, vec!["rust".to_string()]);
}

#[test]
fn opening_stamps_the_current_schema_version_everywhere() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("projects.db");
    seed_v1(&path);

    let _repo = SqliteRepository::open(&path).expect("open");

    assert_eq!(user_version(&path), CURRENT_SCHEMA_VERSION);
    assert_eq!(meta_schema_version(&path), CURRENT_SCHEMA_VERSION.to_string());
}

#[test]
fn a_database_from_a_newer_binary_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("projects.db");
    seed_v1(&path);
    Connection::open(&path)
        .expect("open")
        .pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION + 1)
        .expect("stamp a future version");

    assert!(
        SqliteRepository::open(&path).is_err(),
        "a database written by a newer binary must be refused, not migrated"
    );
}

#[test]
fn opening_an_already_current_database_is_a_no_op() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("projects.db");
    seed_v1(&path);

    let _first = SqliteRepository::open(&path).expect("first open migrates");
    drop(_first);
    let repo = SqliteRepository::open(&path).expect("second open must be a no-op");

    assert_eq!(user_version(&path), CURRENT_SCHEMA_VERSION);
    assert!(repo.get("seeded-1").expect("read").is_some());
}
```

- [ ] **Step 2: Run the fixtures against the current v1 code**

Run: `cargo test -p indexer-core --test migrations`
Expected: PASS (4 tests) with `CURRENT_SCHEMA_VERSION` still `1`. The harness is proven before the migration exists — that is the point of doing it first.

- [ ] **Step 3: Commit**

```bash
git add crates/core/tests/migrations.rs
git commit -m "test(core): add the migration fixture scaffold

ROADMAP gates this on CURRENT_SCHEMA_VERSION reaching 2, and it lands
before the migration rather than after so the step arrives against a
harness already known to work.

Seeds a database at a known user_version from a verbatim copy of the v1
schema — copied rather than referenced, because a fixture has to keep
describing the old shape after production code moves on — then asserts
that rows survive, that both the pragma and the meta mirror get stamped,
that a newer database is refused, and that reopening is a no-op.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 4: The v2 migration

**Files:**
- Modify: `crates/core/src/infra/sqlite_repository.rs`
- Modify: `crates/core/tests/migrations.rs`

**Interfaces:**
- Consumes: the fixture helpers from Task 3.
- Produces: `CURRENT_SCHEMA_VERSION == 2`; tables `groups`, column `projects.group_id`.

- [ ] **Step 1: Write the failing tests**

Add to `crates/core/tests/migrations.rs`:

```rust
fn table_exists(path: &Path, name: &str) -> bool {
    Connection::open(path)
        .expect("reopen")
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |r| r.get::<_, i64>(0),
        )
        .expect("query sqlite_master")
        > 0
}

fn column_exists(path: &Path, table: &str, column: &str) -> bool {
    let conn = Connection::open(path).expect("reopen");
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("table_info");
    let mut rows = stmt.query([]).expect("query");
    while let Some(row) = rows.next().expect("row") {
        let name: String = row.get(1).expect("column name");
        if name == column {
            return true;
        }
    }
    false
}

#[test]
fn v2_adds_groups_and_the_project_group_column() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("projects.db");
    seed_v1(&path);

    let _repo = SqliteRepository::open(&path).expect("open");

    assert_eq!(user_version(&path), 2);
    assert!(table_exists(&path, "groups"));
    assert!(column_exists(&path, "projects", "group_id"));
}

#[test]
fn v2_leaves_existing_projects_ungrouped() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("projects.db");
    seed_v1(&path);

    let repo = SqliteRepository::open(&path).expect("open");

    let project = repo.get("seeded-1").expect("read").expect("still there");
    assert!(
        project.group_id.is_none(),
        "a migrated project must land in Ungrouped, not in a group that does not exist"
    );

    let column: Option<String> = Connection::open(&path)
        .expect("reopen")
        .query_row("SELECT group_id FROM projects WHERE id = 'seeded-1'", [], |r| r.get(0))
        .expect("read column");
    assert!(column.is_none(), "the mirrored column must agree with the blob");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p indexer-core --test migrations`
Expected: FAIL — `assertion failed: left == 1, right == 2`, and `table_exists(groups)` is false.

- [ ] **Step 3: Implement the migration**

In `crates/core/src/infra/sqlite_repository.rs`, change the constant:

```rust
pub const CURRENT_SCHEMA_VERSION: i64 = 2;
```

Then, in `run_migrations`, add this block after the closing brace of the `if from < 1 { … }` block and before the `meta` mirror update:

```rust
    if from < 2 {
        // Groups, and the project's membership in one.
        //
        // `group_id` is added to `projects` as a mirror of the same key inside
        // the JSON blob — the house pattern, as `directory_normalized` already
        // is. `ON DELETE SET NULL` is a safety net only: `delete_group` clears
        // blob and column together in one transaction, because the constraint
        // alone would fix the column and leave the blob stale.
        //
        // SQLite permits a REFERENCES clause on ADD COLUMN as long as the
        // default is NULL, which it is.
        conn.execute_batch(
            "BEGIN;
             CREATE TABLE groups (
               id         TEXT PRIMARY KEY,
               name       TEXT NOT NULL,
               color      TEXT NOT NULL,
               icon       TEXT NOT NULL,
               position   INTEGER NOT NULL,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL
             );
             CREATE UNIQUE INDEX idx_groups_name_nocase ON groups(name COLLATE NOCASE);
             CREATE INDEX idx_groups_position ON groups(position);
             ALTER TABLE projects ADD COLUMN group_id TEXT
               REFERENCES groups(id) ON DELETE SET NULL;
             CREATE INDEX idx_projects_group_id ON projects(group_id);
             COMMIT;",
        )
        .map_err(be)?;
        conn.pragma_update(None, "user_version", 2).map_err(be)?;
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p indexer-core --test migrations`
Expected: PASS (6 tests).

- [ ] **Step 5: Run the full gate and commit**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets && cargo test --workspace`
Expected: all pass; clippy at the one-warning baseline.

```bash
git add crates/core/src/infra/sqlite_repository.rs crates/core/tests/migrations.rs
git commit -m "feat(core): migrate the schema to v2 — groups

The project's first schema migration. A groups table with a
case-insensitive unique name, and a group_id column on projects that
mirrors the same key inside the JSON blob, exactly as
directory_normalized already does.

ON DELETE SET NULL is a safety net rather than the mechanism: it would
fix the column and leave the blob stale, so delete_group clears both in
one transaction instead. Existing projects migrate to Ungrouped, asserted
in both the blob and the column.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 5: `GroupRepository` port and its SQLite implementation

**Files:**
- Modify: `crates/core/src/ports/repository.rs`
- Modify: `crates/core/src/ports/mod.rs`
- Modify: `crates/core/src/infra/sqlite_repository.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `Group` (Task 1), the v2 schema (Task 4).
- Produces: `GroupReader::{get_group, list_groups}`, `GroupRepository::{save_group, delete_group, set_group_positions}`, all implemented on `SqliteRepository`; `ProjectRepository::save` now also writes the `group_id` column.

- [ ] **Step 1: Write the failing tests**

In `crates/core/src/infra/sqlite_repository.rs`, inside `mod tests`, add:

```rust
    use crate::domain::Group;
    use crate::ports::{GroupReader, GroupRepository};

    fn group(name: &str, position: i64) -> Group {
        Group::new(name.into(), "cyan".into(), "briefcase".into(), position).expect("valid group")
    }

    #[test]
    fn round_trips_a_group() {
        let repo = SqliteRepository::in_memory().unwrap();
        let g = group("Client work", 0);
        repo.save_group(&g).unwrap();
        let got = repo.get_group(&g.id).unwrap().unwrap();
        assert_eq!(got.name, "Client work");
        assert_eq!(got.color, "cyan");
        assert_eq!(got.icon, "briefcase");
    }

    #[test]
    fn lists_groups_in_position_order() {
        let repo = SqliteRepository::in_memory().unwrap();
        repo.save_group(&group("Third", 2)).unwrap();
        repo.save_group(&group("First", 0)).unwrap();
        repo.save_group(&group("Second", 1)).unwrap();

        let names: Vec<String> = repo.list_groups().unwrap().into_iter().map(|g| g.name).collect();
        assert_eq!(names, vec!["First", "Second", "Third"]);
    }

    #[test]
    fn save_writes_group_id_to_both_the_blob_and_the_column() {
        let repo = SqliteRepository::in_memory().unwrap();
        let g = group("Client work", 0);
        repo.save_group(&g).unwrap();

        let mut p = sample("id-1", &tmp());
        p.group_id = Some(g.id.clone());
        repo.save(&p).unwrap();

        assert_eq!(repo.get("id-1").unwrap().unwrap().group_id, Some(g.id.clone()));
        let column: Option<String> = repo
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT group_id FROM projects WHERE id = 'id-1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(column, Some(g.id));
    }

    #[test]
    fn deleting_a_group_ungroups_its_projects_in_blob_and_column() {
        let repo = SqliteRepository::in_memory().unwrap();
        let g = group("Client work", 0);
        repo.save_group(&g).unwrap();
        let mut p = sample("id-1", &tmp());
        p.group_id = Some(g.id.clone());
        repo.save(&p).unwrap();

        repo.delete_group(&g.id).unwrap();

        // The project survives — deleting a group never deletes a project.
        let got = repo.get("id-1").unwrap().expect("project must survive");
        assert!(got.group_id.is_none(), "the blob must be cleared, not just the column");
        let column: Option<String> = repo
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT group_id FROM projects WHERE id = 'id-1'", [], |r| r.get(0))
            .unwrap();
        assert!(column.is_none());
        assert!(repo.get_group(&g.id).unwrap().is_none());
    }

    #[test]
    fn deleting_a_missing_group_is_idempotent() {
        let repo = SqliteRepository::in_memory().unwrap();
        assert!(repo.delete_group("nope").is_ok());
    }

    #[test]
    fn set_group_positions_rewrites_the_whole_ordering() {
        let repo = SqliteRepository::in_memory().unwrap();
        let a = group("A", 0);
        let b = group("B", 1);
        let c = group("C", 2);
        for g in [&a, &b, &c] {
            repo.save_group(g).unwrap();
        }

        repo.set_group_positions(&[c.id.clone(), a.id.clone(), b.id.clone()])
            .unwrap();

        let names: Vec<String> = repo.list_groups().unwrap().into_iter().map(|g| g.name).collect();
        assert_eq!(names, vec!["C", "A", "B"]);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p indexer-core sqlite`
Expected: FAIL — `no method named save_group`.

- [ ] **Step 3: Define the ports**

Append to `crates/core/src/ports/repository.rs`:

```rust
use crate::domain::Group;

/// Read access to stored groups. Split from [`GroupRepository`] for the same
/// reason [`ProjectReader`] is split from [`ProjectRepository`]: an external
/// consumer can depend on reads without the write surface.
pub trait GroupReader: Send + Sync {
    fn get_group(&self, id: &str) -> Result<Option<Group>, RepositoryError>;
    /// Every group, ordered by `position` ascending.
    fn list_groups(&self) -> Result<Vec<Group>, RepositoryError>;
}

pub trait GroupRepository: GroupReader {
    /// Insert or replace by `group.id`.
    fn save_group(&self, group: &Group) -> Result<(), RepositoryError>;

    /// Deletes the group **and** clears membership from every project that
    /// belonged to it, in one transaction. Idempotent — a missing id is `Ok(())`.
    ///
    /// The blob and the column are cleared together deliberately: the
    /// `ON DELETE SET NULL` constraint on the column would leave a stale
    /// `group_id` inside the JSON, which is the authoritative copy.
    fn delete_group(&self, id: &str) -> Result<(), RepositoryError>;

    /// Rewrites `position` so it matches the given order, in one transaction.
    /// The single path that renumbers; ids not listed are left untouched.
    fn set_group_positions(&self, ordered_ids: &[String]) -> Result<(), RepositoryError>;
}
```

In `crates/core/src/ports/mod.rs`, extend the re-export:

```rust
pub use repository::{GroupReader, GroupRepository, ProjectReader, ProjectRepository};
```

In `crates/core/src/lib.rs`, extend its ports re-export the same way:

```rust
pub use ports::{AppLauncher, GroupReader, GroupRepository, ProjectReader, ProjectRepository};
```

- [ ] **Step 4: Write the `group_id` column into `save`**

In `crates/core/src/infra/sqlite_repository.rs`, in `ProjectRepository::save`, replace the `INSERT INTO projects …` statement with the version that carries the new column:

```rust
        tx.execute(
            "INSERT INTO projects (id, data, is_deleted, directory_normalized, updated_at, group_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
               data = excluded.data,
               is_deleted = excluded.is_deleted,
               directory_normalized = excluded.directory_normalized,
               updated_at = excluded.updated_at,
               group_id = excluded.group_id",
            rusqlite::params![
                project.id,
                data,
                project.is_deleted as i64,
                dir_norm,
                project.updated_at.to_rfc3339(),
                project.group_id,
            ],
        )
        .map_err(be)?;
```

- [ ] **Step 5: Implement the group repository**

Add to `crates/core/src/infra/sqlite_repository.rs`, after the `impl ProjectRepository for SqliteRepository` block:

```rust
impl GroupReader for SqliteRepository {
    fn get_group(&self, id: &str) -> Result<Option<Group>, RepositoryError> {
        let conn = self.lock_conn();
        conn.query_row(
            "SELECT id, name, color, icon, position, created_at, updated_at
             FROM groups WHERE id = ?1",
            [id],
            group_from_row,
        )
        .optional()
        .map_err(be)?
        .transpose()
    }

    fn list_groups(&self) -> Result<Vec<Group>, RepositoryError> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, name, color, icon, position, created_at, updated_at
                 FROM groups ORDER BY position ASC",
            )
            .map_err(be)?;
        let rows = stmt.query_map([], group_from_row).map_err(be)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(be)??);
        }
        Ok(out)
    }
}

impl GroupRepository for SqliteRepository {
    fn save_group(&self, group: &Group) -> Result<(), RepositoryError> {
        let conn = self.lock_conn();
        conn.execute(
            "INSERT INTO groups (id, name, color, icon, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name,
               color = excluded.color,
               icon = excluded.icon,
               position = excluded.position,
               updated_at = excluded.updated_at",
            rusqlite::params![
                group.id,
                group.name,
                group.color,
                group.icon,
                group.position,
                group.created_at.to_rfc3339(),
                group.updated_at.to_rfc3339(),
            ],
        )
        .map_err(be)?;
        Ok(())
    }

    fn delete_group(&self, id: &str) -> Result<(), RepositoryError> {
        let mut conn = self.lock_conn();
        let tx = conn.transaction().map_err(be)?;

        // Read the members first: the blob is the authoritative copy of
        // `group_id`, so clearing the column alone would leave it stale.
        let members: Vec<(String, String)> = {
            let mut stmt = tx
                .prepare("SELECT id, data FROM projects WHERE group_id = ?1")
                .map_err(be)?;
            let rows = stmt
                .query_map([id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                .map_err(be)?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(be)?);
            }
            out
        };

        for (project_id, data) in members {
            let mut project = parse(&data)?;
            project.group_id = None;
            let updated = serde_json::to_string(&project)
                .map_err(|e| RepositoryError::Backend(format!("serialize: {e}")))?;
            tx.execute(
                "UPDATE projects SET data = ?2, group_id = NULL WHERE id = ?1",
                rusqlite::params![project_id, updated],
            )
            .map_err(be)?;
        }

        tx.execute("DELETE FROM groups WHERE id = ?1", [id])
            .map_err(be)?;
        tx.commit().map_err(be)?;
        Ok(())
    }

    fn set_group_positions(&self, ordered_ids: &[String]) -> Result<(), RepositoryError> {
        let mut conn = self.lock_conn();
        let tx = conn.transaction().map_err(be)?;
        for (position, id) in ordered_ids.iter().enumerate() {
            tx.execute(
                "UPDATE groups SET position = ?2 WHERE id = ?1",
                rusqlite::params![id, position as i64],
            )
            .map_err(be)?;
        }
        tx.commit().map_err(be)?;
        Ok(())
    }
}

/// Row mapper shared by `get_group` and `list_groups`. The outer `Result` is
/// rusqlite's; the inner one carries a timestamp that failed to parse, which is
/// corruption rather than a backend fault.
fn group_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Result<Group, RepositoryError>> {
    let created_at: String = row.get(5)?;
    let updated_at: String = row.get(6)?;
    Ok((|| {
        Ok(Group {
            id: row_string(row, 0)?,
            name: row_string(row, 1)?,
            color: row_string(row, 2)?,
            icon: row_string(row, 3)?,
            position: row.get(4).map_err(|e| RepositoryError::Corrupt(e.to_string()))?,
            created_at: parse_timestamp(&created_at)?,
            updated_at: parse_timestamp(&updated_at)?,
        })
    })())
}

fn row_string(row: &rusqlite::Row<'_>, idx: usize) -> Result<String, RepositoryError> {
    row.get(idx)
        .map_err(|e| RepositoryError::Corrupt(e.to_string()))
}

fn parse_timestamp(raw: &str) -> Result<chrono::DateTime<chrono::Utc>, RepositoryError> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| RepositoryError::Corrupt(format!("timestamp {raw:?}: {e}")))
}
```

Add the imports at the top of the file:

```rust
use crate::domain::{Group, Project};
use crate::ports::{GroupReader, GroupRepository, ProjectReader, ProjectRepository};
```

(replacing the existing `use crate::domain::Project;` and `use crate::ports::{ProjectReader, ProjectRepository};` lines).

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p indexer-core`
Expected: PASS, including the six new tests.

- [ ] **Step 7: Run the full gate and commit**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets && cargo test --workspace`

```bash
git add crates/core/src/ports/repository.rs crates/core/src/ports/mod.rs \
        crates/core/src/infra/sqlite_repository.rs crates/core/src/lib.rs
git commit -m "feat(core): add the GroupRepository port and its SQLite impl

Reader and writer are split the way ProjectReader and ProjectRepository
are, so a read-only consumer never gets the write surface.

delete_group is the interesting one: it reads its members, clears
group_id inside each JSON blob, writes blob and column back, and deletes
the group — all in one transaction. The FK constraint would only fix the
column, and the blob is the authoritative copy. Deleting a group never
deletes a project. set_group_positions is the single path that renumbers.

save now mirrors group_id into its column alongside the blob, matching
how directory_normalized already works.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 6: `GroupService`

**Files:**
- Create: `crates/core/src/application/group_service.rs`
- Modify: `crates/core/src/application/mod.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `GroupRepository` (Task 5), `Group`/`UpdateGroup` (Task 1).
- Produces: `GroupService::new(repo: Arc<dyn GroupRepository>) -> GroupService`; `list() -> Result<Vec<Group>, ProjectError>`; `create(name: String, color: String, icon: String) -> Result<Group, ProjectError>`; `update(id: &str, update: UpdateGroup) -> Result<Group, ProjectError>`; `delete(id: &str) -> Result<(), ProjectError>`; `reorder(ordered_ids: Vec<String>) -> Result<Vec<Group>, ProjectError>`.

- [ ] **Step 1: Write the failing tests**

Create `crates/core/src/application/group_service.rs` with only this test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::SqliteRepository;

    fn service() -> GroupService {
        GroupService::new(Arc::new(SqliteRepository::in_memory().expect("in-memory db")))
    }

    #[test]
    fn create_appends_after_the_last_group() {
        let svc = service();
        svc.create("First".into(), "cyan".into(), "briefcase".into()).unwrap();
        let second = svc.create("Second".into(), "gold".into(), "star".into()).unwrap();
        assert_eq!(second.position, 1);
    }

    #[test]
    fn create_rejects_a_duplicate_name_ignoring_case() {
        let svc = service();
        svc.create("Client work".into(), "cyan".into(), "briefcase".into()).unwrap();
        let result = svc.create("CLIENT WORK".into(), "gold".into(), "star".into());
        assert!(matches!(result, Err(ProjectError::DuplicateGroupName(_))));
    }

    #[test]
    fn update_renames_a_group() {
        let svc = service();
        let g = svc.create("Old".into(), "cyan".into(), "briefcase".into()).unwrap();
        let updated = svc
            .update(&g.id, UpdateGroup { name: Some("New".into()), color: None, icon: None })
            .unwrap();
        assert_eq!(updated.name, "New");
    }

    #[test]
    fn update_rejects_a_name_another_group_already_has() {
        let svc = service();
        svc.create("Taken".into(), "cyan".into(), "briefcase".into()).unwrap();
        let g = svc.create("Mine".into(), "gold".into(), "star".into()).unwrap();
        let result = svc.update(&g.id, UpdateGroup { name: Some("taken".into()), color: None, icon: None });
        assert!(matches!(result, Err(ProjectError::DuplicateGroupName(_))));
    }

    #[test]
    fn update_allows_a_group_to_keep_its_own_name() {
        let svc = service();
        let g = svc.create("Mine".into(), "cyan".into(), "briefcase".into()).unwrap();
        let updated = svc
            .update(&g.id, UpdateGroup { name: Some("Mine".into()), color: Some("gold".into()), icon: None })
            .unwrap();
        assert_eq!(updated.color, "gold");
    }

    #[test]
    fn update_of_a_missing_group_is_not_found() {
        let svc = service();
        let result = svc.update("nope", UpdateGroup { name: Some("x".into()), color: None, icon: None });
        assert!(matches!(result, Err(ProjectError::GroupNotFound(_))));
    }

    #[test]
    fn reorder_returns_the_new_order() {
        let svc = service();
        let a = svc.create("A".into(), "cyan".into(), "briefcase".into()).unwrap();
        let b = svc.create("B".into(), "gold".into(), "star".into()).unwrap();
        let names: Vec<String> = svc
            .reorder(vec![b.id.clone(), a.id.clone()])
            .unwrap()
            .into_iter()
            .map(|g| g.name)
            .collect();
        assert_eq!(names, vec!["B", "A"]);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p indexer-core group_service`
Expected: FAIL — `cannot find struct GroupService`.

- [ ] **Step 3: Implement the service**

At the top of `crates/core/src/application/group_service.rs`:

```rust
use std::sync::Arc;

use crate::domain::{Group, UpdateGroup};
use crate::error::ProjectError;
use crate::ports::GroupRepository;

/// Orchestration for groups: uniqueness, positioning, and the read-back that
/// gives callers the persisted value rather than the one they sent.
///
/// Assigning a *project* to a group is not here — that is an ordinary
/// `UpdateProject.group_id` and belongs to `ProjectService`.
pub struct GroupService {
    repo: Arc<dyn GroupRepository>,
}

/// Opaque by necessity — the field is a `dyn` port with no `Debug` bound.
/// Mirrors `ProjectService`, so types holding one can still derive `Debug`.
impl std::fmt::Debug for GroupService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupService").finish_non_exhaustive()
    }
}

impl GroupService {
    pub fn new(repo: Arc<dyn GroupRepository>) -> Self {
        Self { repo }
    }

    pub fn list(&self) -> Result<Vec<Group>, ProjectError> {
        Ok(self.repo.list_groups()?)
    }

    /// Appends after the last group. New groups never displace existing ones —
    /// `reorder` is the only thing that renumbers.
    pub fn create(
        &self,
        name: String,
        color: String,
        icon: String,
    ) -> Result<Group, ProjectError> {
        let existing = self.repo.list_groups()?;
        Group::check_for_duplicate_name(&name, &existing)?;

        let position = existing.iter().map(|g| g.position).max().unwrap_or(-1) + 1;
        let group = Group::new(name, color, icon, position)?;
        self.repo.save_group(&group)?;
        Ok(group)
    }

    pub fn update(&self, id: &str, update: UpdateGroup) -> Result<Group, ProjectError> {
        let mut group = self
            .repo
            .get_group(id)?
            .ok_or_else(|| ProjectError::GroupNotFound(id.to_string()))?;

        // A group keeping its own name is not a duplicate, so exclude itself
        // from the check rather than comparing against every group.
        if let Some(name) = &update.name {
            let others: Vec<Group> = self
                .repo
                .list_groups()?
                .into_iter()
                .filter(|g| g.id != group.id)
                .collect();
            Group::check_for_duplicate_name(name, &others)?;
        }

        group.update(update)?;
        self.repo.save_group(&group)?;
        Ok(group)
    }

    /// Members become Ungrouped; no project is deleted.
    pub fn delete(&self, id: &str) -> Result<(), ProjectError> {
        self.repo.delete_group(id)?;
        Ok(())
    }

    pub fn reorder(&self, ordered_ids: Vec<String>) -> Result<Vec<Group>, ProjectError> {
        self.repo.set_group_positions(&ordered_ids)?;
        self.list()
    }
}
```

In `crates/core/src/application/mod.rs`, add `pub mod group_service;` and `pub use group_service::GroupService;`.

In `crates/core/src/lib.rs`, extend the application re-export:

```rust
pub use application::{GroupService, ProjectInspection, ProjectService};
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p indexer-core group_service`
Expected: PASS (7 tests).

- [ ] **Step 5: Run the full gate and commit**

```bash
git add crates/core/src/application/group_service.rs crates/core/src/application/mod.rs crates/core/src/lib.rs
git commit -m "feat(core): add GroupService

Create appends after the last group, so a new group never displaces an
existing one and reorder stays the only thing that renumbers. Rename
excludes the group itself from the duplicate check — keeping your own
name is not a collision.

Assigning a project to a group is deliberately absent: that is an
ordinary UpdateProject.group_id and belongs to ProjectService.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 7: Group commands and Tauri wiring

**Files:**
- Create: `src-tauri/src/commands/groups.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `GroupService` (Task 6).
- Produces: commands `list_groups`, `create_group`, `update_group`, `delete_group`, `reorder_groups`, all taking `State<'_, Arc<GroupService>>`.

- [ ] **Step 1: Write the commands**

Create `src-tauri/src/commands/groups.rs`:

```rust
use std::sync::Arc;

use tauri::State;

use indexer_core::application::GroupService;
use indexer_core::domain::{Group, UpdateGroup};
use indexer_core::error::ProjectError;

#[tauri::command]
pub fn list_groups(service: State<'_, Arc<GroupService>>) -> Result<Vec<Group>, ProjectError> {
    service.list()
}

#[tauri::command]
pub fn create_group(
    service: State<'_, Arc<GroupService>>,
    name: String,
    color: String,
    icon: String,
) -> Result<Group, ProjectError> {
    service.create(name, color, icon)
}

#[tauri::command]
pub fn update_group(
    service: State<'_, Arc<GroupService>>,
    id: String,
    update: UpdateGroup,
) -> Result<Group, ProjectError> {
    service.update(&id, update)
}

/// Members become Ungrouped. No project is deleted by this.
#[tauri::command]
pub fn delete_group(service: State<'_, Arc<GroupService>>, id: String) -> Result<(), ProjectError> {
    service.delete(&id)
}

/// Rewrites the whole sidebar ordering and returns it, so the caller renders
/// what was persisted rather than what it hoped for.
#[tauri::command]
pub fn reorder_groups(
    service: State<'_, Arc<GroupService>>,
    ordered_ids: Vec<String>,
) -> Result<Vec<Group>, ProjectError> {
    service.reorder(ordered_ids)
}
```

In `src-tauri/src/commands/mod.rs`, add `pub mod groups;`.

- [ ] **Step 2: Share one repository between both services**

In `src-tauri/src/lib.rs`, the repository is currently moved into `ProjectService`. Both services need it, so wrap it once. Replace the body of the `.setup(|app| { … })` closure's service construction:

```rust
            let repo = match open_repository(app) {
                Ok(repo) => repo,
                Err(e) => {
                    fatal_startup_error(&format!("Project Indexer can't start:\n\n{e}"));
                }
            };
            // One repository, two services. `Arc<SqliteRepository>` coerces to
            // each port, so both views share a single connection and its lock.
            let repo = Arc::new(repo);
            let service = ProjectService::new(
                repo.clone(),
                Arc::new(OpenerLauncher),
                Arc::new(DetectorRunner::default()),
            );
            app.manage(Arc::new(service));
            app.manage(Arc::new(GroupService::new(repo)));
```

Add the import alongside the existing core imports near the top of the file:

```rust
use indexer_core::application::{GroupService, ProjectService};
```

(replacing the existing `use indexer_core::application::ProjectService;` line).

- [ ] **Step 3: Register the commands**

In `src-tauri/src/lib.rs`, add to the `tauri::generate_handler![…]` list, after `commands::inspect::inspect_project`:

```rust
            commands::groups::list_groups,
            commands::groups::create_group,
            commands::groups::update_group,
            commands::groups::delete_group,
            commands::groups::reorder_groups
```

Remember the entry before it needs a trailing comma once it is no longer last.

- [ ] **Step 4: Verify it compiles and the app still starts**

Run: `cargo build --workspace`
Expected: clean build.

Then **launch the real app**: `pnpm tauri dev`

Expected: the window appears and the project list loads as before. This step is not optional — neither CI nor the pre-commit hook launches the app, and `PI-005` compiled, passed every test, and still exited before showing a window. This task changes startup wiring, which is exactly that risk.

- [ ] **Step 5: Run the full gate and commit**

```bash
git add src-tauri/src/commands/groups.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(app): expose the group commands

Five thin commands over GroupService, matching the existing command style.

The repository is now wrapped in an Arc once and shared by both services
rather than moved into ProjectService: Arc<SqliteRepository> coerces to
each port, so the two views share one connection and one lock instead of
opening the database twice.

Verified by launching the app, not just by building it — this touches
startup, and PI-005 is the standing reminder that a clean build can still
fail to show a window.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 8: The SVG sanitizer

The security-critical unit in this feature. A custom icon is a user-supplied SVG, and an SVG is a document that can carry `<script>`, `on*` handlers and external references — inlining an untrusted one in the webview would be XSS with full `invoke` reach.

**Files:**
- Modify: `crates/core/Cargo.toml`
- Create: `crates/core/src/error/icon.rs`
- Modify: `crates/core/src/error/mod.rs`
- Modify: `crates/core/src/error/project_error.rs`
- Create: `crates/core/src/icons/mod.rs`
- Create: `crates/core/src/icons/sanitize.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `icons::sanitize_svg(input: &str) -> Result<String, IconError>`; `icons::MAX_SVG_BYTES: usize`; `IconError::{TooLarge, Malformed, NotAnSvg, NothingDrawable, Io}`; `From<IconError> for ProjectError`.

- [ ] **Step 1: Add the dependency**

In `crates/core/Cargo.toml`, add to `[dependencies]`:

```toml
quick-xml = "0.37"
```

An XML parser is not something to hand-roll at a trust boundary. If the resolved version's API differs from the code below (`Reader::read_event`, `BytesStart::push_attribute`), adapt the calls — the allow-list logic is what matters, not the exact call shape.

Run: `cargo build -p indexer-core`
Expected: clean build with the new dependency resolved.

- [ ] **Step 2: Write the failing tests**

Create `crates/core/src/icons/sanitize.rs` with only this test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
        <path d="M3 6h18" stroke="currentColor" stroke-width="2"/>
        <circle cx="12" cy="12" r="4"/>
    </svg>"#;

    #[test]
    fn keeps_a_well_formed_icon() {
        let out = sanitize_svg(GOOD).expect("a clean icon must survive");
        assert!(out.contains("<svg"));
        assert!(out.contains("viewBox"));
        assert!(out.contains("M3 6h18"));
        assert!(out.contains("<circle"));
    }

    #[test]
    fn strips_a_script_element_and_its_contents() {
        let input = r#"<svg viewBox="0 0 1 1"><script>fetch('/x')</script><path d="M0 0"/></svg>"#;
        let out = sanitize_svg(input).expect("still has a path");
        assert!(!out.contains("script"));
        assert!(!out.contains("fetch"));
        assert!(out.contains("M0 0"));
    }

    #[test]
    fn strips_event_handlers() {
        let input = r#"<svg viewBox="0 0 1 1" onload="alert(1)"><path d="M0 0" onclick="x()"/></svg>"#;
        let out = sanitize_svg(input).expect("path survives");
        assert!(!out.contains("onload"));
        assert!(!out.contains("onclick"));
        assert!(!out.contains("alert"));
    }

    #[test]
    fn strips_hrefs_including_the_xlink_form() {
        let input = r#"<svg viewBox="0 0 1 1"><path d="M0 0" href="javascript:alert(1)" xlink:href="http://evil/x.svg"/></svg>"#;
        let out = sanitize_svg(input).expect("path survives");
        assert!(!out.contains("href"));
        assert!(!out.contains("evil"));
        assert!(!out.contains("javascript"));
    }

    #[test]
    fn strips_foreign_object_use_and_image() {
        // `r##"…"##`, not `r#"…"#`: the `"#` inside `href="#x"` would close a
        // single-hash raw string early and this would not compile.
        let input = r##"<svg viewBox="0 0 1 1">
            <foreignObject><div>hi</div></foreignObject>
            <use href="#x"/>
            <image href="http://evil/x.png"/>
            <path d="M0 0"/>
        </svg>"##;
        let out = sanitize_svg(input).expect("path survives");
        assert!(!out.contains("foreignObject"));
        assert!(!out.contains("<use"));
        assert!(!out.contains("<image"));
        assert!(!out.contains("evil"));
        assert!(out.contains("M0 0"));
    }

    #[test]
    fn strips_style_elements() {
        let input = r#"<svg viewBox="0 0 1 1"><style>@import url(http://evil/x.css);</style><path d="M0 0"/></svg>"#;
        let out = sanitize_svg(input).expect("path survives");
        assert!(!out.contains("style"));
        assert!(!out.contains("evil"));
    }

    #[test]
    fn drops_attribute_values_carrying_url_or_javascript() {
        let input = r#"<svg viewBox="0 0 1 1"><path d="M0 0" fill="url(#evil)" stroke="JavaScript:alert(1)"/></svg>"#;
        let out = sanitize_svg(input).expect("path survives");
        assert!(!out.contains("url("));
        assert!(!out.to_lowercase().contains("javascript"));
        assert!(out.contains("M0 0"));
    }

    #[test]
    fn drops_cdata_payloads() {
        let input = r#"<svg viewBox="0 0 1 1"><path d="M0 0"/><![CDATA[<script>alert(1)</script>]]></svg>"#;
        let out = sanitize_svg(input).expect("path survives");
        assert!(!out.contains("alert"));
        assert!(!out.contains("script"));
    }

    #[test]
    fn rejects_input_over_the_size_cap() {
        let huge = format!(
            r#"<svg viewBox="0 0 1 1"><path d="{}"/></svg>"#,
            "M0 0 ".repeat(MAX_SVG_BYTES / 4)
        );
        assert!(matches!(sanitize_svg(&huge), Err(IconError::TooLarge { .. })));
    }

    #[test]
    fn rejects_something_that_is_not_an_svg() {
        let input = r#"<html><body>nope</body></html>"#;
        assert!(matches!(sanitize_svg(input), Err(IconError::NotAnSvg)));
    }

    #[test]
    fn rejects_an_svg_with_nothing_drawable_left() {
        let input = r#"<svg viewBox="0 0 1 1"><script>alert(1)</script></svg>"#;
        assert!(matches!(sanitize_svg(input), Err(IconError::NothingDrawable)));
    }

    #[test]
    fn rejects_malformed_xml() {
        let input = r#"<svg viewBox="0 0 1 1"><path d="M0 0"></svg>"#;
        assert!(sanitize_svg(input).is_err());
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p indexer-core sanitize`
Expected: FAIL — `cannot find function sanitize_svg`.

- [ ] **Step 4: Add the error type**

Create `crates/core/src/error/icon.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IconError {
    #[error("icon is too large: {size} bytes (maximum {max})")]
    TooLarge { size: usize, max: usize },

    #[error("icon is not valid XML: {0}")]
    Malformed(String),

    #[error("icon has no <svg> root")]
    NotAnSvg,

    #[error("icon has nothing drawable left after sanitizing")]
    NothingDrawable,

    #[error("icon store error: {0}")]
    Io(String),
}
```

In `crates/core/src/error/mod.rs`, add `pub mod icon;` and `pub use icon::IconError;`.

In `crates/core/src/error/project_error.rs`, add the variant and the conversion:

```rust
    #[error("Icon problem: {0}")]
    Icon(String),
```

```rust
impl From<crate::error::IconError> for ProjectError {
    fn from(e: crate::error::IconError) -> Self {
        ProjectError::Icon(e.to_string())
    }
}
```

- [ ] **Step 5: Implement the sanitizer**

At the top of `crates/core/src/icons/sanitize.rs`, above the test module:

```rust
use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::{Reader, Writer};

use crate::error::IconError;

/// Largest icon accepted, before sanitizing. A glyph is a few kilobytes; this
/// is generous enough for a detailed logo and small enough that a hostile file
/// cannot be used to exhaust memory.
pub const MAX_SVG_BYTES: usize = 256 * 1024;

/// Elements that survive. An allow-list, never a deny-list: a deny-list is
/// wrong the moment SVG gains an element nobody here has heard of.
const ALLOWED_ELEMENTS: &[&str] = &[
    "svg", "g", "path", "circle", "ellipse", "rect", "line", "polyline", "polygon", "title",
    "desc",
];

/// Attributes that survive. Note what is absent and therefore dropped by
/// construction: every `on*` handler, `href`, `xlink:href`, `style`, `class`,
/// and anything namespaced.
const ALLOWED_ATTRS: &[&str] = &[
    "viewBox", "d", "cx", "cy", "r", "rx", "ry", "x", "y", "x1", "y1", "x2", "y2", "width",
    "height", "points", "transform", "fill", "stroke", "stroke-width", "stroke-linecap",
    "stroke-linejoin", "fill-rule", "clip-rule", "opacity",
];

/// Elements that actually draw something. An icon that keeps none of these
/// after sanitizing is empty, and an empty icon is a silent failure.
const DRAWABLE: &[&str] = &["path", "circle", "ellipse", "rect", "line", "polyline", "polygon"];

/// Rewrites an untrusted SVG as one containing only allow-listed elements and
/// attributes.
///
/// This is the first of two layers. The second is the render path: a sanitized
/// icon is displayed through `<img src="data:…">`, which cannot execute script
/// or load external resources whatever this function misses. Never inline the
/// result — that would collapse both layers into this one.
pub fn sanitize_svg(input: &str) -> Result<String, IconError> {
    if input.len() > MAX_SVG_BYTES {
        return Err(IconError::TooLarge {
            size: input.len(),
            max: MAX_SVG_BYTES,
        });
    }

    let mut reader = Reader::from_str(input);
    let mut writer = Writer::new(Vec::new());

    // Depth of the disallowed subtree currently being skipped. Skipping the
    // whole subtree, rather than just the element, is what keeps a
    // `<script>`'s text content out of the output.
    let mut skip_depth = 0usize;
    let mut saw_svg = false;
    let mut saw_drawable = false;

    loop {
        let event = reader
            .read_event()
            .map_err(|e| IconError::Malformed(e.to_string()))?;

        match event {
            Event::Eof => break,

            Event::Start(e) => {
                let name = local_name(e.name().as_ref());
                if skip_depth > 0 {
                    skip_depth += 1;
                    continue;
                }
                if !ALLOWED_ELEMENTS.contains(&name.as_str()) {
                    skip_depth = 1;
                    continue;
                }
                if name == "svg" {
                    saw_svg = true;
                }
                if DRAWABLE.contains(&name.as_str()) {
                    saw_drawable = true;
                }
                writer
                    .write_event(Event::Start(filter_attributes(&name, &e)?))
                    .map_err(|e| IconError::Malformed(e.to_string()))?;
            }

            Event::Empty(e) => {
                if skip_depth > 0 {
                    continue;
                }
                let name = local_name(e.name().as_ref());
                if !ALLOWED_ELEMENTS.contains(&name.as_str()) {
                    continue;
                }
                if DRAWABLE.contains(&name.as_str()) {
                    saw_drawable = true;
                }
                writer
                    .write_event(Event::Empty(filter_attributes(&name, &e)?))
                    .map_err(|e| IconError::Malformed(e.to_string()))?;
            }

            Event::End(e) => {
                if skip_depth > 0 {
                    skip_depth -= 1;
                    continue;
                }
                let name = local_name(e.name().as_ref());
                if !ALLOWED_ELEMENTS.contains(&name.as_str()) {
                    continue;
                }
                writer
                    .write_event(Event::End(BytesEnd::new(name)))
                    .map_err(|e| IconError::Malformed(e.to_string()))?;
            }

            Event::Text(t) => {
                if skip_depth == 0 {
                    writer
                        .write_event(Event::Text(t))
                        .map_err(|e| IconError::Malformed(e.to_string()))?;
                }
            }

            // Comments, processing instructions, doctypes and CDATA are dropped
            // outright. None of them carry anything an icon needs, and CDATA is
            // a classic way to smuggle a script payload past a naive filter.
            _ => {}
        }
    }

    if !saw_svg {
        return Err(IconError::NotAnSvg);
    }
    if !saw_drawable {
        return Err(IconError::NothingDrawable);
    }

    String::from_utf8(writer.into_inner()).map_err(|e| IconError::Malformed(e.to_string()))
}

/// Strips any namespace prefix, so `svg:path` and `path` are treated alike and
/// a prefix cannot be used to slip an element past the allow-list.
fn local_name(raw: &[u8]) -> String {
    let name = String::from_utf8_lossy(raw);
    match name.rsplit_once(':') {
        Some((_, local)) => local.to_string(),
        None => name.to_string(),
    }
}

/// Rebuilds a start tag carrying only allow-listed attributes whose values look
/// inert.
fn filter_attributes(name: &str, e: &BytesStart<'_>) -> Result<BytesStart<'static>, IconError> {
    let mut out = BytesStart::new(name.to_string());

    for attr in e.attributes().with_checks(false) {
        let attr = attr.map_err(|err| IconError::Malformed(err.to_string()))?;
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();

        // Compared case-insensitively because SVG attribute names are
        // case-sensitive but hostile input is not obliged to be tidy.
        if !ALLOWED_ATTRS.iter().any(|a| a.eq_ignore_ascii_case(&key)) {
            continue;
        }

        let value = String::from_utf8_lossy(&attr.value).to_string();
        let lowered = value.to_ascii_lowercase();
        // `url(…)` can reach an external resource or a filter; a `javascript:`
        // scheme is self-explanatory. Both are dropped even on an allowed
        // attribute, because the attribute name alone does not make a value safe.
        if lowered.contains("url(") || lowered.contains("javascript:") {
            continue;
        }

        out.push_attribute((key.as_str(), value.as_str()));
    }

    Ok(out)
}
```

Create `crates/core/src/icons/mod.rs`:

```rust
pub mod sanitize;

pub use sanitize::{sanitize_svg, MAX_SVG_BYTES};
```

In `crates/core/src/lib.rs`, add `pub mod icons;` to the module list and extend the error re-export:

```rust
pub use error::{
    DetectorError, GitError, IconError, LauncherError, ProjectError, RepositoryError, UnrealError,
};
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p indexer-core sanitize`
Expected: PASS (12 tests).

- [ ] **Step 7: Run the full gate and commit**

```bash
git add crates/core/Cargo.toml Cargo.lock crates/core/src/error/icon.rs \
        crates/core/src/error/mod.rs crates/core/src/error/project_error.rs \
        crates/core/src/icons/mod.rs crates/core/src/icons/sanitize.rs crates/core/src/lib.rs
git commit -m "feat(core): sanitize user-supplied SVG icons

An SVG is a document, not an image: it can carry script, event handlers,
foreignObject and external references, so an untrusted one inlined in the
webview would be XSS with full invoke reach.

Allow-list, never deny-list — a deny-list is wrong the moment SVG gains
an element nobody here has heard of. Disallowed elements take their whole
subtree with them, so a script's text content cannot survive its tag.
Namespace prefixes are stripped before matching so svg:script cannot slip
past, and url(…) or javascript: values are dropped even on an allowed
attribute, because the attribute name alone does not make a value safe.

This is the first of two layers; the render path is <img src=\"data:…\">,
which cannot execute whatever this misses. Never inline the result.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 9: `IconStore` and the icon commands

**Files:**
- Create: `crates/core/src/infra/icon_store.rs`
- Modify: `crates/core/src/infra/mod.rs`
- Create: `src-tauri/src/commands/icons.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `sanitize_svg`, `IconError` (Task 8).
- Produces: `IconStore::new(dir: PathBuf) -> IconStore`; `import(&self, source: &Path) -> Result<StoredIcon, IconError>`; `list(&self) -> Result<Vec<StoredIcon>, IconError>`; `delete(&self, name: &str) -> Result<(), IconError>`; `StoredIcon { name: String, svg: String }`; commands `list_custom_icons`, `import_custom_icon`, `delete_custom_icon`.

**Note on the data URI.** `StoredIcon` carries the sanitized **SVG source**, not a data URI. The frontend builds `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}` itself, which avoids adding a base64 dependency to core for a string the browser can assemble in one expression.

- [ ] **Step 1: Write the failing tests**

Create `crates/core/src/infra/icon_store.rs` with only this test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"<svg viewBox="0 0 24 24"><path d="M3 6h18"/></svg>"#;

    fn store() -> (tempfile::TempDir, IconStore) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = IconStore::new(dir.path().join("icons"));
        (dir, store)
    }

    fn source(dir: &std::path::Path, file: &str, body: &str) -> std::path::PathBuf {
        let path = dir.join(file);
        std::fs::write(&path, body).expect("write source");
        path
    }

    #[test]
    fn imports_sanitizes_and_lists_an_icon() {
        let (dir, store) = store();
        let src = source(dir.path(), "My Logo.svg", GOOD);

        let imported = store.import(&src).expect("import");
        assert_eq!(imported.name, "my-logo");
        assert!(imported.svg.contains("M3 6h18"));

        let listed = store.list().expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "my-logo");
    }

    #[test]
    fn import_rejects_a_hostile_svg_rather_than_storing_it() {
        let (dir, store) = store();
        let src = source(dir.path(), "bad.svg", r#"<svg viewBox="0 0 1 1"><script>alert(1)</script></svg>"#);

        assert!(store.import(&src).is_err());
        assert!(store.list().expect("list").is_empty(), "nothing hostile may reach the store");
    }

    #[test]
    fn import_stores_the_sanitized_form_not_the_original() {
        let (dir, store) = store();
        let src = source(
            dir.path(),
            "mixed.svg",
            r#"<svg viewBox="0 0 1 1"><script>alert(1)</script><path d="M0 0"/></svg>"#,
        );

        store.import(&src).expect("import");
        let stored = &store.list().expect("list")[0];
        assert!(!stored.svg.contains("script"));
        assert!(!stored.svg.contains("alert"));
    }

    #[test]
    fn a_second_icon_with_the_same_name_does_not_overwrite_the_first() {
        let (dir, store) = store();
        let a = source(dir.path(), "logo.svg", GOOD);
        std::fs::create_dir_all(dir.path().join("other")).expect("subdir");
        let b = source(&dir.path().join("other"), "logo.svg", GOOD);

        let first = store.import(&a).expect("first");
        let second = store.import(&b).expect("second");

        assert_eq!(first.name, "logo");
        assert_eq!(second.name, "logo-2");
        assert_eq!(store.list().expect("list").len(), 2);
    }

    #[test]
    fn deletes_an_icon() {
        let (dir, store) = store();
        let src = source(dir.path(), "logo.svg", GOOD);
        store.import(&src).expect("import");

        store.delete("logo").expect("delete");

        assert!(store.list().expect("list").is_empty());
    }

    #[test]
    fn delete_refuses_a_name_that_could_escape_the_store() {
        let (_dir, store) = store();
        assert!(store.delete("../projects.db").is_err());
        assert!(store.delete("nested/name").is_err());
        assert!(store.delete("..").is_err());
    }

    #[test]
    fn listing_an_absent_directory_is_empty_not_an_error() {
        let (_dir, store) = store();
        assert!(store.list().expect("list must tolerate a store never written to").is_empty());
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p indexer-core icon_store`
Expected: FAIL — `cannot find struct IconStore`.

- [ ] **Step 3: Implement the store**

At the top of `crates/core/src/infra/icon_store.rs`, above the test module:

```rust
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::IconError;
use crate::icons::sanitize_svg;

/// One custom icon, as the frontend consumes it.
///
/// `svg` is the **sanitized** source, not a data URI: the frontend builds
/// `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}` itself, which
/// saves core a base64 dependency for a string the browser assembles in one
/// expression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredIcon {
    pub name: String,
    pub svg: String,
}

/// A directory of user-supplied icons, sanitized on the way in.
///
/// Knows nothing about Tauri — `src-tauri` resolves the config directory and
/// passes the path in, which is what keeps this in `core`.
pub struct IconStore {
    dir: PathBuf,
}

impl IconStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Reads `source`, sanitizes it, and writes the result under a name derived
    /// from the file stem. **Nothing unsanitized is ever written**, so a
    /// rejected icon leaves the store untouched.
    pub fn import(&self, source: &Path) -> Result<StoredIcon, IconError> {
        let raw = std::fs::read_to_string(source)
            .map_err(|e| IconError::Io(format!("{}: {e}", source.display())))?;
        let svg = sanitize_svg(&raw)?;

        let stem = source
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let name = self.unique_name(&slugify(&stem))?;

        std::fs::create_dir_all(&self.dir)
            .map_err(|e| IconError::Io(format!("{}: {e}", self.dir.display())))?;
        let path = self.dir.join(format!("{name}.svg"));
        std::fs::write(&path, &svg).map_err(|e| IconError::Io(format!("{}: {e}", path.display())))?;

        Ok(StoredIcon { name, svg })
    }

    /// Every stored icon. A store that has never been written to is empty
    /// rather than an error — the directory is created lazily on first import.
    pub fn list(&self) -> Result<Vec<StoredIcon>, IconError> {
        if !self.dir.exists() {
            return Ok(Vec::new());
        }
        let entries = std::fs::read_dir(&self.dir)
            .map_err(|e| IconError::Io(format!("{}: {e}", self.dir.display())))?;

        let mut out = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| IconError::Io(e.to_string()))?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("svg") {
                continue;
            }
            let name = match path.file_stem().and_then(|s| s.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };
            let svg = std::fs::read_to_string(&path)
                .map_err(|e| IconError::Io(format!("{}: {e}", path.display())))?;
            out.push(StoredIcon { name, svg });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    pub fn delete(&self, name: &str) -> Result<(), IconError> {
        let path = self.dir.join(format!("{}.svg", safe_name(name)?));
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(IconError::Io(format!("{}: {e}", path.display()))),
        }
    }

    /// Appends `-2`, `-3`, … rather than overwriting. Importing a second
    /// `logo.svg` from a different folder must not silently replace the first.
    fn unique_name(&self, base: &str) -> Result<String, IconError> {
        let taken: Vec<String> = self.list()?.into_iter().map(|i| i.name).collect();
        if !taken.contains(&base.to_string()) {
            return Ok(base.to_string());
        }
        for n in 2..1000 {
            let candidate = format!("{base}-{n}");
            if !taken.contains(&candidate) {
                return Ok(candidate);
            }
        }
        Err(IconError::Io(format!("too many icons named {base}")))
    }
}

/// Lowercase, ASCII alphanumerics and hyphens only. Everything else collapses
/// to a hyphen, so a name can never carry a separator, a `..`, or anything else
/// that would let it address a file outside the store.
fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut last_hyphen = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_hyphen = false;
        } else if !last_hyphen && !out.is_empty() {
            out.push('-');
            last_hyphen = true;
        }
    }
    let trimmed = out.trim_end_matches('-').to_string();
    if trimmed.is_empty() {
        "icon".to_string()
    } else {
        trimmed
    }
}

/// Guards the one path where a name arrives from outside rather than being
/// produced by `slugify`. Path traversal is the risk: the store sits next to
/// `projects.db`.
fn safe_name(name: &str) -> Result<String, IconError> {
    let ok = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if ok {
        Ok(name.to_string())
    } else {
        Err(IconError::Io(format!("not a valid icon name: {name:?}")))
    }
}
```

In `crates/core/src/infra/mod.rs`:

```rust
pub mod icon_store;
pub mod sqlite_repository;

pub use icon_store::{IconStore, StoredIcon};
pub use sqlite_repository::{SqliteRepository, CURRENT_SCHEMA_VERSION};
```

In `crates/core/src/lib.rs`, extend the infra re-export:

```rust
pub use infra::{IconStore, SqliteRepository, StoredIcon, CURRENT_SCHEMA_VERSION};
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p indexer-core icon_store`
Expected: PASS (7 tests).

- [ ] **Step 5: Add the commands**

Create `src-tauri/src/commands/icons.rs`:

```rust
use std::sync::Arc;

use tauri::State;

use indexer_core::error::ProjectError;
use indexer_core::infra::{IconStore, StoredIcon};

#[tauri::command]
pub fn list_custom_icons(store: State<'_, Arc<IconStore>>) -> Result<Vec<StoredIcon>, ProjectError> {
    Ok(store.list()?)
}

/// Reads an SVG from `path`, sanitizes it, and stores the result. The original
/// is never copied — only the sanitized form reaches the store.
#[tauri::command]
pub fn import_custom_icon(
    store: State<'_, Arc<IconStore>>,
    path: String,
) -> Result<StoredIcon, ProjectError> {
    Ok(store.import(std::path::Path::new(&path))?)
}

#[tauri::command]
pub fn delete_custom_icon(
    store: State<'_, Arc<IconStore>>,
    name: String,
) -> Result<(), ProjectError> {
    Ok(store.delete(&name)?)
}
```

In `src-tauri/src/commands/mod.rs`, add `pub mod icons;`.

- [ ] **Step 6: Wire the store into startup**

In `src-tauri/src/lib.rs`, add a resolver next to `open_repository`:

```rust
/// The icon store lives beside `projects.db` in the app config directory. The
/// directory itself is created lazily on first import, so a missing one is not
/// a startup failure.
fn icon_store(app: &tauri::App) -> Result<IconStore, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("could not locate the app config directory: {e}"))?;
    Ok(IconStore::new(dir.join("icons")))
}
```

In the `.setup` closure, after the two services are managed:

```rust
            let icons = match icon_store(app) {
                Ok(store) => store,
                Err(e) => {
                    fatal_startup_error(&format!("Project Indexer can't start:\n\n{e}"));
                }
            };
            app.manage(Arc::new(icons));
```

Add to the imports near the top:

```rust
use indexer_core::infra::{IconStore, SqliteRepository};
```

(replacing the existing `use indexer_core::infra::SqliteRepository;` line).

Register the commands in `tauri::generate_handler![…]`, after the group commands:

```rust
            commands::icons::list_custom_icons,
            commands::icons::import_custom_icon,
            commands::icons::delete_custom_icon
```

- [ ] **Step 7: Verify the app starts and the commands work**

Run: `cargo build --workspace`
Expected: clean build.

Then launch the real app: `pnpm tauri dev`

Expected: the window appears and the project list loads. In the webview devtools console, confirm the backend is reachable end to end:

```js
await window.__TAURI__.core.invoke("list_groups")   // → []
await window.__TAURI__.core.invoke("list_custom_icons")  // → []
```

- [ ] **Step 8: Run the full gate and commit**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets && cargo test --workspace`

```bash
git add crates/core/src/infra/icon_store.rs crates/core/src/infra/mod.rs \
        crates/core/src/lib.rs src-tauri/src/commands/icons.rs \
        src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat: store custom icons, sanitized on import

The store sits beside projects.db and holds only sanitized SVG — a
rejected icon leaves it untouched, so nothing hostile is ever written to
disk and nothing unsanitized is ever read back.

Names are slugified to lowercase alphanumerics and hyphens, which is also
what makes them safe as filenames: a name can never carry a separator or
a '..'. delete() re-validates, because that is the one path where a name
arrives from outside rather than being produced by slugify. Importing a
second logo.svg appends -2 rather than overwriting the first.

StoredIcon carries the SVG source rather than a data URI, so core needs
no base64 dependency for a string the frontend assembles in one
expression.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Done when

- `cargo test --workspace` passes, with **52 new tests** on top of the existing 105 (11 in Task 1, 3 in Task 2, 4 in Task 3, 2 in Task 4, 6 in Task 5, 7 in Task 6, 12 in Task 8, 7 in Task 9).
- `cargo clippy --workspace --all-targets` is still at the one-warning `module-inception` baseline.
- `pnpm run check` reports 0 errors (the 8 `state_referenced_locally` warnings are PI-003 and expected).
- The app launches, and `list_groups` and `list_custom_icons` both return `[]` from the devtools console.
- `docs/handoffs/2026-09-05-project-views-frontend.md` is accurate — if any interface in this plan changed during implementation, update the handoff before finishing.
