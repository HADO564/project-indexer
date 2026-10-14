use serde::{Deserialize, Serialize};

/// What the PHP detector gathers from a `composer.json`: the package `name`,
/// usually `vendor/package`, or `None` when the manifest omits it (an
/// application often declares none) or couldn't be parsed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhpInfo {
    pub name: Option<String>,
}
