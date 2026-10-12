//! The whole observer — gatekeeper, command, evidence, recording — with a real
//! `mkdir` in a temp folder, an in-memory database and stand-in answers.

use std::cell::Cell;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use clap::Parser;
use indexer_core::domain::sorting::SortOptions;
use indexer_core::{GroupService, Project, ProjectService, UpdateProject};

use crate::commands::Failure;
use crate::confirm::{Confirmer, Track};
use crate::observe::{observe, Options};
use crate::tests::commands::support::{context, never_asked, Scripted};
use crate::{Cli, Invocation};

/// What the user "answers" to `track "x"?` — or `None` for nobody there to
/// ask, as under `--yes` or a pipe.
struct Says(Option<Track>);

impl Confirmer for Says {
    fn confirm(&self, _prompt: &str) -> anyhow::Result<bool> {
        panic!("the observer never asks a yes/no question")
    }

    fn track(&self, _prompt: &str) -> anyhow::Result<Option<Track>> {
        Ok(self.0)
    }
}

/// A confirmer for runs that must not ask at all.
struct MustNotAsk;

impl Confirmer for MustNotAsk {
    fn confirm(&self, _prompt: &str) -> anyhow::Result<bool> {
        panic!("asked, but nothing should have been")
    }

    fn track(&self, prompt: &str) -> anyhow::Result<Option<Track>> {
        panic!("asked \"{prompt}\", but flags or --json mean no question")
    }
}

/// One observed run: its temp folder, its database, and what came of it.
struct Run {
    root: tempfile::TempDir,
    projects: Arc<ProjectService>,
    groups: Arc<GroupService>,
    /// Whether the database was opened at all.
    opened: Rc<Cell<bool>>,
}

impl Run {
    fn new() -> Self {
        let ctx = context(true, Scripted(never_asked));
        Run {
            root: tempfile::tempdir().unwrap(),
            projects: ctx.projects.clone(),
            groups: ctx.groups.clone(),
            opened: Rc::new(Cell::new(false)),
        }
    }

    /// Runs `dexily <words>` in the temp folder, parsed as the real command
    /// line is, with `confirmer` answering and `editor` standing in for the
    /// form. Gives back the exit code or error, and dexily's own lines.
    fn go(
        &self,
        words: &[&str],
        confirmer: impl Confirmer + 'static,
        editor: fn(&Project) -> Option<UpdateProject>,
    ) -> (anyhow::Result<i32>, String) {
        self.go_in(self.root.path(), words, confirmer, editor)
    }

    /// [`Run::go`] from another working folder.
    fn go_in(
        &self,
        cwd: &Path,
        words: &[&str],
        confirmer: impl Confirmer + 'static,
        editor: fn(&Project) -> Option<UpdateProject>,
    ) -> (anyhow::Result<i32>, String) {
        let mut argv = vec!["dexily"];
        argv.extend_from_slice(words);
        let cli = Cli::try_parse_from(argv).expect("arguments should parse");
        let Some(Invocation::Observe(command)) = cli.invocation else {
            panic!("expected an observed command");
        };
        let options = Options {
            yes: cli.yes,
            json: cli.json,
            quiet: cli.quiet,
            flags: cli.observe,
        };
        let mut ctx = context(true, Scripted(editor));
        ctx.projects = self.projects.clone();
        ctx.groups = self.groups.clone();
        ctx.confirmer = Box::new(confirmer);
        let opened = self.opened.clone();
        let mut err = Vec::new();
        let result = observe(
            &command,
            cwd,
            options,
            move || {
                opened.set(true);
                Ok(ctx)
            },
            &mut err,
        );
        (result, String::from_utf8(err).unwrap())
    }

    /// The projects tracked under the temp folder, by folder name.
    fn tracked(&self) -> Vec<Project> {
        let root = std::fs::canonicalize(self.root.path()).unwrap();
        let mut projects: Vec<Project> = self
            .projects
            .list(SortOptions::default())
            .unwrap()
            .into_iter()
            .filter(|p| Path::new(&p.directory).starts_with(&root))
            .collect();
        projects.sort_by(|a, b| a.directory.cmp(&b.directory));
        projects
    }

    fn folder(&self, name: &str) -> PathBuf {
        self.root.path().join(name)
    }
}

fn is_usage(result: &anyhow::Result<i32>) -> bool {
    matches!(
        result.as_ref().unwrap_err().downcast_ref::<Failure>(),
        Some(Failure::Usage { .. })
    )
}

