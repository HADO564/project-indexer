use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnrealInfo {
    pub project_root: String,
    pub project_name: String,
    pub uproject_path: String,
    pub engine_association: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub modules: Vec<String>,
    pub plugins: Vec<String>,
    pub vcs_provider: Option<String>,
}
