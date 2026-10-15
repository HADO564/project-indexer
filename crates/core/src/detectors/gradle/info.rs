use serde::{Deserialize, Serialize};

/// What the Gradle detector gathers: `marker` is which build script proved this
/// a Gradle project — `build.gradle` (Groovy) or `build.gradle.kts` (Kotlin).
/// The project name lives in `settings.gradle`, not the build script, so there
/// is nothing more to read cheaply here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradleInfo {
    pub marker: String,
}
