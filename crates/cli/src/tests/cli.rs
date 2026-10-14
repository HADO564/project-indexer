//! Parse tests for the top-level [`Cli`]'s global flags.

use clap::Parser;

use crate::Cli;

#[test]
fn verbose_is_global_before_and_after_a_subcommand() {
    // Before the subcommand, where it must sit ahead of an observed command.
    let cli = Cli::try_parse_from(["dexily", "-v", "list"])
        .expect("`-v` should be accepted before a subcommand");
    assert!(cli.verbose);

    // And after it, like the other global flags (`--yes`, `--quiet`).
    let cli = Cli::try_parse_from(["dexily", "list", "--verbose"])
        .expect("`--verbose` should be accepted after a subcommand");
    assert!(cli.verbose);
}

#[test]
fn verbose_is_off_without_the_flag() {
    let cli = Cli::try_parse_from(["dexily", "list"]).expect("arguments should parse");
    assert!(!cli.verbose);
}