// Tracking, and the prompt's three answers.

#[test]
fn a_folder_the_command_made_is_tracked_under_its_real_path() {
    let run = Run::new();
    let (result, said) = run.go(&["mkdir", "thing"], Says(Some(Track::Yes)), never_asked);
    assert_eq!(result.unwrap(), 0);
    let tracked = run.tracked();
    assert_eq!(tracked.len(), 1);
    assert_eq!(tracked[0].name, "thing");
    assert_eq!(
        Path::new(&tracked[0].directory),
        std::fs::canonicalize(run.folder("thing")).unwrap()
    );
    assert_eq!(
        said, "",
        "the user just answered the prompt; no line repeats it"
    );
}

#[test]
fn with_nobody_to_ask_it_tracks_and_says_so() {
    let run = Run::new();
    let (result, said) = run.go(&["mkdir", "thing"], Says(None), never_asked);
    assert_eq!(result.unwrap(), 0);
    assert_eq!(run.tracked().len(), 1);
    assert!(said.starts_with("dexily: tracking \"thing\" at "), "{said}");
}

#[test]
fn answering_no_leaves_an_ordinary_folder() {
    let run = Run::new();
    let (result, _) = run.go(&["mkdir", "thing"], Says(Some(Track::No)), never_asked);
    assert_eq!(result.unwrap(), 0);
    assert!(run.folder("thing").is_dir(), "the command still ran");
    assert!(run.tracked().is_empty());
}

#[test]
fn answering_e_tracks_then_saves_what_the_form_gave() {
    let run = Run::new();
    let form = |_: &Project| {
        Some(UpdateProject {
            description: Some("from the form".into()),
            ..Default::default()
        })
    };
    let (result, _) = run.go(&["mkdir", "thing"], Says(Some(Track::Edit)), form);
    assert_eq!(result.unwrap(), 0);
    assert_eq!(run.tracked()[0].description, "from the form");
}

#[test]
fn escaping_the_form_keeps_the_project() {
    let run = Run::new();
    let (result, _) = run.go(&["mkdir", "thing"], Says(Some(Track::Edit)), |_| None);
    assert_eq!(result.unwrap(), 0);
    assert_eq!(run.tracked().len(), 1);
}

#[test]
fn every_folder_made_is_asked_about_and_tracked() {
    let run = Run::new();
    let (result, _) = run.go(&["mkdir", "a", "b"], Says(Some(Track::Yes)), never_asked);
    assert_eq!(result.unwrap(), 0);
    let names: Vec<String> = run.tracked().into_iter().map(|p| p.name).collect();
    assert_eq!(names, ["a", "b"]);
}

// dexily's own flags.

#[test]
fn flags_set_the_project_up_without_asking() {
    let run = Run::new();
    let work = run
        .groups
        .create("Work".into(), "cyan".into(), "folder".into())
        .unwrap();
    let (result, said) = run.go(
        &[
            "--group",
            "work",
            "--add-tag",
            "rust,web",
            "--notes",
            "demo",
            "--color",
            "Violet",
            "--set",
            "client=Acme",
            "mkdir",
            "thing",
        ],
        MustNotAsk,
        never_asked,
    );
    assert_eq!(result.unwrap(), 0);
    let project = &run.tracked()[0];
    assert_eq!(project.group_id.as_deref(), Some(work.id.as_str()));
    assert_eq!(project.tags, ["Rust", "Web"]);
    assert_eq!(project.notes.as_deref(), Some("demo"));
    assert_eq!(project.color.as_deref(), Some("violet"));
    assert_eq!(
        project.properties.get("client").map(String::as_str),
        Some("Acme")
    );
    assert!(said.starts_with("dexily: tracking \"thing\""), "{said}");
}

#[test]
fn a_name_flag_names_the_project() {
    let run = Run::new();
    let (result, _) = run.go(
        &["--name", "friction", "mkdir", "thing"],
        MustNotAsk,
        never_asked,
    );
    assert_eq!(result.unwrap(), 0);
    assert_eq!(run.tracked()[0].name, "friction");
}

