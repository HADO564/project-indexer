use serde::{Deserialize, Serialize};

/// What the Python detector gathers. `marker` is which file proved this a
/// Python project — `pyproject.toml`, `setup.py` or `requirements.txt` — and
/// only the first declares a `name` in a fixed enough place to read, so the
/// other two leave `name` as `None`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonInfo {
    pub marker: String,
    pub name: Option<String>,
}
