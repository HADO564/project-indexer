use serde::{Deserialize, Serialize};

/// What the Maven detector gathers from a `pom.xml`: the project's
/// `<artifactId>` and `<version>`, when the file declares them. Each is `None`
/// for a manifest that omits it or that couldn't be parsed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MavenInfo {
    pub artifact_id: Option<String>,
    pub version: Option<String>,
}
