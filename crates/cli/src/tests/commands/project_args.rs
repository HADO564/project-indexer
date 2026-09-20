//! The arguments shared by the commands that act on one project — `show`,
//! `open` and `untrack` — which all resolve their query through
//! [`commands::find_one`](crate::commands::find_one).

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
fn a_project_is_required() {
    // Unlike `add`, whose directory defaults to the current one: there is no
    // sensible default project until `show .` and the TUI's selection exist.
    assert!(Cli::try_parse_from(["indexer", "open"]).is_err());
    assert!(Cli::try_parse_from(["indexer", "untrack"]).is_err());
}

#[test]
fn yes_is_global_so_it_can_follow_the_project() {
    let cli = Cli::try_parse_from(["indexer", "untrack", "app", "--yes"])
        .expect("`--yes` should be accepted after the subcommand");

    assert!(cli.yes);
}