#[test]
fn a_flag_that_does_not_check_out_stops_the_command_before_it_runs() {
    for flags in [
        &["--group", "Wrok"][..],
        &["--color", "red"],
        &["--icon", "rockt"],
        &["--name", "app"], // the database's own project
        &["--set", "bad:name=x"],
    ] {
        let run = Run::new();
        let mut words = flags.to_vec();
        words.extend(["mkdir", "thing"]);
        let (result, _) = run.go(&words, MustNotAsk, never_asked);
        assert!(is_usage(&result), "{flags:?}: {result:?}");
        assert!(
            !run.folder("thing").exists(),
            "{flags:?}: mkdir must not have run"
        );
        assert!(run.tracked().is_empty(), "{flags:?}");
    }
}

#[test]
fn one_name_cannot_name_two_folders() {
    let run = Run::new();
    let (result, _) = run.go(&["--name", "x", "mkdir", "a", "b"], MustNotAsk, never_asked);
    assert!(is_usage(&result));
    assert!(!run.folder("a").exists());
}

#[test]
fn the_database_is_opened_before_the_command_only_for_a_flag_that_needs_it() {
    // A colour is checked without it; the command fails; nothing to record:
    // the database is never touched at all.
    let run = Run::new();
    std::fs::create_dir(run.folder("thing")).unwrap();
    let (result, _) = run.go(
        &["--color", "cyan", "mkdir", "thing"],
        MustNotAsk,
        never_asked,
    );
    assert_ne!(result.unwrap(), 0);
    assert!(!run.opened.get());
}

#[test]
fn a_property_set_twice_in_another_case_is_one_property() {
    let run = Run::new();
    let (result, _) = run.go(
        &["--set", "a=1", "--set", "A=2", "mkdir", "thing"],
        MustNotAsk,
        never_asked,
    );
    assert_eq!(result.unwrap(), 0);
    let properties = &run.tracked()[0].properties;
    assert_eq!(properties.len(), 1, "{properties:?}");
    assert_eq!(properties.get("A").map(String::as_str), Some("2"));
}

// The gatekeeper, and commands that fail.

#[test]
fn a_command_without_a_recognizer_never_runs() {
    let run = Run::new();
    let words: Vec<OsString> = ["rm", "-rf", "thing"].iter().map(OsString::from).collect();
    std::fs::create_dir(run.folder("thing")).unwrap();
    let mut err = Vec::new();
    let result = observe(
        &words,
        run.root.path(),
        Options::default(),
        || panic!("the database is not opened for a refused command"),
        &mut err,
    );
    assert!(is_usage(&result));
    assert_eq!(
        result.unwrap_err().to_string(),
        "`rm` isn't a command dexily observes (it knows: mkdir, git init, git clone). Run it directly."
    );
    assert!(run.folder("thing").is_dir(), "rm never ran");
}

#[test]
fn a_failed_command_keeps_its_exit_code_and_tracks_nothing() {
    let run = Run::new();
    std::fs::create_dir(run.folder("thing")).unwrap();
    let (result, said) = run.go(&["mkdir", "thing"], MustNotAsk, never_asked);
    let code = result.unwrap();
    assert_ne!(code, 0, "mkdir refuses a folder that exists");
    assert!(run.tracked().is_empty());
    assert_eq!(
        said,
        format!("dexily: mkdir failed (exit {code}), nothing tracked\n")
    );
    assert!(!run.opened.get(), "nothing to record, so no database");
}

// Recording that fails, and the other outputs.

#[test]
fn a_recording_failure_keeps_exit_0_and_says_how_to_retry() {
    let run = Run::new();
    let words: Vec<OsString> = ["mkdir", "thing"].iter().map(OsString::from).collect();
    let mut err = Vec::new();
    let quiet = Options {
        quiet: true,
        ..Options::default()
    };
    let result = observe(
        &words,
        run.root.path(),
        quiet,
        || anyhow::bail!("database is locked"),
        &mut err,
    );
    assert_eq!(result.unwrap(), 0, "the command succeeded; its code stands");
    let said = String::from_utf8(err).unwrap();
    // Shown even under --quiet: quiet hides success, never a problem.
    assert!(
        said.starts_with("dexily: could not track \"thing\": database is locked\n"),
        "{said}"
    );
    assert!(
        said.contains("run `dexily add ") && said.contains("to try again"),
        "{said}"
    );
}

#[test]
fn quiet_hides_the_tracking_line() {
    let run = Run::new();
    let (result, said) = run.go(&["--quiet", "mkdir", "thing"], Says(None), never_asked);
    assert_eq!(result.unwrap(), 0);
    assert_eq!(run.tracked().len(), 1);
    assert_eq!(said, "");
}

