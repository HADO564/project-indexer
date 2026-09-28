//! Failures a script may want to tell apart, rather than read as prose.
//!
//! Anything else stays a plain `anyhow` error; `--json` reports those with
//! the kind `error`.

use std::fmt;

use indexer_core::Project;

use super::{Corpus, TrackerKind};
use crate::output::human;

#[derive(Debug)]
pub enum Failure {
    /// No project matches the query. `corpus` is where it looked, so a
    /// `restore` of a project that is not in the bin says so rather than
    /// claiming nothing matches at all.
    NotFound { query: String, corpus: Corpus },
    /// Several projects match the query, best first.
    Ambiguous {
        query: String,
        matches: Vec<Project>,
        tracker: Vec<TrackerKind>,
    },
    /// The command line was fine as far as clap could tell, but cannot be
    /// carried out as given — a bare `edit` where no form can be opened. `main`
    /// prints it as prose and exits 2, as clap does for its own usage errors,
    /// even under `--json`. `message` is written by whoever raises it, so it
    /// can say what to type instead.
    Usage { message: String },
}

impl Failure {
    /// The one-line message, without the table human output adds.
    pub fn summary(&self) -> String {
        match self {
            Failure::NotFound {
                query,
                corpus: Corpus::Live,
            } => format!("no project matches \"{query}\""),
            Failure::NotFound {
                query,
                corpus: Corpus::Bin,
            } => format!("no project in the bin matches \"{query}\""),
            Failure::Ambiguous { .. } => {
                "multiple matches found, which one did you mean?".to_string()
            }
            Failure::Usage { message } => message.clone(),
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
            Failure::Usage { .. } => write!(f, "{}", self.summary()),
        }
    }
}

impl std::error::Error for Failure {}
