//! `config form-wrap`: how it parses. Running it reads and writes the real
//! settings file, so that is checked by hand against a scratch `HOME`.

use clap::Parser;

use crate::commands::config::{Setting, Switch};
use crate::commands::Command;
use crate::{Cli, Invocation};

fn setting(argv: &[&str]) -> Setting {
    let cli = Cli::try_parse_from(argv).expect("arguments should parse");
    match cli.invocation {
        Some(Invocation::Command(Command::Config(args))) => args.setting,
        other => panic!("expected config, got {other:?}"),
    }
}

#[test]
fn form_wrap_takes_on_or_off() {
    assert!(matches!(
        setting(&["indexer", "config", "form-wrap", "on"]),
        Setting::FormWrap {
            value: Some(Switch::On),
            reset: false
        }
    ));
    assert!(matches!(
        setting(&["indexer", "config", "form-wrap", "off"]),
        Setting::FormWrap {
            value: Some(Switch::Off),
            reset: false
        }
    ));
}

#[test]
fn form_wrap_alone_shows_the_current_value() {
    assert!(matches!(
        setting(&["indexer", "config", "form-wrap"]),
        Setting::FormWrap {
            value: None,
            reset: false
        }
    ));
}

#[test]
fn form_wrap_rejects_anything_but_on_or_off() {
    assert!(Cli::try_parse_from(["indexer", "config", "form-wrap", "maybe"]).is_err());
}

#[test]
fn form_wrap_reset_cannot_be_given_a_value_too() {
    assert!(Cli::try_parse_from(["indexer", "config", "form-wrap", "off", "--reset"]).is_err());
}
