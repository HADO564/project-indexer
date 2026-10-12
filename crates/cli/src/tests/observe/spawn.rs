//! `spawn::run` runs small real commands, so these check the exit code the
//! way a shell sees it — the observer's first rule. The rules for Windows'
//! `cmd` built-ins are pure functions, checked here on every platform.

use std::ffi::{OsStr, OsString};
use std::path::Path;

use crate::observe::spawn::{check_cmd_args, cmd_builtin, cmd_exe, cmd_tail, run, unsafe_for_cmd};

fn argv(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

/// Runs `words` in a fresh folder of its own, so no test depends on where
/// the test process happens to be.
fn run_in_tempdir(words: &[&str]) -> anyhow::Result<i32> {
    let dir = tempfile::tempdir().unwrap();
    run(&argv(words), dir.path())
}

/// A command that exits with `code`, on whichever system runs the tests.
fn exits_with(code: i32) -> Vec<String> {
    let program = if cfg!(windows) { "cmd" } else { "sh" };
    let flag = if cfg!(windows) { "/C" } else { "-c" };
    vec![program.into(), flag.into(), format!("exit {code}")]
}

#[test]
fn the_command_s_exit_code_passes_through_untouched() {
    for code in [0, 1, 3, 42] {
        let words = exits_with(code);
        let words: Vec<&str> = words.iter().map(String::as_str).collect();
        assert_eq!(run_in_tempdir(&words).unwrap(), code);
    }
}

#[test]
fn an_argument_reaches_the_command_whole_with_no_shell_between() {
    // One argument holding `;` and a second command: with no shell, it is
    // just text, and `test` sees exactly one non-empty argument.
    if cfg!(windows) {
        return;
    }
    let code = run_in_tempdir(&["test", "-n", "x; exit 7"]).unwrap();
    assert_eq!(code, 0, "the `; exit 7` never ran");
}

#[cfg(unix)]
#[test]
fn the_command_runs_in_the_folder_it_is_given() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(run(&argv(&["mkdir", "made-here"]), dir.path()).unwrap(), 0);
    assert!(dir.path().join("made-here").is_dir());
}

#[test]
fn a_missing_program_is_an_error_naming_it() {
    let err = run_in_tempdir(&["dexily-no-such-program"]).unwrap_err();
    assert!(
        format!("{err:#}").starts_with("could not run `dexily-no-such-program`"),
        "{err:#}"
    );
}

#[test]
fn nothing_to_run_is_an_error() {
    let err = run(&[], Path::new(".")).unwrap_err();
    assert_eq!(err.to_string(), "no command to run");
}

#[cfg(unix)]
#[test]
fn a_command_ended_by_a_signal_is_128_plus_the_signal() {
    // The shell sends itself SIGINT (2), as Ctrl+C would: 130.
    assert_eq!(run_in_tempdir(&["sh", "-c", "kill -INT $$"]).unwrap(), 130);
    // SIGTERM (15): 143.
    assert_eq!(run_in_tempdir(&["sh", "-c", "kill -TERM $$"]).unwrap(), 143);
}

// Windows' `cmd` built-ins: the rules, as pure functions.

#[test]
fn mkdir_and_its_kind_are_cmd_built_ins_ignoring_case() {
    assert_eq!(cmd_builtin(OsStr::new("mkdir")), Some("mkdir"));
    assert_eq!(cmd_builtin(OsStr::new("MKDIR")), Some("mkdir"));
    assert_eq!(cmd_builtin(OsStr::new("md")), Some("md"));
    assert_eq!(cmd_builtin(OsStr::new("Md")), Some("md"));
    for builtin in [
        "move", "copy", "del", "rmdir", "rd", "ren", "rename", "type", "dir", "echo",
    ] {
        assert_eq!(cmd_builtin(OsStr::new(builtin)), Some(builtin));
    }
}

#[test]
fn an_ordinary_program_or_a_path_is_not_a_built_in() {
    for program in ["git", "cmd", "mkdir.exe", r"C:\tools\mkdir", "./mkdir", ""] {
        assert_eq!(cmd_builtin(OsStr::new(program)), None, "{program:?}");
    }
}

#[test]
fn every_character_cmd_acts_on_is_unsafe() {
    for special in ['&', '|', '<', '>', '^', '%', '"', '\n', '\r'] {
        assert!(unsafe_for_cmd(&format!("a{special}b")), "{special:?}");
    }
}

