use serde::{Deserialize, Serialize};

/// What the .NET detector gathers: `marker` is the file that proved this a
/// .NET project — a solution (`*.sln`), a project (`*.csproj`) or an SDK pin
/// (`global.json`), with its real filename kept so `App.csproj` reads as more
/// than "some .NET project".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotnetInfo {
    pub marker: String,
}