#[test]
fn json_is_one_last_line_with_the_exit_code_and_the_projects_and_no_questions() {
    let run = Run::new();
    let (result, said) = run.go(&["--json", "mkdir", "thing"], MustNotAsk, never_asked);
    assert_eq!(result.unwrap(), 0);
    assert_eq!(said.lines().count(), 1, "one line, no prose: {said}");
    let doc: serde_json::Value = serde_json::from_str(&said).unwrap();
    assert_eq!(doc["schema"], 1);
    assert_eq!(doc["data"]["exit_code"], 0);
    assert_eq!(doc["data"]["tracked"][0]["name"], "thing");
    assert_eq!(doc["data"]["failed"], serde_json::json!([]));
}

#[test]
fn json_reports_a_failed_command_too() {
    let run = Run::new();
    std::fs::create_dir(run.folder("thing")).unwrap();
    let (result, said) = run.go(&["--json", "mkdir", "thing"], MustNotAsk, never_asked);
    let code = result.unwrap();
    let doc: serde_json::Value = serde_json::from_str(said.trim()).unwrap();
    assert_eq!(doc["data"]["exit_code"], code);
    assert_eq!(doc["data"]["tracked"], serde_json::json!([]));
}

#[test]
fn details_that_cannot_be_saved_leave_the_project_tracked_and_say_so() {
    // The form hands back a name core refuses. The project was tracked on
    // the way in: it stays, the exit code stands, and the message does not
    // claim it could not be tracked.
    let run = Run::new();
    let form = |_: &Project| {
        Some(UpdateProject {
            name: Some("  ".into()),
            ..Default::default()
        })
    };
    let (result, said) = run.go(&["mkdir", "thing"], Says(Some(Track::Edit)), form);
    assert_eq!(result.unwrap(), 0);
    assert_eq!(run.tracked().len(), 1);
    assert!(
        said.starts_with("dexily: tracked \"thing\", but could not save its details: "),
        "{said}"
    );
    assert!(
        said.contains("run `dexily edit \"thing\"` to set them"),
        "{said}"
    );
    assert!(!said.contains("could not track"), "{said}");
}

// The project flags belong to an observed command only.

#[test]
fn project_flags_parse_before_an_observed_command_and_leave_its_own_alone() {
    let cli =
        Cli::try_parse_from(["dexily", "--add-tag", "rust", "mkdir", "-p", "--name"]).unwrap();
    assert_eq!(cli.observe.add_tag, ["rust"]);
    assert_eq!(cli.observe.name, None, "--name after mkdir is mkdir's");
    let Some(Invocation::Observe(command)) = cli.invocation else {
        panic!("expected an observed command");
    };
    assert_eq!(command, ["mkdir", "-p", "--name"]);
}

// Git, with the real `git`.

fn has_git_tracker(project: &Project) -> bool {
    project.trackers.iter().any(|tracker| tracker.is("git"))
}

#[test]
fn git_init_tracks_the_folder_with_its_git_tracker() {
    let run = Run::new();
    let (result, _) = run.go(
        &["git", "init", "-q", "repo"],
        Says(Some(Track::Yes)),
        never_asked,
    );
    assert_eq!(result.unwrap(), 0);
    let tracked = run.tracked();
    assert_eq!(tracked.len(), 1);
    assert_eq!(tracked[0].name, "repo");
    assert!(has_git_tracker(&tracked[0]), "{:?}", tracked[0].trackers);
}

#[test]
fn git_init_inside_a_tracked_folder_detects_its_trackers_again_without_asking() {
    let run = Run::new();
    run.go(&["mkdir", "thing"], Says(Some(Track::Yes)), never_asked)
        .0
        .unwrap();
    assert!(!has_git_tracker(&run.tracked()[0]), "a bare folder so far");

    let (result, said) = run.go_in(
        &run.folder("thing"),
        &["git", "init", "-q"],
        MustNotAsk,
        never_asked,
    );
    assert_eq!(result.unwrap(), 0);
    let tracked = run.tracked();
    assert_eq!(tracked.len(), 1, "the same project, not a second one");
    assert!(has_git_tracker(&tracked[0]));
    assert_eq!(
        said,
        "dexily: \"thing\" was already tracked; detected its trackers again\n"
    );
}