#[test]
fn an_ordinary_name_is_safe_for_cmd() {
    for name in [
        "thing",
        "my project",
        r"work\clients\acme",
        "a-b_c.d",
        "(x)",
        "",
    ] {
        assert!(!unsafe_for_cmd(name), "{name:?}");
    }
}

#[test]
fn an_argument_cmd_would_act_on_is_refused_by_name() {
    let err = check_cmd_args(&argv(&["fine", "a & calc"])).unwrap_err();
    assert_eq!(
        err,
        "\"a & calc\" has a character cmd would act on (& | < > ^ % \"); \
         dexily will not pass it to cmd"
    );
    for bad in ["a|b", "a>b", "a<b", "a^b", "%PATH%", "a\"b", "a\nb"] {
        assert!(check_cmd_args(&argv(&[bad])).is_err(), "{bad:?}");
    }
}

#[test]
fn arguments_free_of_them_pass() {
    assert_eq!(check_cmd_args(&argv(&["thing", "my project"])), Ok(()));
    assert_eq!(check_cmd_args(&[]), Ok(()));
}

#[test]
fn a_plain_name_goes_into_the_tail_unquoted() {
    assert_eq!(cmd_tail("mkdir", &argv(&["thing"])), "mkdir thing");
    assert_eq!(cmd_tail("mkdir", &argv(&["a", r"b\c"])), r"mkdir a b\c");
    assert_eq!(cmd_tail("mkdir", &[]), "mkdir");
}

#[test]
fn a_name_cmd_would_split_is_quoted_in_the_tail() {
    assert_eq!(
        cmd_tail("mkdir", &argv(&["my project"])),
        "mkdir \"my project\""
    );
    assert_eq!(
        cmd_tail("mkdir", &argv(&["a", "b c", "d"])),
        "mkdir a \"b c\" d"
    );
    // cmd also splits on `,`, `;` and `=`, and groups with brackets.
    for name in ["a,b", "a;b", "a=b", "(x)"] {
        assert_eq!(
            cmd_tail("mkdir", &argv(&[name])),
            format!("mkdir \"{name}\"")
        );
    }
    // An empty argument would vanish without its quotes.
    assert_eq!(cmd_tail("mkdir", &argv(&[""])), "mkdir \"\"");
}

#[test]
fn cmd_is_named_by_its_full_path_under_the_system_root() {
    let cmd = cmd_exe(Some(OsString::from(r"D:\WinNT")));
    assert!(cmd.starts_with(r"D:\WinNT"), "{cmd:?}");
    assert!(cmd.ends_with("System32/cmd.exe"), "{cmd:?}");
}

#[test]
fn without_a_system_root_cmd_is_under_c_windows() {
    for missing in [None, Some(OsString::new())] {
        let cmd = cmd_exe(missing);
        assert!(cmd.starts_with(r"C:\Windows"), "{cmd:?}");
        assert!(cmd.ends_with("System32/cmd.exe"), "{cmd:?}");
    }
}

// Windows only: the built-in really runs, through `cmd`.

#[cfg(windows)]
#[test]
fn mkdir_runs_through_cmd_and_makes_the_folder() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(run(&argv(&["mkdir", "thing"]), dir.path()).unwrap(), 0);
    assert!(dir.path().join("thing").is_dir());
}

#[cfg(windows)]
#[test]
fn a_folder_with_a_space_in_its_name_is_one_folder() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(run(&argv(&["mkdir", "my project"]), dir.path()).unwrap(), 0);
    assert!(dir.path().join("my project").is_dir());
    assert!(!dir.path().join("my").exists(), "it was not split in two");
}

#[cfg(windows)]
#[test]
fn an_argument_cmd_would_act_on_stops_the_command_before_it_runs() {
    use crate::commands::Failure;

    let dir = tempfile::tempdir().unwrap();
    let err = run(&argv(&["mkdir", "a & mkdir b"]), dir.path()).unwrap_err();
    assert!(
        matches!(err.downcast_ref::<Failure>(), Some(Failure::Usage { .. })),
        "{err:#}"
    );
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        0,
        "nothing was created"
    );
}

#[cfg(windows)]
#[test]
fn a_built_in_s_failure_keeps_its_exit_code() {
    // `mkdir` of a folder that exists fails in cmd with exit code 1.
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(run(&argv(&["mkdir", "twice"]), dir.path()).unwrap(), 0);
    assert_eq!(run(&argv(&["mkdir", "twice"]), dir.path()).unwrap(), 1);
}
