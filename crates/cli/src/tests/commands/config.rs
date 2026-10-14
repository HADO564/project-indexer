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
        setting(&["dexily", "config", "form-wrap", "on"]),
        Setting::FormWrap {
            value: Some(Switch::On),
            reset: false
        }
    ));
    assert!(matches!(
        setting(&["dexily", "config", "form-wrap", "off"]),
        Setting::FormWrap {
            value: Some(Switch::Off),
            reset: false
        }
    ));
}

#[test]
fn form_wrap_alone_shows_the_current_value() {
    assert!(matches!(
        setting(&["dexily", "config", "form-wrap"]),
        Setting::FormWrap {
            value: None,
            reset: false
        }
    ));
}

#[test]
fn form_wrap_rejects_anything_but_on_or_off() {
    assert!(Cli::try_parse_from(["dexily", "config", "form-wrap", "maybe"]).is_err());
}

#[test]
fn form_wrap_reset_cannot_be_given_a_value_too() {
    assert!(Cli::try_parse_from(["dexily", "config", "form-wrap", "off", "--reset"]).is_err());
}

#[test]
fn track_prompt_takes_on_or_off() {
    assert!(matches!(
        setting(&["dexily", "config", "track-prompt", "on"]),
        Setting::TrackPrompt {
            value: Some(Switch::On),
            reset: false
        }
    ));
    assert!(matches!(
        setting(&["dexily", "config", "track-prompt", "off"]),
        Setting::TrackPrompt {
            value: Some(Switch::Off),
            reset: false
        }
    ));
}

#[test]
fn track_prompt_alone_shows_the_current_value() {
    assert!(matches!(
        setting(&["dexily", "config", "track-prompt"]),
        Setting::TrackPrompt {
            value: None,
            reset: false
        }
    ));
}

#[test]
fn track_prompt_takes_reset() {
    assert!(matches!(
        setting(&["dexily", "config", "track-prompt", "--reset"]),
        Setting::TrackPrompt {
            value: None,
            reset: true
        }
    ));
}

#[test]
fn track_prompt_rejects_anything_but_on_or_off() {
    assert!(Cli::try_parse_from(["dexily", "config", "track-prompt", "sometimes"]).is_err());
}

#[test]
fn track_prompt_reset_cannot_be_given_a_value_too() {
    assert!(Cli::try_parse_from(["dexily", "config", "track-prompt", "off", "--reset"]).is_err());
}
