use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub repo_root: String,
    pub dirty: bool,
    pub detached_head: bool,
    pub repo_url: Option<String>,
    /// Browser-openable form of `repo_url` (SSH → HTTPS, trailing `.git`
    /// stripped), or `None` if `repo_url` isn't a recognizable http/ssh
    /// git remote. `Option` so records written before this field load fine.
    pub web_url: Option<String>,
    pub contributors: Vec<String>,
    pub curr_branch: Option<String>,
    pub branches: Option<Vec<String>>,
    pub commit_hash: Option<String>,
}
