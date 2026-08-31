use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub repo_root: String,
    pub dirty: bool,
    pub detached_head: bool,
    pub repo_url: Option<String>,
    pub contributors: Vec<String>,
    pub curr_branch: Option<String>,
    pub branches: Option<Vec<String>>,
    pub commit_hash: Option<String>,
}
