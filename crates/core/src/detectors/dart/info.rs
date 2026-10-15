use serde::{Deserialize, Serialize};

/// What the Dart detector gathers from a `pubspec.yaml`: the package `name` and
/// `version`. The one manifest covers plain Dart and Flutter alike. Each field
/// is `None` for a manifest that omits it or that couldn't be parsed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DartInfo {
    pub name: Option<String>,
    pub version: Option<String>,
}
