use serde::{Deserialize, Serialize};

/// What the Docker detector gathers: `marker` is which file proved this a
/// Docker project — a `Dockerfile`, or one of the Compose files (`compose.yaml`
/// / `compose.yml` and the older `docker-compose.*` spellings). Neither carries
/// a project name in a fixed place, so presence is the whole of it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerInfo {
    pub marker: String,
}