#[test]
fn flags_on_an_already_tracked_folder_add_to_its_tags() {
    let run = Run::new();
    run.go(
        &["--add-tag", "one", "mkdir", "thing"],
        MustNotAsk,
        never_asked,
    )
    .0
    .unwrap();
    let (result, _) = run.go_in(
        &run.folder("thing"),
        &["--add-tag", "two", "git", "init", "-q"],
        MustNotAsk,
        never_asked,
    );
    assert_eq!(result.unwrap(), 0);
    assert_eq!(run.tracked()[0].tags, ["One", "Two"]);
}

#[test]
fn git_clone_tracks_the_folder_it_made() {
    let run = Run::new();
    // A local repository to clone, so the test needs no network.
    run.go(
        &["git", "init", "-q", "source"],
        Says(Some(Track::No)),
        never_asked,
    )
    .0
    .unwrap();
    let (result, _) = run.go(
        &["git", "clone", "-q", "source", "copy"],
        Says(Some(Track::Yes)),
        never_asked,
    );
    assert_eq!(result.unwrap(), 0);
    let tracked = run.tracked();
    assert_eq!(tracked.len(), 1, "source was answered no");
    assert_eq!(
        Path::new(&tracked[0].directory),
        std::fs::canonicalize(run.folder("copy")).unwrap()
    );
    // Named after the repository it was cloned from, as core names any git
    // project with a remote — the app's Add does the same.
    assert_eq!(tracked[0].name, "source");
    assert!(has_git_tracker(&tracked[0]));
}

#[test]
fn a_git_clone_that_fails_tracks_nothing() {
    let run = Run::new();
    let (result, said) = run.go(
        &["git", "clone", "-q", "no-such-repository", "copy"],
        MustNotAsk,
        never_asked,
    );
    let code = result.unwrap();
    assert_ne!(code, 0);
    assert!(run.tracked().is_empty());
    assert!(
        said.ends_with(&format!(
            "dexily: git clone failed (exit {code}), nothing tracked\n"
        )),
        "{said}"
    );
}

#[test]
fn git_commands_that_make_no_project_never_run() {
    let run = Run::new();
    for words in [&["git", "push"][..], &["git", "status"]] {
        let command: Vec<OsString> = words.iter().map(OsString::from).collect();
        let mut err = Vec::new();
        let result = observe(
            &command,
            run.root.path(),
            Options::default(),
            || panic!("the database is not opened for a refused command"),
            &mut err,
        );
        assert!(is_usage(&result), "{words:?}");
        assert!(
            result.unwrap_err().to_string().starts_with(&format!(
                "`{}` isn't a command dexily observes",
                words.join(" ")
            )),
            "{words:?}"
        );
    }
}

#[test]
fn a_git_option_that_runs_another_program_is_refused_before_git_starts() {
    let run = Run::new();
    let (result, _) = run.go(
        &[
            "git",
            "clone",
            "--upload-pack=touch pwned",
            "source",
            "copy",
        ],
        MustNotAsk,
        never_asked,
    );
    assert!(is_usage(&result));
    assert!(!run.folder("copy").exists());
    assert!(!run.folder("pwned").exists());
}

#[test]
fn trackers_that_cannot_be_detected_again_leave_the_project_tracked_and_its_flags_applied() {
    // Already tracked; a `.uproject` that is not one makes re-detection fail
    // outright. The folder is tracked all the same — not "could not track",
    // no `dexily add` to retry — and the flags still go in.
    let run = Run::new();
    run.go(
        &["--add-tag", "one", "mkdir", "thing"],
        MustNotAsk,
        never_asked,
    )
    .0
    .unwrap();
    std::fs::write(run.folder("thing").join("Broken.uproject"), "not json").unwrap();

    let (result, said) = run.go_in(
        &run.folder("thing"),
        &["--add-tag", "two", "git", "init", "-q"],
        MustNotAsk,
        never_asked,
    );
    assert_eq!(result.unwrap(), 0);
    let tracked = run.tracked();
    assert_eq!(tracked.len(), 1);
    assert_eq!(
        tracked[0].tags,
        ["One", "Two"],
        "the flags were still applied"
    );
    assert!(
        said.contains(
            "dexily: \"thing\" is tracked, but its trackers could not be detected again: "
        ),
        "{said}"
    );
    assert!(
        !said.contains("could not track") && !said.contains("dexily add"),
        "{said}"
    );
}
