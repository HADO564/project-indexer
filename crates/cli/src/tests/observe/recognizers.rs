//! Phase 1 on argv alone: which commands dexily knows, and the folders each
//! should leave behind. No processes, no disk.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::observe::recognizers::{
    confirm, expect, expect_on, label, Expectation, Expected, Proof, Refusal, KNOWN,
};

fn cwd() -> &'static Path {
    Path::new("/home/me/code")
}

fn argv(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

/// The folders `words` should leave a project in, or `None` when dexily will
/// not run it at all — by the Unix rules, whichever system runs the tests.
fn expected(words: &[&str]) -> Option<Vec<PathBuf>> {
    folders_of(expect_on(&argv(words), cwd(), false))
}

/// The same, by Windows' rules: cmd's `mkdir`.
fn expected_on_windows(words: &[&str]) -> Option<Vec<PathBuf>> {
    folders_of(expect_on(&argv(words), cwd(), true))
}

fn folders_of(result: Result<Expectation, Refusal>) -> Option<Vec<PathBuf>> {
    result
        .ok()
        .map(|Expectation { folders }| folders.into_iter().map(|e| e.folder).collect())
}

fn under_cwd(names: &[&str]) -> Option<Vec<PathBuf>> {
    Some(names.iter().map(|name| cwd().join(name)).collect())
}

#[test]
fn mkdir_expects_the_folder_it_names_under_the_working_folder() {
    assert_eq!(expected(&["mkdir", "thing"]), under_cwd(&["thing"]));
}

#[test]
fn mkdir_expects_every_folder_it_names() {
    assert_eq!(
        expected(&["mkdir", "a", "b", "c"]),
        under_cwd(&["a", "b", "c"])
    );
}

#[test]
fn with_p_only_the_named_path_is_meant_not_its_parents() {
    assert_eq!(
        expected(&["mkdir", "-p", "work/clients/acme"]),
        under_cwd(&["work/clients/acme"])
    );
}

#[test]
fn a_mode_is_never_taken_for_a_folder() {
    for argv in [
        &["mkdir", "-m", "755", "thing"][..],
        &["mkdir", "-pm", "755", "thing"],
        &["mkdir", "-m755", "thing"],
        &["mkdir", "--mode", "755", "thing"],
        &["mkdir", "--mode=755", "thing"],
    ] {
        assert_eq!(expected(argv), under_cwd(&["thing"]), "{argv:?}");
    }
}

#[test]
fn other_options_are_skipped() {
    assert_eq!(
        expected(&["mkdir", "-v", "--parents", "thing"]),
        under_cwd(&["thing"])
    );
}

#[test]
fn after_a_double_dash_a_name_may_start_with_a_dash() {
    assert_eq!(expected(&["mkdir", "--", "-odd"]), under_cwd(&["-odd"]));
}

#[test]
fn an_absolute_path_stays_as_it_is() {
    assert_eq!(
        expected(&["mkdir", "/srv/projects/new"]),
        Some(vec![PathBuf::from("/srv/projects/new")])
    );
}

#[test]
fn mkdir_naming_nothing_is_still_known_and_expects_no_folders() {
    // It runs — mkdir prints its own usage, `--help` works — and there is
    // nothing to track. `None` would call mkdir an unknown command.
    for argv in [&["mkdir"][..], &["mkdir", "-p"], &["mkdir", "--help"]] {
        assert_eq!(expected(argv), Some(Vec::new()), "{argv:?}");
    }
}

#[test]
fn a_command_without_a_recognizer_is_not_known() {
    for argv in [
        &["ls", "-la"][..],
        &["rm", "-rf", "thing"],
        &["git", "push"],
    ] {
        assert_eq!(expected(argv), None, "{argv:?}");
    }
}

#[test]
fn only_the_bare_name_is_known_never_a_path_to_another_program() {
    // `/tmp/x/mkdir` could be anything that happens to share the name.
    assert_eq!(expected(&["/tmp/x/mkdir", "thing"]), None);
    assert_eq!(expected(&["./mkdir", "thing"]), None);
}

#[test]
fn nothing_at_all_is_not_known() {
    assert_eq!(expected(&[]), None);
}

// Phase 3: the evidence. Real folders in a temp dir, made-up exit codes.

/// An expectation of `names` under `root`, with each folder's "was it there
/// before" taken now, as the observer takes it before running the command.
fn before(root: &Path, names: &[&str]) -> (Expectation, Vec<bool>) {
    before_with(root, names, Proof::NewFolder)
}

fn before_with(root: &Path, names: &[&str], proof: Proof) -> (Expectation, Vec<bool>) {
    let folders = names
        .iter()
        .map(|name| Expected {
            folder: root.join(name),
            proof,
        })
        .collect();
    let expectation = Expectation { folders };
    let held = expectation.before();
    (expectation, held)
}

#[test]
fn a_folder_made_by_a_command_that_succeeded_is_confirmed() {
    let root = tempfile::tempdir().unwrap();
    let (expectation, existed) = before(root.path(), &["thing"]);
    std::fs::create_dir(root.path().join("thing")).unwrap();
    assert_eq!(
        confirm(&expectation, &existed, 0),
        vec![root.path().join("thing")]
    );
}

#[test]
fn a_failed_command_confirms_nothing_whatever_it_left_behind() {
    let root = tempfile::tempdir().unwrap();
    let (expectation, existed) = before(root.path(), &["thing"]);
    std::fs::create_dir(root.path().join("thing")).unwrap();
    assert_eq!(confirm(&expectation, &existed, 1), Vec::<PathBuf>::new());
}

#[test]
fn a_folder_that_was_already_there_is_not_this_command_s() {
    // `mkdir -p old new`: exit 0, both exist, only `new` was made.
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("old")).unwrap();
    let (expectation, existed) = before(root.path(), &["old", "new"]);
    std::fs::create_dir(root.path().join("new")).unwrap();
    assert_eq!(
        confirm(&expectation, &existed, 0),
        vec![root.path().join("new")]
    );
}

