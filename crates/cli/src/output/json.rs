//! `--json`: every response is `{"schema": 1, "data": …}`, and every failure
//! `{"schema": 1, "error": …}` on stderr.
//!
//! The contract is settled in `docs/cli/ROADMAP.md` → *The `--json` contract*:
//! additive-only within a schema version, unknown tracker kinds serialise
//! rather than fail, and stdout holds the document and nothing else. The shapes
//! an agent reads are written down in `docs/cli/agents.md`.

use std::io::Write;

use chrono::{DateTime, Utc};
use indexer_core::domain::Tracker;
use indexer_core::Project;
use serde::ser::{Error as _, SerializeMap};
use serde::{Serialize, Serializer};

use crate::commands::{Failure, Outcome};

/// Bumped only for a change that breaks a reader.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize)]
struct Envelope<'a, T: Serialize> {
    schema: u32,
    data: &'a T,
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    schema: u32,
    error: ErrorJson<'a>,
}

#[derive(Serialize)]
struct ErrorJson<'a> {
    /// `not_found`, `ambiguous`, or `error` for anything without a kind of its own.
    kind: &'static str,
    message: String,
    /// What was searched for — for `not_found` and `ambiguous`.
    #[serde(skip_serializing_if = "Option::is_none")]
    query: Option<&'a str>,
    /// The candidates, best first — only for `ambiguous`.
    #[serde(skip_serializing_if = "Option::is_none")]
    matches: Option<Vec<ProjectJson<'a>>>,
}

/// A project as `--json` shows it.
///
/// Spelled out rather than serialising [`Project`] as-is, so the documented
/// shape can't change by accident when core renames a field, and so trackers
/// can take the contract's `{"kind": …}` form.
#[derive(Serialize)]
pub struct ProjectJson<'a> {
    id: &'a str,
    name: &'a str,
    description: &'a str,
    directory: &'a str,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_opened_at: Option<DateTime<Utc>>,
    favorite: bool,
    is_deleted: bool,
    tags: &'a [String],
    properties: &'a std::collections::BTreeMap<String, String>,
    group_id: Option<&'a str>,
    color: Option<&'a str>,
    icon: Option<&'a str>,
    open_with: Option<&'a str>,
    notes: Option<&'a str>,
    trackers: Vec<TrackerJson<'a>>,
}

impl<'a> From<&'a Project> for ProjectJson<'a> {
    fn from(p: &'a Project) -> Self {
        Self {
            id: &p.id,
            name: &p.name,
            description: &p.description,
            directory: &p.directory,
            created_at: p.created_at,
            updated_at: p.updated_at,
            last_opened_at: p.last_opened_at,
            favorite: p.favorite,
            is_deleted: p.is_deleted,
            tags: &p.tags,
            properties: &p.properties,
            group_id: p.group_id.as_deref(),
            color: p.color.as_deref(),
            icon: p.icon.as_deref(),
            open_with: p.open_with.as_deref(),
            notes: p.notes.as_deref(),
            trackers: p.trackers.iter().map(TrackerJson).collect(),
        }
    }
}

/// A tracker as `{"kind": "git", …its fields}`, rather than serde's default
/// `{"Git": {…}}` — so a reader finds the kind under one fixed key, and a kind
/// it has never heard of is still a map it can skip.
///
/// Built from the tracker's own serde shape, not a `match`, so a new detector
/// appears here with no change.
pub struct TrackerJson<'a>(&'a Tracker);

impl Serialize for TrackerJson<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let value = serde_json::to_value(self.0).map_err(S::Error::custom)?;
        let serde_json::Value::Object(outer) = value else {
            return Err(S::Error::custom("a tracker did not serialise as a map"));
        };
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("kind", &self.0.kind().to_lowercase())?;
        for (_, payload) in outer {
            if let serde_json::Value::Object(fields) = payload {
                for (key, field) in fields {
                    map.serialize_entry(&key, &field)?;
                }
            }
        }
        map.end()
    }
}

pub fn write(out: &mut impl Write, outcome: &Outcome) -> anyhow::Result<()> {
    match outcome {
        // No match is an empty array, not an error: `data` is the same shape
        // whether or not a query was given.
        Outcome::Projects { projects, .. } => {
            let projects: Vec<ProjectJson> = projects.iter().map(ProjectJson::from).collect();
            emit(out, &projects)
        }
        // The kinds only pick columns for the human table; JSON always carries
        // every tracker in full.
        Outcome::Project { project, .. } => emit(out, &ProjectJson::from(project.as_ref())),
        Outcome::Color { setting, color } => {
            let mut data = serde_json::Map::new();
            data.insert(setting.key().to_string(), serde_json::to_value(color)?);
            emit(out, &data)
        }
        Outcome::Done => emit(out, &serde_json::Value::Null),
    }
}

/// Writes a failure as `{"schema": 1, "error": {"kind", "message", …}}`.
/// Meant for stderr: under `--json`, stdout only ever holds a result.
pub fn write_error(out: &mut impl Write, error: &anyhow::Error) -> anyhow::Result<()> {
    let error = match error.downcast_ref::<Failure>() {
        Some(Failure::NotFound { query }) => ErrorJson {
            kind: "not_found",
            message: error.to_string(),
            query: Some(query),
            matches: None,
        },
        Some(failure @ Failure::Ambiguous { query, matches, .. }) => ErrorJson {
            kind: "ambiguous",
            message: failure.summary(),
            query: Some(query),
            matches: Some(matches.iter().map(ProjectJson::from).collect()),
        },
        None => ErrorJson {
            kind: "error",
            message: format!("{error:#}"),
            query: None,
            matches: None,
        },
    };
    let envelope = ErrorEnvelope {
        schema: SCHEMA_VERSION,
        error,
    };
    serde_json::to_writer_pretty(&mut *out, &envelope)?;
    writeln!(out)?;
    Ok(())
}

fn emit<T: Serialize>(out: &mut impl Write, data: &T) -> anyhow::Result<()> {
    let envelope = Envelope {
        schema: SCHEMA_VERSION,
        data,
    };
    serde_json::to_writer_pretty(&mut *out, &envelope)?;
    writeln!(out)?;
    Ok(())
}
