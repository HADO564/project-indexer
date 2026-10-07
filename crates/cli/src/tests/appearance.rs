use crate::appearance::{color, icon, rgb, BUNDLED_ICONS};

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
