//! Runs the wrapped command with inherited stdio and hands back its exit
//! status untouched.
//!
//! Never through a shell, with one guarded exception: on Windows `mkdir` and
//! its kind are not programs but built into `cmd.exe`, so they can only run
//! through it. The rules for that path are pure functions here, tested on
//! every platform; the `#[cfg(windows)]` part only strings them together.

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
    sync::{
        atomic::{AtomicBool, Ordering},
        Once,
    },
};

use anyhow::{bail, Context};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[cfg(windows)]
use crate::commands::Failure;

/// Runs `argv` — a program and its arguments — in `cwd`, with this terminal's
/// stdin, stdout and stderr, waits for it, and returns its exit code.
///
/// Each argument is handed over on its own, with no shell between: a folder
/// named `x; rm -rf ~` is one odd name, never a second command. On Windows a
/// `cmd` built-in has to go through `cmd`, which is a shell, so there an
/// argument `cmd` would act on is refused before anything runs.
///
/// `cwd` is given rather than taken from the process, so a caller and its
/// tests never depend on where the process happens to be.
pub fn run(argv: &[OsString], cwd: &Path) -> anyhow::Result<i32> {
    let Some((program, rest)) = argv.split_first() else {
        bail!("no command to run");
    };
    let mut command = command(program, rest)?;
    leave_interrupts_to_the_command();
    RUNNING.store(true, Ordering::SeqCst);
    let status = command.current_dir(cwd).status();
    RUNNING.store(false, Ordering::SeqCst);
    let status =
        status.with_context(|| format!("could not run `{}`", program.to_string_lossy()))?;
    Ok(exit_code(status))
}

/// The command to start: the program itself with its arguments, as given.
#[cfg(not(windows))]
fn command(program: &OsStr, rest: &[OsString]) -> anyhow::Result<Command> {
    let mut command = Command::new(program);
    command.args(rest);
    Ok(command)
}

/// The command to start: the program itself with its arguments, or for a
/// `cmd` built-in, `cmd.exe /D /C <built-in> <arguments>` — once the
/// arguments have been checked for anything `cmd` would act on.
///
/// A program that is not a built-in goes to `Command::new` as it is: Rust
/// does not look for a bare name in the current folder first, and it quotes
/// arguments for `.cmd` and `.bat` scripts itself.
#[cfg(windows)]
fn command(program: &OsStr, rest: &[OsString]) -> anyhow::Result<Command> {
    use std::os::windows::process::CommandExt;

    let Some(builtin) = cmd_builtin(program) else {
        let mut command = Command::new(program);
        command.args(rest);
        return Ok(command);
    };
    if let Err(message) = check_cmd_args(rest) {
        return Err(Failure::Usage { message }.into());
    }
    let mut command = Command::new(cmd_exe(std::env::var_os("SystemRoot")));
    // `/D` skips any AutoRun commands from the registry, and `/V:OFF` keeps
    // `!name!` from expanding even where the registry turns that on. The tail is appended
    // raw: `cmd` has its own quoting rules, and Rust's escaping would fight
    // them.
    command
        .arg("/D")
        .arg("/V:OFF")
        .arg("/C")
        .raw_arg(cmd_tail(builtin, rest));
    Ok(command)
}

/// The built-ins of `cmd.exe` that dexily may be asked to run: on Windows
/// these are not programs on `PATH`, so they can only run through `cmd`.
const CMD_BUILTINS: [&str; 12] = [
    "mkdir", "md", "move", "copy", "del", "rmdir", "rd", "ren", "rename", "type", "dir", "echo",
];

/// `program` as a `cmd` built-in — its name in lower case — or `None` when it
/// is an ordinary program. Matched ignoring case, as `cmd` does (`MKDIR`,
/// `Md`), and only as a bare name: `C:\tools\mkdir` is a program of its own.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn cmd_builtin(program: &OsStr) -> Option<&'static str> {
    let name = program.to_str()?.to_ascii_lowercase();
    CMD_BUILTINS
        .iter()
        .copied()
        .find(|&builtin| builtin == name)
}

