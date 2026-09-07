// Mirrors the Rust models in crates/core/src/domain/ (project.rs,
// update_project.rs, group.rs, update_group.rs, tracker.rs, git.rs,
// unreal.rs, sorting.rs, installed_app.rs) and crates/core/src/infra/
// icon_store.rs. Hand-maintained — nothing checks it, so update it in the
// same commit as anything it mirrors.
// Dates stay as ISO strings (chrono::DateTime<Utc> serializes to RFC3339).

export interface Project {
  id: string;
  is_deleted: boolean;
  name: string;
  description: string;
  directory: string;
  created_at: string;
  updated_at: string;
  last_opened_at: string | null;
  tags: string[];
  favorite: boolean;
  open_with: string | null;
  notes: string | null;
  // User-defined key/value facts. Replaced the hardcoded `client` field, which
  // only suited one kind of user; the v3 migration moved existing client values
  // to properties["client"]. Keys are trimmed, non-empty, and contain no ":" —
  // core rejects the rest, because the search bar reads "name: value".
  properties: Record<string, string>;
  trackers: Tracker[];
  group_id: string | null;
  color: string | null;
  icon: string | null;
}

// Mirrors crates/core/src/domain/git.rs
export interface GitInfo {
  repo_root: string;
  dirty: boolean;
  detached_head: boolean;
  repo_url: string | null;
  web_url: string | null;
  contributors: string[];
  curr_branch: string | null;
  branches: string[] | null;
  commit_hash: string | null;
}

// Mirrors crates/core/src/domain/unreal.rs
export interface UnrealInfo {
  project_root: string;
  project_name: string;
  uproject_path: string;
  engine_association: string | null;
  category: string | null;
  description: string | null;
  modules: string[];
  plugins: string[];
  vcs_provider: string | null;
}

// Mirrors crates/core/src/domain/tracker.rs. Serde's default (externally
// tagged) enum representation: a variant with data becomes `{ VariantName:
// <data> }`, and a plain unit variant (none today) would be just its name as
// a string — trackers.ts handles both shapes generically.
export type Tracker =
  | { Git: GitInfo }
  | { Unreal: UnrealInfo };

// Mirrors src-tauri/src/commands/inspect.rs
export type DetectorStatus = "detected" | "not_detected" | "failed";

export interface DetectorResult {
  kind: string;
  status: DetectorStatus;
  tracker?: Tracker;
  error?: string;
}

export interface DirectoryStatus {
  ok: boolean;
  message?: string;
}

export interface ProjectInspection {
  project: Project;
  directory_status: DirectoryStatus;
  results: DetectorResult[];
}

// Partial update: omit a key to leave that field unchanged. For
// open_with/notes, an explicit `null` clears the field (the Rust
// side distinguishes "key absent" from "key present but null" via a
// double-Option deserializer), which JSON.stringify's undefined-key
// dropping matches naturally.
export interface UpdateProject {
  name?: string;
  directory?: string;
  description?: string;
  tags?: string[];
  favorite?: boolean;
  open_with?: string | null;
  notes?: string | null;
  // Replaces the whole map; an empty object clears every property. Not
  // nullable, so there is no absent-vs-null case to distinguish.
  properties?: Record<string, string>;
  group_id?: string | null;
  color?: string | null;
  icon?: string | null;
}

export interface CreateProjectInput {
  name: string;
  directory: string;
  description?: string | null;
  tags?: string[] | null;
}

// Mirrors crates/core/src/domain/installed_app.rs
export interface InstalledApp {
  name: string;
  path: string;
}

// Mirrors crates/core/src/domain/sorting.rs. Used by get_favorite_projects and
// get_deleted_projects; omit entirely to get the backend default
// (alphabetical, ascending).
export type SortBy = "alphabetical" | "last_opened";
export type SortDirection = "ascending" | "descending";

export interface SortOptions {
  by: SortBy;
  direction: SortDirection;
}

// Mirrors crates/core/src/domain/group.rs. `color` is a palette name (see
// palette.ts); `icon` is a bundled icon name (see icons.ts) — core validates
// only that it is non-empty, because it does not own the bundled set, so an
// unknown name falls back to a default glyph at render rather than failing.
export interface Group {
  id: string;
  name: string;
  color: string;
  icon: string;
  position: number;
  created_at: string;
  updated_at: string;
}

// Mirrors crates/core/src/domain/update_group.rs. Unlike UpdateProject, no
// field here is nullable — a group always has a name, a colour and an icon —
// so an omitted key means unchanged and there is no "clear" case.
// `position` is deliberately absent: reordering goes through reorderGroups,
// which rewrites the whole ordering, so there is exactly one path that
// renumbers.
export interface UpdateGroup {
  name?: string;
  color?: string;
  icon?: string;
}

// Mirrors crates/core/src/infra/icon_store.rs. `svg` is sanitized SVG
// *source*, not a data URI — core returns source so it needs no base64
// dependency; the browser assembles the URI in one expression. See
// customIconSrc in src/lib/icons.ts, which is the only place that happens.
export interface StoredIcon {
  name: string;
  svg: string;
}

// Mirrors crates/core/src/domain/scan.rs and
// crates/core/src/application/scan_service.rs.
//
// ScanMode is #[serde(tag = "mode", rename_all = "lowercase")] and is
// #[serde(flatten)]ed into ScanRequest, so the wire shape is a flat object:
// { scan_root, mode: "quick", detectors, include_ignored } or
// { scan_root, mode: "deep", depth: 3, detectors, include_ignored }.
export type ScanRequest =
  | {
      scan_root: string;
      mode: "quick";
      detectors: string[];
      include_ignored: boolean;
    }
  | {
      scan_root: string;
      mode: "deep";
      depth: number;
      detectors: string[];
      include_ignored: boolean;
    };

export interface Candidate {
  directory: string;
  suggested_name: string;
  matched_kinds: string[];
  already_tracked: boolean;
  // True when ScanService::scan changed suggested_name to resolve a
  // collision — distinct from the user editing the name themselves, which
  // the frontend tracks separately by comparing against this field.
  disambiguated: boolean;
}

export interface ScanReport {
  candidates: Candidate[];
  visited: number;
  // True when MAX_DIRECTORIES (50,000) was reached, so the results are
  // incomplete rather than the disk holding no more.
  stopped_early: boolean;
}

export interface ImportSelection {
  directory: string;
  name: string;
}

export interface ImportFailure {
  directory: string;
  message: string;
}

export interface ImportReport {
  imported: Project[];
  skipped: number;
  failures: ImportFailure[];
}
