use serde::{Deserialize, Serialize};

/// What the Rust detector gathers from a `Cargo.toml`: the package name, or
/// `None` for a virtual-manifest workspace root that declares no `[package]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustInfo {
    pub name: Option<String>,
}