#[test]
fn a_folder_expected_but_never_made_is_not_confirmed() {
    // Exit 0 without the folder: the prediction was wrong, so trust the disk.
    let root = tempfile::tempdir().unwrap();
    let (expectation, existed) = before(root.path(), &["thing"]);
    assert_eq!(confirm(&expectation, &existed, 0), Vec::<PathBuf>::new());
}

#[test]
fn a_file_where_the_folder_was_expected_is_not_a_project() {
    let root = tempfile::tempdir().unwrap();
    let (expectation, existed) = before(root.path(), &["thing"]);
    std::fs::write(root.path().join("thing"), "").unwrap();
    assert_eq!(confirm(&expectation, &existed, 0), Vec::<PathBuf>::new());
}

#[test]
fn a_folder_named_twice_is_expected_once() {
    assert_eq!(
        expected(&["mkdir", "-p", "dup", "dup"]),
        under_cwd(&["dup"])
    );
}

// Git: only `init` and `clone` make a project, and a `.git` proves it.

fn git_expects(words: &[&str]) -> Result<Vec<Expected>, Refusal> {
    expect(&argv(words), cwd()).map(|e| e.folders)
}

fn repo_at(path: &str) -> Result<Vec<Expected>, Refusal> {
    Ok(vec![Expected {
        folder: cwd().join(path),
        proof: Proof::Holds(".git"),
    }])
}

#[test]
fn git_init_is_the_working_folder_or_the_one_it_names() {
    assert_eq!(git_expects(&["git", "init"]), repo_at(""));
    assert_eq!(git_expects(&["git", "init", "app"]), repo_at("app"));
    assert_eq!(
        git_expects(&["git", "init", "-q", "--", "app"]),
        repo_at("app")
    );
}

#[test]
fn git_init_s_options_with_a_value_are_not_the_folder() {
    for words in [
        &["git", "init", "-b", "main", "app"][..],
        &["git", "init", "--initial-branch", "main", "app"],
        &["git", "init", "--initial-branch=main", "app"],
        &[
            "git",
            "init",
            "--template",
            "tpl",
            "--object-format",
            "sha256",
            "app",
        ],
    ] {
        assert_eq!(git_expects(words), repo_at("app"), "{words:?}");
    }
}

#[test]
fn dash_c_moves_the_folder_git_works_in() {
    assert_eq!(git_expects(&["git", "-C", "sub", "init"]), repo_at("sub"));
    assert_eq!(
        git_expects(&["git", "-C", "a", "-C", "b", "init", "x"]),
        repo_at("a/b/x")
    );
    assert_eq!(
        git_expects(&["git", "--git-dir", "elsewhere", "-C", "sub", "init"]),
        repo_at("sub")
    );
}

