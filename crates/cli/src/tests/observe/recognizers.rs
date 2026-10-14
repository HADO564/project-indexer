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
            "--separate-git-dir",
            "elsewhere",
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
        git_expects(&["git", "--no-pager", "-C", "sub", "init"]),
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
            matches!(git_expects(words), Err(Refusal::Refused(_, _))),
            "{words:?}: {:?}",
            git_expects(words)
        );
    }
}

#[test]
fn a_refusal_names_the_option_and_says_to_run_git_directly() {
    let Err(Refusal::Refused(why, _)) = git_expects(&["git", "clone", "--upload-pack=evil", "x"])
    else {
        panic!("refused");
    };
    assert_eq!(
        why,
        "`--upload-pack` makes git run another program, so it is not passed on. Run git directly."
    );
}

#[test]
fn a_recognized_git_line_refused_for_an_option_still_knows_the_repo_it_would_make() {
    // Refused for an option, yet dexily can place the repo all the same: the
    // expectation rides on the refusal, for a terminal override to track. The
    // refused word's own value is stepped over, so `repo` is the folder, not
    // the template path.
    for (words, folder) in [
        (&["git", "init", "--template", "hooks", "repo"][..], "repo"),
        (&["git", "init", "--template=hooks"][..], ""),
        (
            &[
                "git",
                "-C",
                "sub",
                "clone",
                "--upload-pack",
                "evil",
                "host:thing",
            ][..],
            "sub/thing",
        ),
    ] {
        let Err(Refusal::Refused(_, Some(expectation))) = expect(&argv(words), cwd()) else {
            panic!("{words:?} should carry its expectation");
        };
        assert_eq!(
            expectation.folders,
            vec![Expected {
                folder: cwd().join(folder),
                proof: Proof::Holds(".git"),
            }],
            "{words:?}"
        );
    }
}

#[test]
fn a_refused_git_line_whose_folder_cannot_be_placed_carries_no_expectation() {
    // The refused word is an abbreviation or a run of short options: it might
    // eat the next word, so where the repo lands is a guess. Rather than track
    // the wrong folder, dexily offers nothing, and an override tracks nothing.
    for words in [
        &["git", "clone", "--upl", "x", "host:thing"][..],
        &["git", "clone", "-qu", "evil", "host:thing"],
        &["git", "--shallow-file", "init", "app"],
    ] {
        let Err(Refusal::Refused(_, overridden)) = expect(&argv(words), cwd()) else {
            panic!("{words:?} should be refused");
        };
        assert_eq!(overridden, None, "{words:?}");
    }
}