/// The characters `cmd` acts on rather than passes along: `&` and `|` chain
/// commands, `<` and `>` redirect, `^` escapes, `%` expands a variable, `"`
/// opens or closes quoting, and a line break ends the command.
const CMD_SPECIAL: [char; 9] = ['&', '|', '<', '>', '^', '%', '"', '\n', '\r'];

/// Whether `cmd` would act on something in `arg` instead of passing it along.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn unsafe_for_cmd(arg: &str) -> bool {
    arg.contains(CMD_SPECIAL)
}

/// Refuses the first argument `cmd` would act on, with the message to show:
/// `dexily mkdir "a & calc"` would otherwise make `a` and then start
/// Calculator. Nothing is escaped or repaired — a name that needs one of
/// these is rare, and refusing is the only answer that cannot be wrong.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn check_cmd_args(args: &[OsString]) -> Result<(), String> {
    for arg in args {
        let arg = arg.to_string_lossy();
        if unsafe_for_cmd(&arg) {
            return Err(format!(
                "\"{}\" has a character cmd would act on (& | < > ^ % \"); \
                 dexily will not pass it to cmd",
                // A line break would split the message itself.
                arg.replace(['\n', '\r'], " ")
            ));
        }
    }
    Ok(())
}

/// The text after `cmd /C`: the built-in, then each argument, in double
/// quotes when `cmd` would otherwise split it or drop it — one with a space
/// or another of `cmd`'s separators, or an empty one. An argument cannot hold
/// a `"` by the time it gets here ([`check_cmd_args`]), so the quoting never
/// needs escaping.
///
/// It always starts with the built-in's name, never a quote, so `cmd /C`
/// leaves the quotes in it alone.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn cmd_tail(builtin: &str, args: &[OsString]) -> String {
    let mut tail = String::from(builtin);
    for arg in args {
        let arg = arg.to_string_lossy();
        tail.push(' ');
        if arg.is_empty() || arg.contains([' ', '\t', ',', ';', '=', '(', ')']) {
            tail.push('"');
            tail.push_str(&arg);
            tail.push('"');
        } else {
            tail.push_str(&arg);
        }
    }
    tail
}

/// `cmd.exe` by its full path under `system_root` — the `SystemRoot`
/// variable, `C:\Windows` when it is missing or empty. Never looked up on
/// `PATH`, where a planted `cmd.exe` could be found first.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn cmd_exe(system_root: Option<OsString>) -> PathBuf {
    let root = system_root
        .filter(|root| !root.is_empty())
        .unwrap_or_else(|| OsString::from(r"C:\Windows"));
    PathBuf::from(root).join("System32").join("cmd.exe")
}

/// The number a shell would report for `status`: the command's own exit code,
/// or for one ended by a signal (Unix only), 128 + the signal's number — Ctrl+C
/// is 130 — so a script can tell "cancelled" from "failed".
pub fn exit_code(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    if let Some(signal) = status.signal() {
        return 128 + signal;
    }
    1
}

/// Whether the wrapped command is running right now.
static RUNNING: AtomicBool = AtomicBool::new(false);

/// Ctrl+C reaches dexily as well as the command, since they share the
/// terminal. Without this dexily would die on the spot, before it could report
/// the command's exit code or record anything; with it, while the command
/// runs the command alone decides what Ctrl+C means, and dexily carries on
/// once it has exited.
///
/// Only while it runs. Afterwards — at the `[Y/n/e]` prompt — Ctrl+C is the
/// user's to stop dexily with, and ends it as a shell would report it, 130.
/// A handler cannot be taken back out once installed, so it checks which of
/// the two moments this is.
///
/// Installed once: a process may hold only one handler. If it cannot be
/// installed, Ctrl+C simply ends dexily too, as it would without it — never a
/// reason to refuse to run the command.
fn leave_interrupts_to_the_command() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = ctrlc::set_handler(|| {
            if !RUNNING.load(Ordering::SeqCst) {
                std::process::exit(130);
            }
        });
    });
}