#[test]
fn git_clone_is_the_folder_named_after_the_repository() {
    for repository in [
        "https://github.com/you/thing.git",
        "https://github.com/you/thing",
        "https://github.com/you/thing/",
        "git@github.com:you/thing.git",
        "host:thing",
        "../elsewhere/thing",
        "/srv/git/thing/.git",
        "thing.bundle",
    ] {
        assert_eq!(
            git_expects(&["git", "clone", repository]),
            repo_at("thing"),
            "{repository}"
        );
    }
}

#[test]
fn git_clone_into_a_named_folder_is_that_folder() {
    assert_eq!(
        git_expects(&["git", "clone", "https://host/you/thing.git", "mine"]),
        repo_at("mine")
    );
    assert_eq!(
        git_expects(&[
            "git",
            "clone",
            "-b",
            "dev",
            "--depth",
            "1",
            "--origin=up",
            "-q",
            "https://host/you/thing.git",
            "mine",
        ]),
        repo_at("mine")
    );
}

#[test]
fn a_bare_repository_runs_but_is_not_a_project() {
    for words in [
        &["git", "init", "--bare", "x.git"][..],
        &["git", "clone", "--bare", "https://host/you/thing.git"],
        &["git", "clone", "--mirror", "https://host/you/thing.git"],
        &["git", "clone"], // nothing named: git prints its usage
    ] {
        assert_eq!(git_expects(words), Ok(Vec::new()), "{words:?}");
    }
}

#[test]
fn any_other_git_command_is_not_known() {
    for words in [
        &["git", "push"][..],
        &["git", "status"],
        &["git", "commit", "-m", "init"],
        &["git", "--version"],
        &["git"],
        &["git", "-C", "sub", "pull"],
        // Unknown first, whatever options it carries.
        &["git", "-c", "alias.x=!sh", "push"],
    ] {
        assert_eq!(git_expects(words), Err(Refusal::Unknown), "{words:?}");
    }
}

#[test]
fn an_option_that_makes_git_run_another_program_is_refused() {
    for words in [
        &["git", "-c", "core.sshCommand=evil", "clone", "host:thing"][..],
        &[
            "git",
            "--config-env=core.sshCommand=EVIL",
            "clone",
            "host:thing",
        ],
        // Both spellings: git takes the value as the next argument too.
        &["git", "--config-env", "core.sshCommand=EVIL", "init", "x"],
        &["git", "--exec-path=/tmp/evil", "init"],
        &["git", "clone", "-u", "evil", "host:thing"],
        &["git", "clone", "--upload-pack", "evil", "host:thing"],
        &["git", "clone", "--upload-pack=evil", "host:thing"],
        &["git", "clone", "-c", "core.fsmonitor=evil", "host:thing"],
        &[
            "git",
            "clone",
            "--config",
            "core.fsmonitor=evil",
            "host:thing",
        ],
        &["git", "clone", "--template=/tmp/hooks", "host:thing"],
        &["git", "clone", "ext::sh -c evil", "x"],
        &["git", "clone", "fd::7", "x"],
        // A bare clone tracks nothing, and is refused all the same: every
        // refusal is decided before anything about the folder.
        &["git", "clone", "--bare", "ext::true", "x"],
        &["git", "clone", "--mirror", "fd::7", "x"],
        &["git", "clone", "--bare", "--upload-pack=evil", "host:thing"],
    ] {
        assert!(
            matches!(git_expects(words), Err(Refusal::Unsafe(_))),
            "{words:?}: {:?}",
            git_expects(words)
        );
    }
}

#[test]
fn a_refusal_names_the_option_and_says_to_run_git_directly() {
    let Err(Refusal::Unsafe(why)) = git_expects(&["git", "clone", "--upload-pack=evil", "x"])
    else {
        panic!("refused");
    };
    assert_eq!(
        why,
        "`--upload-pack` makes git run another program, so it is not passed on. Run git directly."
    );
}

