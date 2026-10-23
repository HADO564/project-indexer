//! `.` as a project: the one the working directory is in. `cargo test` runs
//! these from `crates/cli`, the folder the projects below are added at, or
//! around.

use super::support::{context, never_asked, run, Scripted};
use crate::commands::{Corpus, Failure, Outcome};

/// The directory of the project `argv` acted on.
fn directory_of(outcome: Outcome) -> String {
    match outcome {
        Outcome::Project { project, .. } => project.directory,
        other => panic!("expected a project, got {other:?}"),
    }
}

/// `.` canonicalised, as `add` and the lookup both see it.
fn canonical(path: &str) -> String {
    let path = std::fs::canonicalize(path).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn a_dot_is_the_project_in_the_current_folder() {
    let ctx = context(true, Scripted(never_asked));
    run(&ctx, &["dexily", "add", "."]).unwrap();

    let shown = run(&ctx, &["dexily", "show", "."]).unwrap();
    assert_eq!(directory_of(shown), canonical("."));
}

#[test]
fn a_dot_finds_the_project_from_a_folder_inside_it() {
    // `crates`, around `crates/cli`.
    let ctx = context(true, Scripted(never_asked));
    run(&ctx, &["dexily", "add", ".."]).unwrap();

    let shown = run(&ctx, &["dexily", "show", "."]).unwrap();
    assert_eq!(directory_of(shown), canonical(".."));
}

#[test]
fn a_dot_outside_every_project_says_so() {
    // The test projects live under `/home/me/work`, nowhere near here.
    let ctx = context(true, Scripted(never_asked));
    let err = run(&ctx, &["dexily", "show", "."]).expect_err("no project here");
    let failure = err.downcast_ref::<Failure>().expect("a `Failure`");

    assert!(matches!(
        failure,
        Failure::NotFound { query, corpus: Corpus::Live } if query == "."
    ));
    assert!(
        failure.summary().contains("not inside a tracked project"),
        "{}",
        failure.summary()
    );
}

#[test]
fn a_dot_for_the_bin_commands_looks_in_the_bin() {
    let ctx = context(true, Scripted(never_asked));
    let err = run(&ctx, &["dexily", "restore", "."]).expect_err("nothing binned here");
    let failure = err.downcast_ref::<Failure>().expect("a `Failure`");

    assert!(matches!(
        failure,
        Failure::NotFound {
            corpus: Corpus::Bin,
            ..
        }
    ));
    assert!(failure.summary().contains("not inside a binned project"));
}
