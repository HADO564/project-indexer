use serde::{Deserialize, Serialize};

/// What the Node detector gathers from a `package.json`. The one manifest
/// covers npm, pnpm and yarn alike, so a single detector serves all three.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub manifest_path: String,
    /// The package `name`, or `None` when the manifest omits it or couldn't be
    /// parsed — a private app often declares none.
    pub name: Option<String>,
    /// The package `version`, with the same `None` treatment.
    pub version: Option<String>,
}
