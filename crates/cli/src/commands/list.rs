use clap::{Args, ValueEnum};

use crate::context::Context;
use indexer_core::domain::matching::filter;
use indexer_core::domain::sorting::{SortBy, SortDirection, SortOptions};

use super::{unique_kinds, with_tracker, Outcome, TrackerKind, View};

/// The field `--sort` orders by, as `name` and `last-opened` on the command
/// line.
///
/// Mirrors [`SortBy`] rather than reusing it, for the reason [`TrackerKind`]
/// mirrors core's tracker kinds: `ValueEnum` is a clap concern that core
/// should not carry, and core's own spelling (`Alphabetical`) is not the word
/// anyone reaches for at a prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SortByKind {
    /// By name, A to Z.
    Name,
    /// By when it was last opened, most recent first.
    LastOpened,
}

impl SortByKind {
    /// The spelling clap parses, which is also what `--help` prints as the
    /// default. The two must agree, or the help would name a value the
    /// command refuses.
    pub fn name(self) -> &'static str {
        match self {
            SortByKind::Name => "name",
            SortByKind::LastOpened => "last-opened",
        }
    }

    /// Which way to sort this field, given `--reverse`.
    ///
    /// **`--reverse` means the opposite of this field's natural order, not
    /// "descending".** The two fields disagree about which way is natural:
    /// names read A to Z, while "last opened" is only useful most-recent
    /// first, which core calls `Descending`. Leaving the direction at
    /// `SortDirection::default()` — `Ascending` — would make
    /// `list --sort last-opened` answer with the stalest projects first.
    pub fn direction(self, reverse: bool) -> SortDirection {
        use SortDirection::{Ascending, Descending};

        match self {
            SortByKind::Name => {
                if reverse {
                    Descending
                } else {
                    Ascending
                }
            }
            SortByKind::LastOpened => {
                if reverse {
                    Ascending
                } else {
                    Descending
                }
            }
        }
    }
}

/// The core field this flag names.
///
/// Written as `From` rather than an inherent `sort_by` method so the call site
/// can say `args.sort.into()` wherever the target type is already known — the
/// standard conversion every Rust reader recognises. Implementing it also
/// yields `SortByKind: Into<SortBy>` for free.
impl From<SortByKind> for SortBy {
    fn from(kind: SortByKind) -> Self {
        match kind {
            SortByKind::Name => SortBy::Alphabetical,
            SortByKind::LastOpened => SortBy::LastOpened,
        }
    }
}

// `default_value_t` renders the default value back into the text clap would
// have parsed, so the value has to know how to print itself.
impl std::fmt::Display for SortByKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Only list projects whose name contains this, or, when it has a `/`,
    /// whose path ends with it (`work/app`). Ignores case.
    pub query: Option<String>,

    /// Order the table by this field. Each field has its own natural
    /// direction — names run A to Z, last opened runs most recent first.
    #[arg(long, short = 's', value_enum, default_value_t = SortByKind::Name)]
    pub sort: SortByKind,

    /// Flip the sort: Z to A by name, oldest first by last opened.
    #[arg(long, short = 'r')]
    pub reverse: bool,

    /// Only include projects with one of these trackers, before the query is
    /// matched. Repeat the flag or separate the kinds with commas.
    #[arg(long, short = 't', value_enum, value_delimiter = ',')]
    pub tracker: Vec<TrackerKind>,

    /// Which projects to draw from. Binned projects appear only under `binned`.
    #[arg(long, value_enum, default_value_t = View::All)]
    pub view: View,
}

pub fn run(args: ListArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    let options = SortOptions {
        by: args.sort.into(),
        direction: args.sort.direction(args.reverse),
    };

    // The view picks the *source*, not a filter over one. `list_favorites`
    // excludes binned projects itself and `list_deleted` ignores `favorite`
    // entirely, so the three sets are core's to define, not the CLI's — the
    // same split `src/lib/views.ts` makes, where only the bin swaps the list
    // being filtered.
    let projects = match args.view {
        View::All => ctx.projects.list(options)?,
        View::Favorites => ctx.projects.list_favorites(options)?,
        View::Binned => ctx.projects.list_deleted(options)?,
    };

    // Narrowing happens after the sort, and both steps preserve order, so the
    // rows keep the sequence core put them in.
    let tracker = unique_kinds(&args.tracker);
    let projects = with_tracker(projects, &tracker);
    let projects = match &args.query {
        Some(query) => filter(&projects, query).into_iter().cloned().collect(),
        None => projects,
    };
    Ok(Outcome::Projects {
        projects,
        query: args.query,
        tracker,
        view: args.view,
    })
}
