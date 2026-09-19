//! Failures a script may want to tell apart, rather than read as prose.
//!
//! Anything else stays a plain `anyhow` error; `--json` reports those with
//! the kind `error`.

use std::fmt;

use indexer_core::Project;

use super::TrackerKind;
use crate::output::human;

#[derive(Debug)]
pub enum Failure {
    /// No project matches the query.
    NotFound { query: String },
    /// Several projects match the query, best first.
    Ambiguous {
        query: String,
        matches: Vec<Project>,
        tracker: Vec<TrackerKind>,
    },
}

impl Failure {
    /// The one-line message, without the table human output adds.
    pub fn summary(&self) -> String {
        match self {
            Failure::NotFound { query } => format!("no project matches \"{query}\""),
            Failure::Ambiguous { .. } => {
                "multiple matches found, which one did you mean?".to_string()
            }
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::NotFound { .. } => write!(f, "{}", self.summary()),
            Failure::Ambiguous {
                matches, tracker, ..
            } => {
                let matches: Vec<&Project> = matches.iter().collect();
                // Plain apart from the tracker: an error is not a terminal
                // view, so no colour and no stretching to the terminal's width.
                let style = human::TableStyle {
                    tracker: tracker.clone(),
                    ..Default::default()
                };
                write!(
                    f,
                    "{}\n{}add a folder from the path, like parent/folder, to narrow it down",
                    self.summary(),
                    human::project_table(&matches, &style)
                )
            }
        }
    }
}

impl std::error::Error for Failure {}
