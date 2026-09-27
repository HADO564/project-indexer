//! The arguments shared by the commands that act on one project — `show`,
//! `open`, `untrack`, `favorite`, `unfavorite` and `edit` — which all resolve
//! their query through [`commands::find_one`](crate::commands::find_one).

use clap::Parser;

use crate::commands::{Command, TrackerKind};
use crate::{Cli, Invocation};

#[test]
fn open_takes_a_project_and_the_tracker_flag() {
    let cli = Cli::try_parse_from(["indexer", "open", "app", "-t", "git,unreal"])
        .expect("arguments should parse");
    let Some(Invocation::Command(Command::Open(args))) = cli.invocation else {
        panic!("expected `open`");
    };

    assert_eq!(args.project, "app");
    assert_eq!(args.tracker, vec![TrackerKind::Git, TrackerKind::Unreal]);
}

#[test]
fn untrack_takes_a_project_and_the_tracker_flag() {
    let cli = Cli::try_parse_from(["indexer", "untrack", "app", "--tracker", "git"])
        .expect("arguments should parse");
    let Some(Invocation::Command(Command::Untrack(args))) = cli.invocation else {
        panic!("expected `untrack`");
    };

    assert_eq!(args.project, "app");
    assert_eq!(args.tracker, vec![TrackerKind::Git]);
}

#[test]
fn favorite_and_unfavorite_take_a_project_and_the_tracker_flag() {
    let cli = Cli::try_parse_from(["indexer", "favorite", "app", "-t", "git"])
        .expect("arguments should parse");
    let Some(Invocation::Command(Command::Favorite(args))) = cli.invocation else {
        panic!("expected `favorite`");
    };
    assert_eq!(args.project, "app");
    assert_eq!(args.tracker, vec![TrackerKind::Git]);

    // Two verbs over one argument struct: `unfavorite` must parse into its own
    // variant, or `run` would store the wrong flag.
    let cli =
        Cli::try_parse_from(["indexer", "unfavorite", "work/app"]).expect("arguments should parse");
    let Some(Invocation::Command(Command::Unfavorite(args))) = cli.invocation else {
        panic!("expected `unfavorite`");
    };
    assert_eq!(args.project, "work/app");
    assert!(args.tracker.is_empty());
}

#[test]
fn edit_takes_a_project_and_a_description() {
    let cli = Cli::try_parse_from(["indexer", "edit", "app", "--description", "The gateway"])
        .expect("arguments should parse");
    let Some(Invocation::Command(Command::Edit(args))) = cli.invocation else {
        panic!("expected `edit`");
    };
    assert_eq!(args.project, "app");
    assert_eq!(args.description.as_deref(), Some("The gateway"));

    // An empty description is a value — it clears the field — not a missing
    // flag, so it must reach `update` as `Some("")` rather than `None`.
    let cli = Cli::try_parse_from(["indexer", "edit", "app", "--description", ""])
        .expect("an empty description should parse");
    let Some(Invocation::Command(Command::Edit(args))) = cli.invocation else {
        panic!("expected `edit`");
    };
    assert_eq!(args.description.as_deref(), Some(""));
}

#[test]
fn edit_without_a_change_is_a_usage_error() {
    // The `change` group requires at least one field flag: an edit that
    // changes nothing is almost certainly a mistake, and a usage error says
    // so with exit code 2 before anything is looked up.
    let err =
        Cli::try_parse_from(["indexer", "edit", "app"]).expect_err("a bare edit should be refused");
    assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
}

#[test]
fn a_project_is_required() {
    // Unlike `add`, whose directory defaults to the current one: there is no
    // sensible default project until `show .` and the TUI's selection exist.
    assert!(Cli::try_parse_from(["indexer", "open"]).is_err());
    assert!(Cli::try_parse_from(["indexer", "untrack"]).is_err());
    assert!(Cli::try_parse_from(["indexer", "favorite"]).is_err());
    assert!(Cli::try_parse_from(["indexer", "unfavorite"]).is_err());
    assert!(Cli::try_parse_from(["indexer", "edit", "--description", "x"]).is_err());
}

#[test]
fn yes_is_global_so_it_can_follow_the_project() {
    let cli = Cli::try_parse_from(["indexer", "untrack", "app", "--yes"])
        .expect("`--yes` should be accepted after the subcommand");

    assert!(cli.yes);
}
