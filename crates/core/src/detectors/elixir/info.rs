use serde::{Deserialize, Serialize};

/// What the Elixir detector gathers from a `mix.exs`: the OTP application name
/// from the project's `app: :name` key, when it can be read. `None` for a
/// manifest the scan can't make sense of — the file's presence is the signal
/// either way.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElixirInfo {
    pub app: Option<String>,
}
