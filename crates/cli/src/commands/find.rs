//! `find <query>`: the app's search, as a command beside `list`.
//!
//! `list [query]` finds projects by name or path ending, the rule `show`,
//! `open` and `edit` resolve a project by. `find` searches what the GUI's
//! search bar does — the name, any part of the path, tags and property values,
//! or `name: value` for one property — by calling the very function the app
//! calls, `domain::views::matches_query`, so the two cannot drift apart.

use clap::Args;
use indexer_core::domain::sorting::SortOptions;
use indexer_core::domain::views::matches_query;

use crate::context::Context;

use super::list::SortByKind;
use super::{unique_kinds, with_tracker, Outcome, TrackerKind, View};

// The flags after `query` are `list`'s, copied rather than shared: the two
// commands draw from the same sets in the same order, and only the query
// differs.
#[derive(Debug, Args)]
pub struct FindArgs {
    /// Text to search for, or `name: value` for one property.
    ///
    /// Free text matches the name, any part of the path, tags and property
    /// values; `client: acme` matches only projects whose `client` property
    /// contains `acme`. Ignores case.
    pub query: String,

    /// Order the table (`name` or `last-opened`).
    ///
    /// Each field has its own natural direction — names run A to Z, last
    /// opened runs most recent first.
    #[arg(long, short = 's', value_enum, default_value_t = SortByKind::Name)]
    pub sort: SortByKind,

    /// Flip the sort.
    ///
    /// Z to A by name, oldest first by last opened.
    #[arg(long, short = 'r')]
    pub reverse: bool,

    /// Keep only projects with these trackers.
    ///
    /// Applied before the query is matched. Repeat the flag or separate the
    /// kinds with commas.
    #[arg(long, short = 't', value_enum, value_delimiter = ',')]
    pub tracker: Vec<TrackerKind>,

    /// Which set to draw from.
    ///
    /// Binned projects appear only under `binned`.
    #[arg(long, value_enum, default_value_t = View::All)]
    pub view: View,
}

pub fn run(args: FindArgs, ctx: &Context) -> anyhow::Result<Outcome> {
    let options = SortOptions {
        by: args.sort.into(),
        direction: args.sort.direction(args.reverse),
    };

    // The same three sources as `list`, for the reason given there: which
    // projects a view holds is core's to define.
    let projects = match args.view {
        View::All => ctx.projects.list(options)?,
        View::Favorites => ctx.projects.list_favorites(options)?,
        View::Binned => ctx.projects.list_deleted(options)?,
    };

    // Trackers first, then the query; filtering keeps the order, so the sort
    // still holds.
    let tracker = unique_kinds(&args.tracker);
    let projects = with_tracker(projects, &tracker);
    let projects = projects
        .into_iter()
        .filter(|p| matches_query(p, &args.query))
        .collect();
    Ok(Outcome::Projects {
        projects,
        query: Some(args.query),
        tracker,
        view: args.view,
        groups: ctx.groups.list()?,
    })
}
