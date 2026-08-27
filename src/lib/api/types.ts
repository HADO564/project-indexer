// Mirrors src-tauri/src/models/project.rs and update_project.rs.
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
  client: string | null;
  trackers: Tracker[];
}

// Mirrors src-tauri/src/models/git.rs
export interface GitInfo {
  repo_root: string;
  dirty: boolean;
  detached_head: boolean;
  repo_url: string | null;
  contributors: string[];
  curr_branch: string | null;
  branches: string[] | null;
  commit_hash: string | null;
}

// Mirrors src-tauri/src/models/unreal.rs
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

// Mirrors src-tauri/src/models/tracker.rs. Serde's default (externally
// tagged) enum representation: a variant with data becomes `{ VariantName:
// <data> }`, a plain unit variant becomes just its name as a string.
export type Tracker =
  | { Git: GitInfo }
  | { Unreal: UnrealInfo }
  | "Unity"
  | "Blender";

// Partial update: omit a key to leave that field unchanged. For
// open_with/notes/client, an explicit `null` clears the field (the Rust
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
  client?: string | null;
}

export interface CreateProjectInput {
  name: string;
  directory: string;
  description?: string | null;
  tags?: string[] | null;
}

// Mirrors src-tauri/src/models/installed_app.rs
export interface InstalledApp {
  name: string;
  path: string;
}

// Mirrors src-tauri/src/utils/sorting.rs. Used by get_favorite_projects and
// get_deleted_projects; omit entirely to get the backend default
// (alphabetical, ascending).
export type SortBy = "alphabetical" | "last_opened";
export type SortDirection = "ascending" | "descending";

export interface SortOptions {
  by: SortBy;
  direction: SortDirection;
}
