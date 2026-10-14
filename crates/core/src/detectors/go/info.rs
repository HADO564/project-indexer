use serde::{Deserialize, Serialize};

/// What the Go detector gathers from a `go.mod`: the module path, which is the
/// import prefix and the closest thing Go has to a project name (e.g.
/// `github.com/user/thing`). `None` if the file declares no `module` directive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoInfo {
    pub module: Option<String>,
}
