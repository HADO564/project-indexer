use serde::{Deserialize, Serialize};

/// What the Godot detector gathers from a `project.godot`: the configured
/// project name, when the file declares one under `[application]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodotInfo {
    pub project_name: Option<String>,
}
