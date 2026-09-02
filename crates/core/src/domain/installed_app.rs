use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct InstalledApp {
    pub name: String,
    pub path: String,
}
