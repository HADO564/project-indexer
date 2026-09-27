//! `restore` and `purge`, run for real against an in-memory database: the
//! commands that resolve against the bin rather than the live list.

use std::sync::Arc;

use clap::Parser;
use indexer_core::{
    DetectorRunner, GroupService, Project, ProjectRepository, ProjectService, ScanService,
    SqliteRepository,
};
use serde_json::json;

use crate::commands::{self, Corpus, Failure, Outcome};
use crate::confirm::Confirmer;
use crate::context::Context;
use crate::launcher::SystemLauncher;
use crate::{Cli, Invocation};

/// Answers every confirmation the same way, so a test decides consent up
/// front instead of reading stdin.
struct Answer(bool);

impl Confirmer for Answer {
    fn confirm(&self, _prompt: &str) -> anyhow::Result<bool> {
        Ok(self.0)
    }
}

/// A `Context` over an in-memory database, wired as `Context::open` wires the
/// real one, holding one live project `app` and one binned project `old`.
fn context(consent: bool) -> Context {
    let repo = Arc::new(SqliteRepository::in_memory().unwrap());
    repo.save(&project("app", false)).unwrap();
    repo.save(&project("old", true)).unwrap();

    let detectors = Arc::new(DetectorRunner::default());
    let projects = Arc::new(ProjectService::new(
        repo.clone(),
        Arc::new(SystemLauncher),
        detectors.clone(),
        repo.clone(),
    ));
    Context {
        scan: Arc::new(ScanService::new(projects.clone(), detectors)),
        groups: Arc::new(GroupService::new(repo)),
        projects,
        confirmer: Box::new(Answer(consent)),
    }
}

fn project(name: &str, binned: bool) -> Project {
    serde_json::from_value(json!({
        "id": format!("{name}-0000-4000-8000-000000000000"),
        "name": name,
        "directory": format!("/home/me/work/{name}"),
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
        "is_deleted": binned,
    }))
    .unwrap()
}

/// Parses `argv` exactly as the shell would, then runs it.
fn run(ctx: &Context, argv: &[&str]) -> anyhow::Result<Outcome> {
    let cli = Cli::try_parse_from(argv).expect("arguments should parse");
    let Some(Invocation::Command(command)) = cli.invocation else {
        panic!("expected a command");
    };
    commands::run(command, ctx)
}

fn binned_names(ctx: &Context) -> Vec<String> {
    let binned = ctx.projects.list_deleted(Default::default()).unwrap();
    binned.into_iter().map(|p| p.name).collect()
}

#[test]
fn restore_finds_a_binned_project_and_brings_it_back() {
    let ctx = context(true);
    let Outcome::Restored { project } = run(&ctx, &["indexer", "restore", "old"]).unwrap() else {
        panic!("expected `Restored`");
    };
    assert_eq!(project.name, "old");
    assert!(!project.is_deleted);
    assert!(binned_names(&ctx).is_empty());

    // Back in the live list, so the commands that use `find_one` see it again.
    assert!(run(&ctx, &["indexer", "show", "old"]).is_ok());
}

#[test]
fn the_bin_commands_cannot_see_a_live_project() {
    // Resolving against the bin is the point: a live `app` is not a match, and
    // the error says where it looked rather than that nothing matches at all.
    let ctx = context(true);
    for verb in ["restore", "purge"] {
        let err = run(&ctx, &["indexer", verb, "app"]).expect_err("a live project");
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
    assert!(run(&ctx, &["indexer", "show", "app"]).is_ok());
}

#[test]
fn the_live_commands_cannot_see_a_binned_project() {
    let ctx = context(true);
    let err = run(&ctx, &["indexer", "show", "old"]).expect_err("a binned project");
    assert_eq!(err.to_string(), "no project matches \"old\"");
}

#[test]
fn purge_deletes_a_binned_project_once_confirmed() {
    let ctx = context(true);
    let Outcome::Purged { project } = run(&ctx, &["indexer", "purge", "old"]).unwrap() else {
        panic!("expected `Purged`");
    };
    assert_eq!(project.name, "old");
    assert!(binned_names(&ctx).is_empty());
    assert!(run(&ctx, &["indexer", "restore", "old"]).is_err());
}

#[test]
fn purge_answered_no_is_cancelled_and_keeps_the_project() {
    let ctx = context(false);
    assert!(matches!(
        run(&ctx, &["indexer", "purge", "old"]).unwrap(),
        Outcome::Cancelled
    ));
    assert_eq!(binned_names(&ctx), ["old"]);
}

#[test]
fn restore_and_purge_take_a_project_and_the_tracker_flag() {
    for verb in ["restore", "purge"] {
        assert!(Cli::try_parse_from(["indexer", verb, "old", "-t", "git"]).is_ok());
        assert!(Cli::try_parse_from(["indexer", verb]).is_err());
    }
}
