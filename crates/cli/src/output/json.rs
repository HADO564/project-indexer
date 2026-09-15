//! `--json`: every response is `{"schema": 1, "data": …}`.
//!
//! The contract is settled in `docs/cli/ROADMAP.md` → *The `--json` contract*:
//! additive-only within a schema version, unknown tracker kinds serialise
//! rather than fail, and stdout holds the document and nothing else.

use std::io::Write;

use serde::Serialize;

use crate::commands::Outcome;

/// Bumped only for a change that breaks a reader.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize)]
struct Envelope<'a, T: Serialize> {
    schema: u32,
    data: &'a T,
}

pub fn write(out: &mut impl Write, outcome: &Outcome) -> anyhow::Result<()> {
    match outcome {
        Outcome::Projects(projects) => emit(out, projects),
        Outcome::Project(project) => emit(out, project),
        Outcome::Color { setting, color } => {
            let mut data = serde_json::Map::new();
            data.insert(setting.key().to_string(), serde_json::to_value(color)?);
            emit(out, &data)
        }
        Outcome::Done => emit(out, &serde_json::Value::Null),
    }
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
