// Mirrors src-tauri/src/models/project.rs and update_project.rs.
// Dates stay as ISO strings (chrono::DateTime<Utc> serializes to RFC3339).

export interface Project {
  id: string;
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
}

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