#[test]
fn a_command_is_labelled_as_a_person_would_name_it() {
    assert_eq!(label(&argv(&["mkdir", "-p", "x"])), "mkdir");
    assert_eq!(label(&argv(&["git", "clone", "u"])), "git clone");
    assert_eq!(label(&argv(&["git", "-C", "sub", "push"])), "git push");
    assert_eq!(label(&argv(&["git"])), "git");
    // cargo and go name the subcommand, whether dexily runs the line or not.
    assert_eq!(label(&argv(&["cargo", "new", "app"])), "cargo new");
    assert_eq!(label(&argv(&["cargo", "init"])), "cargo init");
    assert_eq!(
        label(&argv(&["cargo", "build", "--release"])),
        "cargo build"
    );
    assert_eq!(label(&argv(&["cargo", "--version"])), "cargo");
    assert_eq!(label(&argv(&["cargo"])), "cargo");
    assert_eq!(label(&argv(&["go", "mod", "init", "x"])), "go mod init");
    assert_eq!(label(&argv(&["go", "mod", "tidy"])), "go mod tidy");
    assert_eq!(label(&argv(&["go", "build"])), "go build");
    assert_eq!(label(&argv(&["go"])), "go");
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

// Fail closed: a git line runs only if dexily knows every word of it. Each of
// these once reached git and ran another program, because a list of bad
// spellings cannot hold every spelling git takes.

fn refusal(words: &[&str]) -> String {
    match git_expects(words) {
        Err(Refusal::Refused(why, _)) => why,
        other => panic!("{words:?} should be refused, but: {other:?}"),
    }
}

#[test]
fn a_global_option_dexily_does_not_know_is_refused_whatever_follows() {
    // `--shallow-file` takes the next word as its value, so `init` here is a
    // file name and the real subcommand is whatever comes after it: git would
    // run `git -c alias.x=… x`, or with `status` in its place, `git status`.
    let why = refusal(&[
        "git",
        "--shallow-file",
        "init",
        "-c",
        "alias.x=!touch pwned",
        "x",
    ]);
    assert!(
        why.starts_with("`--shallow-file` is not an option dexily passes on to git:"),
        "{why}"
    );
    refusal(&["git", "--shallow-file", "init", "status"]);
    refusal(&["git", "--attr-source", "init", "status"]);
    // Even one that changes nothing dangerous: where the repository goes is
    // no longer where dexily would look.
    refusal(&["git", "--git-dir", "elsewhere", "init"]);
    refusal(&["git", "--work-tree=elsewhere", "init"]);
}

#[test]
fn an_abbreviated_option_is_refused() {
    // Git takes any unique start of a long option: `--upl` is `--upload-pack`.
    let why = refusal(&[
        "git",
        "clone",
        "--bare",
        "--upl=touch pwned; false",
        "file:///src",
        "c.git",
    ]);
    assert!(
        why.starts_with("`--upl` is not an option dexily passes on to git clone:"),
        "{why}"
    );
    for abbreviated in ["--conf", "--templ", "--dep", "--bran"] {
        refusal(&["git", "clone", abbreviated, "x", "host:thing"]);
    }
    refusal(&["git", "init", "--templ=hooks", "app"]);
}

#[test]
fn short_options_run_together_are_refused() {
    // `-qu` is `-q -u`, and `-u` names a program to run.
    refusal(&[
        "git",
        "clone",
        "--bare",
        "-qu",
        "touch pwned; false",
        "file:///src",
        "c.git",
    ]);
    refusal(&["git", "clone", "-qb", "dev", "host:thing"]);
    refusal(&["git", "clone", "-j4", "host:thing"]);
    refusal(&["git", "init", "-qb", "main", "app"]);
}

#[test]
fn an_option_dexily_does_not_know_is_refused_on_either_subcommand() {
    for words in [
        &["git", "init", "--new-in-some-later-git", "app"][..],
        &["git", "clone", "--new-in-some-later-git", "host:thing"],
        &["git", "init", "--template", "hooks", "app"],
        &["git", "clone", "--bundle-uri", "x", "host:thing"],
        // A flag given a value it does not take.
        &["git", "clone", "--quiet=yes", "host:thing"],
    ] {
        refusal(words);
    }
}

#[test]
fn a_value_is_never_read_as_the_repository_or_the_folder() {
    // With every option's arity known. `--bran dev ../other` once made dexily
    // look for the new project in `../other`, an existing repository.
    refusal(&["git", "clone", "--bran", "dev", "../other"]);
    assert_eq!(
        git_expects(&[
            "git",
            "clone",
            "--branch",
            "dev",
            "--depth",
            "1",
            "host:thing"
        ]),
        repo_at("thing")
    );
    assert_eq!(
        git_expects(&["git", "clone", "--recurse-submodules", "host:thing"]),
        repo_at("thing"),
        "its value is optional, and only after `=`"
    );
    assert_eq!(
        git_expects(&[
            "git",
            "clone",
            "--recurse-submodules=lib",
            "host:thing",
            "mine"
        ]),
        repo_at("mine")
    );
}

#[test]
fn everyday_lines_still_run() {
    for (words, folder) in [
        (&["git", "init"][..], ""),
        (&["git", "init", "-q", "-b", "main", "app"], "app"),
        (
            &["git", "init", "--initial-branch=main", "--shared", "app"],
            "app",
        ),
        (&["git", "init", "--shared=group", "app"], "app"),
        (&["git", "-C", "sub", "init"], "sub"),
        (&["git", "--no-pager", "init", "app"], "app"),
        (&["git", "clone", "https://host/you/thing.git"], "thing"),
        (
            &[
                "git",
                "clone",
                "--depth",
                "1",
                "-b",
                "dev",
                "host:thing",
                "dir",
            ],
            "dir",
        ),
        (
            &[
                "git",
                "clone",
                "--depth=1",
                "--single-branch",
                "--no-tags",
                "host:thing",
            ],
            "thing",
        ),
        (
            &[
                "git",
                "clone",
                "-q",
                "--recurse-submodules",
                "-j",
                "4",
                "host:thing",
            ],
            "thing",
        ),
        (
            &[
                "git",
                "clone",
                "--filter=blob:none",
                "--sparse",
                "host:thing",
            ],
            "thing",
        ),
        (&["git", "clone", "--", "host:thing", "-odd"], "-odd"),
    ] {
        assert_eq!(git_expects(words), repo_at(folder), "{words:?}");
    }
}

#[test]
fn asking_git_about_itself_is_not_a_project_command() {
    // Known words, no subcommand: unknown rather than refused.
    assert_eq!(git_expects(&["git", "--version"]), Err(Refusal::Unknown));
    assert_eq!(git_expects(&["git", "--help"]), Err(Refusal::Unknown));
}

// cargo: `new` makes the folder it names (like `mkdir`), `init` writes a
// `Cargo.toml` into a folder (like `git init`'s `.git`). Only these two run,
// and only with options dexily models.

fn expects(words: &[&str]) -> Result<Vec<Expected>, Refusal> {
    expect(&argv(words), cwd()).map(|e| e.folders)
}

fn new_folder_at(path: &str) -> Result<Vec<Expected>, Refusal> {
    Ok(vec![Expected {
        folder: cwd().join(path),
        proof: Proof::NewFolder,
    }])
}

fn holds_at(path: &str, entry: &'static str) -> Result<Vec<Expected>, Refusal> {
    Ok(vec![Expected {
        folder: cwd().join(path),
        proof: Proof::Holds(entry),
    }])
}

#[test]
fn cargo_new_expects_the_folder_it_names_as_a_new_folder() {
    assert_eq!(expects(&["cargo", "new", "app"]), new_folder_at("app"));
    assert_eq!(
        expects(&["cargo", "new", "-q", "--vcs", "none", "app"]),
        new_folder_at("app")
    );
    assert_eq!(
        expects(&["cargo", "new", "--lib", "--", "-odd"]),
        new_folder_at("-odd"),
        "after `--` a name may start with a dash"
    );
}

#[test]
fn cargo_new_options_with_a_value_are_not_the_folder() {
    for words in [
        &["cargo", "new", "--edition", "2021", "app"][..],
        &["cargo", "new", "--edition=2021", "app"],
        &["cargo", "new", "--name", "mycrate", "app"],
        &[
            "cargo",
            "new",
            "--vcs",
            "git",
            "--registry",
            "crates-io",
            "app",
        ],
    ] {
        assert_eq!(expects(words), new_folder_at("app"), "{words:?}");
    }
}

#[test]
fn cargo_new_with_no_path_names_no_folder_but_is_still_known() {
    // It runs — cargo prints its usage, `--help` works — and there is nothing
    // to track, exactly as `mkdir` with no name or `git clone` with no repo.
    for words in [&["cargo", "new"][..], &["cargo", "new", "--help"]] {
        assert_eq!(expects(words), Ok(Vec::new()), "{words:?}");
    }
}

#[test]
fn cargo_init_is_the_working_folder_or_the_one_it_names() {
    assert_eq!(expects(&["cargo", "init"]), holds_at("", "Cargo.toml"));
    assert_eq!(
        expects(&["cargo", "init", "app"]),
        holds_at("app", "Cargo.toml")
    );
    assert_eq!(
        expects(&["cargo", "init", "-q", "--lib", "--edition=2021"]),
        holds_at("", "Cargo.toml")
    );
}

#[test]
fn a_cargo_option_dexily_does_not_model_is_refused() {
    for words in [
        // Config injection and unstable flags: refused, not modelled.
        &["cargo", "new", "--config", "net.offline=true", "app"][..],
        &["cargo", "new", "-Z", "unstable-options", "app"],
        &["cargo", "init", "--manifest-path", "x", "app"],
        // An abbreviation is not the option: cargo takes some, dexily does not.
        &["cargo", "new", "--edi", "2021", "app"],
        // Short options run together, as git refuses `-qu`.
        &["cargo", "new", "-qv", "app"],
        // A flag given a value it does not take.
        &["cargo", "new", "--lib=yes", "app"],
    ] {
        assert!(
            matches!(expects(words), Err(Refusal::Refused(..))),
            "{words:?}: {:?}",
            expects(words)
        );
    }
}

#[test]
fn a_refused_cargo_option_names_it_and_says_to_run_cargo_directly() {
    let Err(Refusal::Refused(why, _)) = expects(&["cargo", "new", "--config", "x=y", "app"]) else {
        panic!("refused");
    };
    assert!(
        why.starts_with("`--config` is not an option dexily models for `cargo new`:"),
        "{why}"
    );
    assert!(why.ends_with("Run `cargo new` directly."), "{why}");
}

#[test]
fn any_other_cargo_command_is_not_known() {
    for words in [
        &["cargo", "build"][..],
        &["cargo", "run"],
        &["cargo", "test"],
        &["cargo", "add", "serde"],
        &["cargo"],
    ] {
        assert_eq!(expects(words), Err(Refusal::Unknown), "{words:?}");
    }
}

// go: the one shape is `go mod init`, which writes a `go.mod` in the working
// folder (like `git init`'s `.git`). The `[module]` argument is a module path,
// never a folder.

#[test]
fn go_mod_init_expects_the_working_folder() {
    assert_eq!(expects(&["go", "mod", "init"]), holds_at("", "go.mod"));
    assert_eq!(
        expects(&["go", "mod", "init", "example.com/you/thing"]),
        holds_at("", "go.mod"),
        "the module path is not a folder"
    );
}

#[test]
fn go_mod_init_takes_no_options() {
    for words in [
        &["go", "mod", "init", "-x"][..],
        &["go", "mod", "init", "--anything", "example.com/x"],
    ] {
        assert!(
            matches!(expects(words), Err(Refusal::Refused(..))),
            "{words:?}: {:?}",
            expects(words)
        );
    }
}

// gh: the one shape is `gh repo create`. With `--clone` it clones the new repo
// into a folder named after it (proven by a `.git`, like `git clone`); without
// `--clone` nothing lands on disk, so nothing is tracked.

#[test]
fn gh_repo_create_with_clone_expects_the_folder_named_after_the_repo() {
    assert_eq!(
        expects(&["gh", "repo", "create", "loc", "--private", "--clone"]),
        holds_at("loc", ".git")
    );
    assert_eq!(
        expects(&["gh", "repo", "create", "me/loc", "-c"]),
        holds_at("loc", ".git"),
        "the owner/ prefix is not part of the local folder"
    );
    assert_eq!(
        expects(&[
            "gh",
            "repo",
            "create",
            "--description",
            "a repo",
            "loc",
            "--clone"
        ]),
        holds_at("loc", ".git"),
        "a value is skipped, never read as the repository name"
    );
}

#[test]
fn gh_repo_create_without_clone_tracks_nothing() {
    // Made on GitHub, but nothing lands on disk.
    assert_eq!(
        expects(&["gh", "repo", "create", "loc", "--private"]),
        Ok(Vec::new())
    );
    // `--clone` but no name: gh cannot clone, so there is nothing to track.
    assert_eq!(
        expects(&["gh", "repo", "create", "--clone"]),
        Ok(Vec::new())
    );
}

#[test]
fn gh_repo_create_options_fail_closed() {
    for words in [
        &["gh", "repo", "create", "loc", "--secret"][..],
        &["gh", "repo", "create", "loc", "--clo"],
        &["gh", "repo", "create", "loc", "--private=yes"],
    ] {
        assert!(
            matches!(expects(words), Err(Refusal::Refused(..))),
            "{words:?}: {:?}",
            expects(words)
        );
    }
}

#[test]
fn any_other_gh_command_is_not_known() {
    for words in [
        &["gh", "repo", "clone", "me/thing"][..],
        &["gh", "pr", "list"],
        &["gh", "repo"],
        &["gh"],
    ] {
        assert_eq!(expects(words), Err(Refusal::Unknown), "{words:?}");
    }
}

// npm / pnpm / yarn init write a package.json in the working folder; poetry
// init writes a pyproject.toml. Only the bare form runs — an initializer
// positional (`npm init vite`) downloads and runs a package, so it is refused.

#[test]
fn a_bare_package_manager_init_writes_its_manifest() {
    for tool in ["npm", "pnpm", "yarn"] {
        assert_eq!(
            expects(&[tool, "init"]),
            holds_at("", "package.json"),
            "{tool}"
        );
        assert_eq!(
            expects(&[tool, "init", "-y"]),
            holds_at("", "package.json"),
            "{tool} -y"
        );
    }
    assert_eq!(expects(&["poetry", "init"]), holds_at("", "pyproject.toml"));
    assert_eq!(
        expects(&["poetry", "init", "--name", "thing", "-n"]),
        holds_at("", "pyproject.toml")
    );
}

#[test]
fn an_init_with_an_initializer_is_refused() {
    // The positional names a package npm would download and run.
    for words in [
        &["npm", "init", "vite"][..],
        &["npm", "init", "@vue/create"],
        &["yarn", "init", "react-app"],
        &["pnpm", "init", "foo"],
        &["poetry", "init", "foo"],
    ] {
        assert!(
            matches!(expects(words), Err(Refusal::Refused(..))),
            "{words:?}: {:?}",
            expects(words)
        );
    }
}

#[test]
fn an_init_with_an_unmodeled_flag_is_refused() {
    assert!(matches!(
        expects(&["npm", "init", "--registry=https://example.test"]),
        Err(Refusal::Refused(..))
    ));
}

#[test]
fn any_package_manager_command_other_than_init_is_unknown() {
    for words in [
        &["npm", "install"][..],
        &["npm", "run", "build"],
        &["npm", "create", "vite"],
        &["pnpm", "add", "x"],
        &["yarn"],
        &["poetry", "add", "x"],
        &["poetry", "new", "x"],
        &["poetry"],
    ] {
        assert_eq!(expects(words), Err(Refusal::Unknown), "{words:?}");
    }
}

// uv init scaffolds a Python project (a pyproject.toml) in the folder it names,
// or the working one. Its positional is that folder — not a downloaded package
// as npm's is — so it is tracked, like `cargo init`.

#[test]
fn uv_init_expects_the_folder_it_names_or_the_working_one() {
    assert_eq!(expects(&["uv", "init"]), holds_at("", "pyproject.toml"));
    assert_eq!(
        expects(&["uv", "init", "app"]),
        holds_at("app", "pyproject.toml")
    );
    assert_eq!(
        expects(&["uv", "init", "--python", "3.12", "app"]),
        holds_at("app", "pyproject.toml"),
        "the --python value is not the folder"
    );
    assert_eq!(
        expects(&["uv", "init", "--lib", "mylib"]),
        holds_at("mylib", "pyproject.toml")
    );
}

#[test]
fn uv_init_fails_closed_and_other_uv_commands_are_unknown() {
    assert!(matches!(
        expects(&["uv", "init", "--badflag"]),
        Err(Refusal::Refused(..))
    ));
    for words in [
        &["uv", "add", "requests"][..],
        &["uv", "sync"],
        &["uv", "run", "main.py"],
        &["uv", "pip", "install", "x"],
        &["uv"],
    ] {
        assert_eq!(expects(words), Err(Refusal::Unknown), "{words:?}");
    }
}

#[test]
fn any_other_go_command_is_not_known() {
    for words in [
        &["go", "build"][..],
        &["go", "run", "."],
        &["go", "mod"],
        &["go", "mod", "tidy"],
        &["go", "version"],
        &["go"],
    ] {
        assert_eq!(expects(words), Err(Refusal::Unknown), "{words:?}");
    }
}
