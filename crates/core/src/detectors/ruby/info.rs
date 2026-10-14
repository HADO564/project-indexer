use serde::{Deserialize, Serialize};

/// What the Ruby detector gathers from a `Gemfile`: the pinned interpreter
/// version from a `ruby "x.y.z"` directive, when one is declared. A Gemfile has
/// no project name, and many omit the `ruby` line, so this is often `None` —
/// the file's presence is the signal either way.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubyInfo {
    pub ruby_version: Option<String>,
}
