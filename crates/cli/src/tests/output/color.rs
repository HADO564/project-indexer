use clap::ValueEnum;

use crate::output::color::Color;

#[test]
fn every_color_paints_and_resets() {
    for color in Color::value_variants() {
        let painted = color.paint("x");
        assert!(painted.starts_with("\x1b[1;"), "{color:?}: {painted:?}");
        assert!(painted.ends_with("x\x1b[0m"), "{color:?}: {painted:?}");
    }
}

#[test]
fn standard_colors_use_ansi_codes() {
    assert_eq!(Color::Cyan.sgr(), "36");
    assert_eq!(Color::BrightRed.sgr(), "91");
}

#[test]
fn named_colors_use_24_bit_codes() {
    assert_eq!(Color::Orange.sgr(), "38;2;255;165;0");
}

#[test]
fn names_parse_in_kebab_case_with_aliases() {
    assert_eq!(Color::from_str("hot-pink", true), Ok(Color::HotPink));
    assert_eq!(Color::from_str("grey", true), Ok(Color::BrightBlack));
    assert!(Color::from_str("not-a-colour", true).is_err());
}
