//! The `:` command line: turn a typed line into the shell's [`Command`] and run
//! it, so the TUI is a way of *typing* commands rather than a second path to the
//! data. Every mutation the TUI makes goes through this, exactly as the shell's
//! mutations do.

use clap::Parser;

use crate::commands::group::GroupAction;
use crate::commands::{self, Command, Outcome};
use crate::context::Context;
use crate::output::{human, Look};

/// A clap wrapper whose only field is the shell's `Command` subcommand, parsed
/// with no leading program name so `open app` parses straight to `Command::Open`.
#[derive(Parser)]
#[command(no_binary_name = true)]
struct Line {
    #[command(subcommand)]
    command: Command,
}

/// Parse `line` into a `Command`, run it, and return the one-line result for the
/// status line: the command's own prose on success (empty for a command whose
/// output is a table the panes already show), or the reason it did not run.
pub(crate) fn run_line(line: &str, ctx: &Context, look: Look) -> String {
    let argv = match tokenize(line) {
        Ok(argv) => argv,
        Err(reason) => return format!("dexily: {reason}"),
    };
    if argv.is_empty() {
        return String::new();
    }
    let command = match Line::try_parse_from(&argv) {
        Ok(parsed) => parsed.command,
        // clap errors are multi-line (the error then the usage); the status line
        // has room for one, so show the first — the "error: …" line.
        Err(error) => return first_line(&error.to_string()),
    };
    match commands::run(command, ctx) {
        Ok(outcome) => message_for(&outcome, look),
        Err(error) => format!("dexily: {error:#}"),
    }
}

/// If `line` parses to a command that deletes or stops tracking something, the
/// prompt to show before running it; otherwise `None`. The TUI puts a `y/N`
/// gate in front of these, matching the shell, which confirms the same three.
pub(crate) fn needs_confirm(line: &str) -> Option<String> {
    let argv = tokenize(line).ok()?;
    let command = Line::try_parse_from(&argv).ok()?.command;
    let prompt = match command {
        Command::Untrack(_) => "Stop tracking this project? Its files are left alone. [y/N]",
        Command::Purge(_) => "Permanently delete this binned record? Cannot be undone. [y/N]",
        Command::Group(args) if matches!(args.action, GroupAction::Delete { .. }) => {
            "Delete this group? Its projects are kept, ungrouped. [y/N]"
        }
        _ => return None,
    };
    Some(prompt.to_string())
}

/// Renders an outcome's **prose** (the message, not the table) by handing the
/// result to a sink and the prose to a buffer — the same split
/// `output::human::write` draws for the shell.
fn message_for(outcome: &Outcome, look: Look) -> String {
    let mut result = Vec::new();
    let mut prose = Vec::new();
    let _ = human::write(&mut result, &mut prose, outcome, look);
    String::from_utf8_lossy(&prose).trim().to_string()
}

/// Splits a line into arguments on whitespace, with single or double quotes to
/// keep a value that contains spaces together (`open "build at 12:30"`). An
/// unclosed quote is an error rather than a silently half-read argument.
pub(crate) fn tokenize(line: &str) -> Result<Vec<String>, &'static str> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quote: Option<char> = None;
    for ch in line.chars() {
        match quote {
            Some(q) if ch == q => quote = None,
            Some(_) => current.push(ch),
            None => match ch {
                '\'' | '"' => {
                    quote = Some(ch);
                    started = true;
                }
                c if c.is_whitespace() => {
                    if started {
                        args.push(std::mem::take(&mut current));
                        started = false;
                    }
                }
                c => {
                    current.push(c);
                    started = true;
                }
            },
        }
    }
    if quote.is_some() {
        return Err("unclosed quote");
    }
    if started {
        args.push(current);
    }
    Ok(args)
}

/// The first non-empty line of a message, trimmed — for a clap error, its
/// `error: …` line.
pub(crate) fn first_line(message: &str) -> String {
    message
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .to_string()
}