#[test]
fn a_command_is_labelled_as_a_person_would_name_it() {
    assert_eq!(label(&argv(&["mkdir", "-p", "x"])), "mkdir");
    assert_eq!(label(&argv(&["git", "clone", "u"])), "git clone");
    assert_eq!(label(&argv(&["git", "-C", "sub", "push"])), "git push");
    assert_eq!(label(&argv(&["git"])), "git");
}

// Git's proof: the folder holds a `.git` now, whether or not it did before.

#[test]
fn a_git_folder_is_confirmed_when_it_holds_dot_git_after() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("app")).unwrap();
    let (expectation, held) = before_with(root.path(), &["app"], Proof::Holds(".git"));
    assert_eq!(
        confirm(&expectation, &held, 0),
        Vec::<PathBuf>::new(),
        "not yet"
    );
    std::fs::create_dir(root.path().join("app/.git")).unwrap();
    assert_eq!(
        confirm(&expectation, &held, 0),
        vec![root.path().join("app")]
    );
    assert_eq!(
        confirm(&expectation, &held, 128),
        Vec::<PathBuf>::new(),
        "git failed"
    );
}

#[test]
fn git_init_in_a_folder_that_was_already_a_repository_is_still_confirmed() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("app/.git")).unwrap();
    let (expectation, held) = before_with(root.path(), &["app"], Proof::Holds(".git"));
    assert_eq!(held, [true]);
    assert_eq!(
        confirm(&expectation, &held, 0),
        vec![root.path().join("app")]
    );
}

// Windows: `mkdir` is cmd's, with rules of its own.

#[test]
fn on_windows_mkdir_and_md_are_known_whatever_their_case() {
    for program in ["mkdir", "MKDIR", "md", "Md"] {
        assert_eq!(
            expected_on_windows(&[program, "thing"]),
            under_cwd(&["thing"]),
            "{program}"
        );
    }
}

#[test]
fn off_windows_md_is_not_a_command_and_case_matters() {
    assert_eq!(expected(&["md", "thing"]), None);
    assert_eq!(expected(&["MKDIR", "thing"]), None);
}

#[test]
fn cmd_s_mkdir_has_no_options_so_a_dash_starts_a_folder_name() {
    // No `-p` or `-m` there: cmd makes parents by itself, and would make
    // folders called `-p` and `755`.
    assert_eq!(
        expected_on_windows(&["mkdir", "-p", "thing"]),
        under_cwd(&["-p", "thing"])
    );
}

#[test]
fn cmd_s_switches_start_with_a_slash_and_are_not_folders() {
    assert_eq!(expected_on_windows(&["mkdir", "/?"]), Some(Vec::new()));
    assert_eq!(
        expected_on_windows(&["mkdir", "/?", "thing"]),
        under_cwd(&["thing"])
    );
}

#[test]
fn the_system_this_was_built_for_is_the_one_expect_uses() {
    let here = expect(&argv(&["mkdir", "-p", "thing"]), cwd());
    assert_eq!(
        here,
        expect_on(&argv(&["mkdir", "-p", "thing"]), cwd(), cfg!(windows))
    );
}

#[test]
fn a_bare_exec_path_only_prints_so_it_is_not_refused() {
    // Not a project command either way; the point is which refusal it gets.
    assert_eq!(git_expects(&["git", "--exec-path"]), Err(Refusal::Unknown));
    assert_eq!(git_expects(&["git", "--exec-path", "init"]), repo_at(""));
}

#[test]
fn every_command_the_refusal_names_is_one_dexily_runs() {
    // `KNOWN` is the list the refusal message prints. A name there that the
    // recognizers do not take would make the message a lie; this pins it on
    // both systems.
    for known in KNOWN {
        let words: Vec<&str> = known.split(' ').collect();
        for windows in [false, true] {
            assert!(
                expect_on(&argv(&words), cwd(), windows).is_ok(),
                "`{known}` (windows: {windows})"
            );
        }
    }
}

#[test]
fn on_windows_git_may_be_named_with_its_exe_and_in_any_case() {
    for program in ["git.exe", "GIT", "Git.EXE"] {
        assert_eq!(
            folders_of(expect_on(&argv(&[program, "init", "app"]), cwd(), true)),
            under_cwd(&["app"]),
            "{program}"
        );
    }
    assert_eq!(
        expected(&["git.exe", "init", "app"]),
        None,
        "not off Windows"
    );
}
