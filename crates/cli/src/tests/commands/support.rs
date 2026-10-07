//! A `Context` over an in-memory database, and stand-ins for the parts that
//! would otherwise need a person at a terminal — shared by the tests that run
//! whole commands.

use std::sync::Arc;

use clap::Parser;
use indexer_core::{
    DetectorRunner, Group, GroupService, Project, ProjectRepository, ProjectService, ScanService,
    SqliteRepository, UpdateProject,
};
use serde_json::json;

use crate::commands::{self, Outcome};
use crate::confirm::Confirmer;
use crate::context::Context;
use crate::editor::{FormKind, ProjectEditor};
use crate::launcher::SystemLauncher;
use crate::{Cli, Invocation};

/// Answers every confirmation the same way, so a test decides consent up
/// front instead of reading stdin.
pub struct Answer(pub bool);

impl Confirmer for Answer {
    fn confirm(&self, _prompt: &str) -> anyhow::Result<bool> {
        Ok(self.0)
    }
}

/// Stands in for the form: whatever the function returns is what the user
/// "typed" — `Some` changes, or `None` for cancelled.
pub struct Scripted(pub fn(&Project) -> Option<UpdateProject>);

impl ProjectEditor for Scripted {
    fn edit(
        &self,
        project: &Project,
        _groups: Vec<Group>,
        _kind: FormKind,
    ) -> anyhow::Result<Option<UpdateProject>> {
        Ok((self.0)(project))
    }
}

/// Stands in for the form and checks which one `edit` asked for, then
/// cancels — so a test sees the kind arrive without a write to look at.
pub struct Expecting(pub FormKind);

impl ProjectEditor for Expecting {
    fn edit(
        &self,
        _project: &Project,
        _groups: Vec<Group>,
        kind: FormKind,
    ) -> anyhow::Result<Option<UpdateProject>> {
        assert_eq!(kind, self.0, "edit asked for the wrong form");
        Ok(None)
    }
}

/// An editor a test does not expect to be asked: a command that has every
/// change it needs on the command line must not open the form.
pub fn never_asked(_: &Project) -> Option<UpdateProject> {
    panic!("the editor was asked, but the command line already said what to change")
}

/// A `Context` over an in-memory database, wired as `Context::open` wires the
/// real one, holding one live project `app` and one binned project `old`.
pub fn context(consent: bool, editor: impl ProjectEditor + 'static) -> Context {
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
        editor: Box::new(editor),
    }
}

pub fn project(name: &str, binned: bool) -> Project {
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
pub fn run(ctx: &Context, argv: &[&str]) -> anyhow::Result<Outcome> {
    let cli = Cli::try_parse_from(argv).expect("arguments should parse");
    let Some(Invocation::Command(command)) = cli.invocation else {
        panic!("expected a command");
    };
    commands::run(command, ctx)
}
