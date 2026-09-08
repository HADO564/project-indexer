//! Tests for [`crate::domain::palette`].

use crate::domain::palette::*;

#[test]
fn known_swatches_are_recognized() {
    assert!(is_known_swatch("cyan"));
    assert!(is_known_swatch("pink"));
}

#[test]
fn unknown_swatch_is_rejected() {
    assert!(!is_known_swatch("chartreuse"));
    assert!(!is_known_swatch("#e7b64e"));
    assert!(!is_known_swatch(""));
}

#[test]
fn hex_literals_are_accepted_only_in_the_exact_six_digit_form() {
    assert!(is_hex_literal("#e7b64e"));
    assert!(is_hex_literal("#FFFFFF"));

    // Everything else is refused rather than normalised — these values end
    // up in a `style` attribute.
    assert!(!is_hex_literal("#abc"));
    assert!(!is_hex_literal("#e7b64"));
    assert!(!is_hex_literal("#e7b64ee"));
    assert!(!is_hex_literal("e7b64e"));
    assert!(!is_hex_literal("#gggggg"));
    assert!(!is_hex_literal("red"));
    assert!(!is_hex_literal("rgb(1,2,3)"));
    assert!(!is_hex_literal("#e7b64e; background: url(x)"));
    assert!(!is_hex_literal(""));
}

#[test]
fn a_stored_colour_is_a_palette_name_or_a_hex_literal_and_nothing_else() {
    assert!(is_valid_color("cyan"));
    assert!(is_valid_color("#e7b64e"));

    assert!(!is_valid_color("chartreuse"));
    assert!(!is_valid_color("red"));
    assert!(!is_valid_color("#abc"));
    assert!(!is_valid_color("#e7b64e; background: url(x)"));
    assert!(!is_valid_color(""));

    // is_known_swatch stays the narrower question — "is this one of the
    // eight?" — which the pickers use to decide what to highlight.
    assert!(!is_known_swatch("#e7b64e"));
}

#[test]
fn there_are_eight_swatches_and_they_are_unique() {
    let mut sorted = SWATCHES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 8);
}
