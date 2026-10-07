use crate::appearance::{color, glyph, icon, mark_rgb, rgb, IconStyle, BUNDLED_ICONS};

// Colours: the eight palette names or `#rrggbb`, as core allows, lowercased.

#[test]
fn a_palette_name_or_hex_is_kept_lowercased_and_trimmed() {
    assert_eq!(color(" Cyan ").unwrap().as_deref(), Some("cyan"));
    assert_eq!(color("#FF8800").unwrap().as_deref(), Some("#ff8800"));
}

#[test]
fn a_blank_colour_clears_it() {
    assert_eq!(color("").unwrap(), None);
    assert_eq!(color("   ").unwrap(), None);
}

#[test]
fn anything_else_is_refused_with_the_palette() {
    for bad in ["red", "#abc", "#gg0000", "rgb(1,2,3)"] {
        let err = color(bad).unwrap_err().to_string();
        assert!(
            err.starts_with(&format!("not a colour: \"{bad}\""))
                && err.contains("cyan, gold, amber, rust, violet, green, blue, pink"),
            "{err}"
        );
    }
}

// Icons: a bundled name, or `custom:<name>` for one added in the app.

#[test]
fn a_bundled_icon_is_kept_lowercased() {
    assert_eq!(icon(" Rocket ", &[]).unwrap().as_deref(), Some("rocket"));
}

#[test]
fn a_blank_icon_clears_it() {
    assert_eq!(icon("  ", &[]).unwrap(), None);
}

#[test]
fn an_unknown_icon_is_refused_with_the_bundled_ones() {
    let err = icon("rockt", &[]).unwrap_err().to_string();
    assert!(
        err.starts_with("not an icon: \"rockt\". Use one of folder, briefcase"),
        "{err}"
    );
}

#[test]
fn a_custom_icon_must_be_one_the_app_has() {
    let custom = vec!["logo".to_string(), "team".to_string()];
    assert_eq!(
        icon("custom:logo", &custom).unwrap().as_deref(),
        Some("custom:logo")
    );
    assert_eq!(
        icon("custom:Logo", &custom).unwrap().as_deref(),
        Some("custom:logo"),
        "the app stores custom names lowercase"
    );
    assert_eq!(
        icon("custom:nope", &custom).unwrap_err().to_string(),
        "no custom icon named \"nope\". Custom icons: logo, team"
    );
    assert_eq!(
        icon("custom:logo", &[]).unwrap_err().to_string(),
        "no custom icon named \"logo\": none have been added in the app yet"
    );
}

/// The list is a copy of the app's: this is what notices when an icon is
/// added to, or removed from, `src/lib/icons.ts` and not here.
#[test]
fn bundled_icons_match_the_app() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../src/lib/icons.ts");
    let source = std::fs::read_to_string(path).expect("the app's icons.ts");
    let glyphs = &source[source.find("const GLYPHS").unwrap()..];
    let glyphs = &glyphs[..glyphs.find("\n};").unwrap()];
    let names: Vec<&str> = glyphs
        .lines()
        .filter_map(|line| {
            let name = line.strip_prefix("  ")?.split(':').next()?;
            (!name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '-'))
                .then_some(name)
        })
        .collect();
    assert_eq!(names, BUNDLED_ICONS);
}

// The swatch: palette names as the app paints them, hex as written.

#[test]
fn rgb_reads_palette_names_and_hex() {
    assert_eq!(rgb("cyan"), Some((0x56, 0xc8, 0xc4)));
    assert_eq!(rgb("#ff8800"), Some((0xff, 0x88, 0x00)));
    assert_eq!(rgb("cya"), None);
    assert_eq!(rgb("#ff88"), None);
}

// Glyphs: how an icon is drawn in a terminal.

#[test]
fn icons_off_draws_nothing() {
    assert_eq!(glyph("rocket", IconStyle::Off), None);
}

#[test]
fn every_bundled_icon_has_a_glyph_of_its_own_in_each_style() {
    for style in [IconStyle::Nerd, IconStyle::Emoji] {
        let mut seen: Vec<&str> = BUNDLED_ICONS
            .iter()
            .map(|name| glyph(name, style).unwrap())
            .collect();
        seen.sort();
        seen.dedup();
        assert_eq!(
            seen.len(),
            BUNDLED_ICONS.len(),
            "{style:?}: two icons share a glyph"
        );
    }
}

#[test]
fn every_emoji_is_two_columns_wide() {
    for name in BUNDLED_ICONS {
        let emoji = glyph(name, IconStyle::Emoji).unwrap();
        assert_eq!(
            unicode_width::UnicodeWidthStr::width(emoji),
            2,
            "{name}: {emoji}"
        );
    }
}

#[test]
fn a_custom_or_unknown_icon_is_drawn_as_the_folder() {
    let folder = glyph("folder", IconStyle::Nerd);
    assert_eq!(glyph("custom:logo", IconStyle::Nerd), folder);
    assert_eq!(glyph("teapot", IconStyle::Nerd), folder);
}

#[test]
fn a_mark_takes_its_own_colour_else_its_group_s() {
    assert_eq!(mark_rgb(Some("cyan"), Some("gold")), rgb("cyan"));
    assert_eq!(mark_rgb(None, Some("gold")), rgb("gold"));
    assert_eq!(mark_rgb(None, None), None);
}
