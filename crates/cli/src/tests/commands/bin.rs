//! `restore` and `purge`, run for real against an in-memory database: the
//! commands that resolve against the bin rather than the live list.

use clap::Parser;

use super::support::{context, never_asked, run, Scripted};
use crate::commands::{Corpus, Failure, Outcome};
use crate::context::Context;
use crate::Cli;

fn binned_names(ctx: &Context) -> Vec<String> {
    let binned = ctx.projects.list_deleted(Default::default()).unwrap();
    binned.into_iter().map(|p| p.name).collect()
}

#[test]
fn restore_finds_a_binned_project_and_brings_it_back() {
    let ctx = context(true, Scripted(never_asked));
    let Outcome::Restored { project } = run(&ctx, &["dexily", "restore", "old"]).unwrap() else {
        panic!("expected `Restored`");
    };
    assert_eq!(project.name, "old");
    assert!(!project.is_deleted);
    assert!(binned_names(&ctx).is_empty());

    // Back in the live list, so the commands that use `find_one` see it again.
    assert!(run(&ctx, &["dexily", "show", "old"]).is_ok());
}

#[test]
fn the_bin_commands_cannot_see_a_live_project() {
    // Resolving against the bin is the point: a live `app` is not a match, and
    // the error says where it looked rather than that nothing matches at all.
    let ctx = context(true, Scripted(never_asked));
    for verb in ["restore", "purge"] {
        let err = run(&ctx, &["dexily", verb, "app"]).expect_err("a live project");
        let failure = err.downcast_ref::<Failure>().expect("a `Failure`");
        assert!(matches!(
            failure,
            Failure::NotFound {
                corpus: Corpus::Bin,
                ..
            }
        ));
        assert_eq!(failure.to_string(), "no project in the bin matches \"app\"");
    }
    assert!(run(&ctx, &["dexily", "show", "app"]).is_ok());
}

#[test]
fn the_live_commands_cannot_see_a_binned_project() {
    let ctx = context(true, Scripted(never_asked));
    let err = run(&ctx, &["dexily", "show", "old"]).expect_err("a binned project");
    assert_eq!(err.to_string(), "no project matches \"old\"");
}

#[test]
fn purge_deletes_a_binned_project_once_confirmed() {
    let ctx = context(true, Scripted(never_asked));
    let Outcome::Purged { project } = run(&ctx, &["dexily", "purge", "old"]).unwrap() else {
        panic!("expected `Purged`");
    };
    assert_eq!(project.name, "old");
    assert!(binned_names(&ctx).is_empty());
    assert!(run(&ctx, &["dexily", "restore", "old"]).is_err());
}

#[test]
fn purge_answered_no_is_cancelled_and_keeps_the_project() {
    let ctx = context(false, Scripted(never_asked));
    assert!(matches!(
        run(&ctx, &["dexily", "purge", "old"]).unwrap(),
        Outcome::Cancelled
    ));
    assert_eq!(binned_names(&ctx), ["old"]);
}

#[test]
fn restore_and_purge_take_a_project_and_the_tracker_flag() {
    for verb in ["restore", "purge"] {
        assert!(Cli::try_parse_from(["dexily", verb, "old", "-t", "git"]).is_ok());
        assert!(Cli::try_parse_from(["dexily", verb]).is_err());
    }
}
