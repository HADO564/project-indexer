use serde::{Deserialize, Serialize};

/// What the CMake detector gathers from a `CMakeLists.txt`: the name from its
/// `project(<name> ...)` command, when one can be read. `None` for a listfile
/// that declares no `project()` in a fixed enough place to scan, or one that
/// couldn't be parsed — the file's presence is the signal either way.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmakeInfo {
    pub project_name: Option<String>,
}
