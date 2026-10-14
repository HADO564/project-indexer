use serde::{Deserialize, Serialize};

/// What the Unity detector gathers from
/// `ProjectSettings/ProjectVersion.txt`: the editor version the project was
/// last saved with (e.g. `2021.3.0f1`), when the file records one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnityInfo {
    pub editor_version: Option<String>,
}
