use serde::{Deserialize, Serialize};

/// What the Swift detector gathers from a `Package.swift`: the package name
/// from the `Package(name: "...")` initializer, when it can be read. `None`
/// for a manifest the scan can't make sense of — the file's presence is the
/// signal either way.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwiftInfo {
    pub name: Option<String>,
}
