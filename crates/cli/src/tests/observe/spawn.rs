//! `spawn::run` runs small real commands, so these check the exit code the
//! way a shell sees it — the observer's first rule.

use std::ffi::OsString;

use crate::observe::spawn::run;

fn argv(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

/// A command that exits with `code`, on whichever system runs the tests.
fn exits_with(code: i32) -> Vec<OsString> {
    if cfg!(windows) {
        argv(&["cmd", "/C", &format!("exit {code}")])
    } else {
        argv(&["sh", "-c", &format!("exit {code}")])
    }
}

#[test]
fn the_command_s_exit_code_passes_through_untouched() {
    for code in [0, 1, 3, 42] {
        assert_eq!(run(&exits_with(code)).unwrap(), code);
    }
}

#[test]
fn an_argument_reaches_the_command_whole_with_no_shell_between() {
    // One argument holding `;` and a second command: with no shell, it is
    // just text, and `test` sees exactly one non-empty argument.
    if cfg!(windows) {
        return;
    }
    let code = run(&argv(&["test", "-n", "x; exit 7"])).unwrap();
    assert_eq!(code, 0, "the `; exit 7` never ran");
}

#[test]
fn a_missing_program_is_an_error_naming_it() {
    let err = run(&argv(&["dexily-no-such-program"])).unwrap_err();
    assert!(
        format!("{err:#}").starts_with("could not run `dexily-no-such-program`"),
        "{err:#}"
    );
}

#[test]
fn nothing_to_run_is_an_error() {
    assert_eq!(run(&[]).unwrap_err().to_string(), "no command to run");
}

#[cfg(unix)]
#[test]
fn a_command_ended_by_a_signal_is_128_plus_the_signal() {
    // The shell sends itself SIGINT (2), as Ctrl+C would: 130.
    assert_eq!(run(&argv(&["sh", "-c", "kill -INT $$"])).unwrap(), 130);
    // SIGTERM (15): 143.
    assert_eq!(run(&argv(&["sh", "-c", "kill -TERM $$"])).unwrap(), 143);
}
