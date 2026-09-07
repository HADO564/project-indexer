use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use crate::error::IconError;

/// Largest icon accepted, before sanitizing. A glyph is a few kilobytes; this
/// is generous enough for a detailed logo and small enough that a hostile file
/// cannot be used to exhaust memory.
pub const MAX_SVG_BYTES: usize = 256 * 1024;

/// Elements that survive. An allow-list, never a deny-list: a deny-list is
/// wrong the moment SVG gains an element nobody here has heard of.
const ALLOWED_ELEMENTS: &[&str] = &[
    "svg", "g", "path", "circle", "ellipse", "rect", "line", "polyline", "polygon", "title", "desc",
];

/// Attributes that survive. Note what is absent and therefore dropped by
/// construction: every `on*` handler, `href`, `xlink:href`, `style`, `class`,
/// and anything namespaced.
const ALLOWED_ATTRS: &[&str] = &[
    "viewBox",
    "d",
    "cx",
    "cy",
    "r",
    "rx",
    "ry",
    "x",
    "y",
    "x1",
    "y1",
    "x2",
    "y2",
    "width",
    "height",
    "points",
    "transform",
    "fill",
    "stroke",
    "stroke-width",
    "stroke-linecap",
    "stroke-linejoin",
    "stroke-dasharray",
    "stroke-dashoffset",
    "stroke-miterlimit",
    "fill-rule",
    "fill-opacity",
    "stroke-opacity",
    "clip-rule",
    "opacity",
];

/// Elements that actually draw something. An icon that keeps none of these
/// after sanitizing is empty, and an empty icon is a silent failure.
const DRAWABLE: &[&str] = &[
    "path", "circle", "ellipse", "rect", "line", "polyline", "polygon",
];

/// Rewrites an untrusted SVG as one containing only allow-listed elements and
/// attributes.
///
/// This is the first of two layers. The second is the render path: a sanitized
/// icon is displayed through `<img src="data:…">`, which cannot execute script
/// or load external resources whatever this function misses. Never inline the
/// result — that would collapse both layers into this one.
pub fn sanitize_svg(input: &str) -> Result<String, IconError> {
    if input.len() > MAX_SVG_BYTES {
        return Err(IconError::TooLarge {
            size: input.len(),
            max: MAX_SVG_BYTES,
        });
    }

    let mut reader = Reader::from_str(input);
    let mut writer = Writer::new(Vec::new());

    // Depth of the disallowed subtree currently being skipped. Skipping the
    // whole subtree, rather than just the element, is what keeps a
    // `<script>`'s text content out of the output.
    let mut skip_depth = 0usize;

    // Nesting depth over *every* element, allowed or not — independent of
    // `skip_depth` — so we always know whether we are looking at the
    // document's single top-level element or at something nested inside it.
    let mut depth = 0usize;
    let mut root_seen = false;
    let mut root_is_svg = false;
    let mut extra_root = false;
    let mut saw_drawable = false;

    loop {
        let event = reader
            .read_event()
            .map_err(|e| IconError::Malformed(e.to_string()))?;

        match event {
            Event::Eof => break,

            Event::Start(e) => {
                let name = local_name(e.name().as_ref());
                let is_root = depth == 0;
                if is_root {
                    if root_seen {
                        extra_root = true;
                    }
                    root_seen = true;
                    if name == "svg" {
                        root_is_svg = true;
                    }
                }
                depth += 1;

                if skip_depth > 0 {
                    skip_depth += 1;
                    continue;
                }
                if !ALLOWED_ELEMENTS.contains(&name.as_str()) {
                    skip_depth = 1;
                    continue;
                }
                if DRAWABLE.contains(&name.as_str()) {
                    saw_drawable = true;
                }
                let mut start = filter_attributes(&name, &e)?;
                if is_root && name == "svg" {
                    // Emitted unconditionally, never taken from the input: a
                    // sanitized icon must parse as an SVG document when later
                    // loaded from a `data:image/svg+xml` URI, and a root
                    // element in no namespace is not an SVG element there.
                    // `xmlns` itself is not on `ALLOWED_ATTRS`, so any
                    // input-supplied value was already dropped above — this
                    // does not let input choose its own namespace.
                    start.push_attribute(("xmlns", "http://www.w3.org/2000/svg"));
                }
                writer
                    .write_event(Event::Start(start))
                    .map_err(|e| IconError::Malformed(e.to_string()))?;
            }

            Event::Empty(e) => {
                let name = local_name(e.name().as_ref());
                let is_root = depth == 0;
                if is_root {
                    if root_seen {
                        extra_root = true;
                    }
                    root_seen = true;
                    if name == "svg" {
                        root_is_svg = true;
                    }
                }
                // A self-closing element opens and closes within this single
                // event, so it never changes `depth`.

                if skip_depth > 0 {
                    continue;
                }
                if !ALLOWED_ELEMENTS.contains(&name.as_str()) {
                    continue;
                }
                if DRAWABLE.contains(&name.as_str()) {
                    saw_drawable = true;
                }
                let mut elem = filter_attributes(&name, &e)?;
                if is_root && name == "svg" {
                    elem.push_attribute(("xmlns", "http://www.w3.org/2000/svg"));
                }
                writer
                    .write_event(Event::Empty(elem))
                    .map_err(|e| IconError::Malformed(e.to_string()))?;
            }

            Event::End(e) => {
                depth = depth.saturating_sub(1);
                if skip_depth > 0 {
                    skip_depth -= 1;
                    continue;
                }
                let name = local_name(e.name().as_ref());
                if !ALLOWED_ELEMENTS.contains(&name.as_str()) {
                    continue;
                }
                writer
                    .write_event(Event::End(BytesEnd::new(name)))
                    .map_err(|e| IconError::Malformed(e.to_string()))?;
            }

            // Text outside the (single) root element — before it, between
            // sibling top-level nodes, or trailing after it closes — is
            // dropped rather than copied, so the output can never end up
            // with more top-level content than the one root element it is
            // supposed to have.
            Event::Text(t) if skip_depth == 0 && depth > 0 => {
                // Decoded, then re-escaped from the decoded string, rather
                // than the raw bytes forwarded verbatim. Forwarding raw bytes
                // would let an entity undefined once its declaring DOCTYPE is
                // stripped (e.g. `&xxe;`) — or a bare `&` that was never a
                // valid entity reference to begin with — sail through as
                // literal text that is fatal to any consumer that re-parses
                // the output, such as a real `data:image/svg+xml` loader.
                // Decoding here fails closed on exactly that; `BytesText::new`
                // re-escapes the decoded text for output.
                let decoded = t
                    .unescape()
                    .map_err(|e| IconError::Malformed(e.to_string()))?;
                writer
                    .write_event(Event::Text(BytesText::new(&decoded)))
                    .map_err(|e| IconError::Malformed(e.to_string()))?;
            }

            // Comments, processing instructions, doctypes and CDATA are dropped
            // outright. None of them carry anything an icon needs, and CDATA is
            // a classic way to smuggle a script payload past a naive filter.
            _ => {}
        }
    }

    // Anything other than exactly one root element named `svg` is rejected:
    // a second top-level element (two sibling roots) or a root whose local
    // name isn't `svg` (e.g. a `<g>` wrapping a nested `<svg>`) would leave
    // the caller no single well-formed SVG document to trust.
    if !root_seen || !root_is_svg || extra_root {
        return Err(IconError::NotAnSvg);
    }
    if !saw_drawable {
        return Err(IconError::NothingDrawable);
    }

    String::from_utf8(writer.into_inner()).map_err(|e| IconError::Malformed(e.to_string()))
}

/// Strips any namespace prefix, so `svg:path` and `path` are treated alike and
/// a prefix cannot be used to slip an element past the allow-list.
fn local_name(raw: &[u8]) -> String {
    let name = String::from_utf8_lossy(raw);
    match name.rsplit_once(':') {
        Some((_, local)) => local.to_string(),
        None => name.to_string(),
    }
}

/// Rebuilds a start tag carrying only allow-listed attributes whose values look
/// inert.
fn filter_attributes(name: &str, e: &BytesStart<'_>) -> Result<BytesStart<'static>, IconError> {
    let mut out = BytesStart::new(name.to_string());
    // Names already emitted for this element. `.with_checks(false)` below
    // means the reader will not reject a duplicate attribute name itself, and
    // a duplicate is not well-formed XML: reproducing it in the output would
    // leave the caller no single well-formed document to trust. Compared
    // case-insensitively, consistent with the allow-list check just below.
    let mut seen = Vec::new();

    for attr in e.attributes().with_checks(false) {
        let attr = attr.map_err(|err| IconError::Malformed(err.to_string()))?;
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();

        // Compared case-insensitively because SVG attribute names are
        // case-sensitive but hostile input is not obliged to be tidy.
        if !ALLOWED_ATTRS.iter().any(|a| a.eq_ignore_ascii_case(&key)) {
            continue;
        }

        let key_lower = key.to_ascii_lowercase();
        if seen.contains(&key_lower) {
            continue;
        }
        seen.push(key_lower);

        // Decoded (XML entities resolved) before the scan below, so the scan
        // sees the same text a real parser will. Scanning the raw bytes let an
        // XML entity spell "url(" or "javascript:" without the literal
        // substring ever appearing in the source.
        let value = attr
            .unescape_value()
            .map_err(|e| IconError::Malformed(e.to_string()))?
            .into_owned();

        // No allow-listed SVG presentation attribute needs a backslash. `fill`
        // and `stroke` are CSS-parsed presentation attributes, and CSS Syntax
        // Level 3 lets an identifier consume a backslash escape (literal or
        // numeric) while it is tokenized — so `\75 rl(`, `\000075rl(` and
        // `u\72 l(` all tokenize as the ident `url` followed by `(`, a
        // url-token, even though the literal substring "url(" never appears
        // for the scan below to catch. Rather than reimplement CSS ident-escape
        // parsing here, reject the escape mechanism outright.
        if value.contains('\\') {
            continue;
        }

        let lowered = value.to_ascii_lowercase();
        // `url(…)` can reach an external resource or a filter; a `javascript:`
        // scheme is self-explanatory. Both are dropped even on an allowed
        // attribute, because the attribute name alone does not make a value safe.
        if lowered.contains("url(") || lowered.contains("javascript:") {
            continue;
        }

        out.push_attribute((key.as_str(), value.as_str()));
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
        <path d="M3 6h18" stroke="currentColor" stroke-width="2"/>
        <circle cx="12" cy="12" r="4"/>
    </svg>"#;

    /// Re-parses `xml`, resolving every entity reference and rejecting a
    /// duplicate attribute, the way a real consumer — e.g. a
    /// `data:image/svg+xml` loader — would. This is what actually proves
    /// "the output is a single well-formed document"; a substring assertion
    /// cannot catch an unresolved entity, a bare `&`, or a duplicate
    /// attribute, all of which a naive scan happily lets through.
    fn assert_reparses_as_well_formed_xml(xml: &str) {
        let mut reader = Reader::from_str(xml);
        loop {
            let event = reader
                .read_event()
                .unwrap_or_else(|e| panic!("output did not re-parse ({e}): {xml}"));
            match event {
                Event::Eof => break,
                Event::Start(ref e) | Event::Empty(ref e) => {
                    // Default `Attributes` checks (no `with_checks(false)`
                    // here) reject a duplicate attribute name on their own.
                    for attr in e.attributes() {
                        let attr = attr.unwrap_or_else(|err| {
                            panic!("attribute error on re-parse ({err}): {xml}")
                        });
                        attr.unescape_value().unwrap_or_else(|err| {
                            panic!("unresolved entity in an attribute on re-parse ({err}): {xml}")
                        });
                    }
                }
                Event::Text(t) => {
                    t.unescape().unwrap_or_else(|err| {
                        panic!("unresolved entity in text on re-parse ({err}): {xml}")
                    });
                }
                _ => {}
            }
        }
    }

    #[test]
    fn decodes_numeric_entities_before_scanning_so_they_cannot_smuggle_url() {
        // "&#117;" is the numeric character reference for 'u'. Scanning the
        // raw, un-decoded attribute bytes would miss the literal substring
        // "url(" here. Attributes are now decoded (`Attribute::unescape_value`)
        // before the url(/javascript: scan runs, so this resolves to the
        // literal value "url(#evil)" pre-scan and is dropped outright — not
        // merely neutralized by the writer's own escaping on the way back out.
        let input = r##"<svg viewBox="0 0 1 1"><path d="M0 0" fill="&#117;rl(#evil)"/></svg>"##;
        let out = sanitize_svg(input).expect("path survives");
        assert!(!out.contains("evil"));
        assert!(!out.contains("fill="));
        assert!(out.contains("M0 0"));
    }

    #[test]
    fn drops_css_ident_escape_obfuscated_url_variants() {
        // CSS Syntax Level 3 lets an identifier consume a backslash escape —
        // literal (`\75`) or numeric with a trailing space (`\000075`) — while
        // it is tokenized. `fill` and `stroke` are CSS-parsed presentation
        // attributes, so a real renderer resolves each of these to the ident
        // `url` followed by `(`, a url-token, even though the literal
        // substring "url(" is never present in the raw XML text for the
        // scan to catch. None of these contain an XML metacharacter, so
        // nothing re-escapes the backslash the way an entity's '&' does.
        for value in [
            r"\75 rl(#evil)",
            r"\000075rl(#evil)",
            r"u\72 l(#evil)",
            r"\55 RL(#evil)",
        ] {
            let input = format!(r#"<svg viewBox="0 0 1 1"><path d="M0 0" fill="{value}"/></svg>"#);
            let out = sanitize_svg(&input).expect("path survives");
            assert!(
                !out.contains("evil"),
                "value {value:?} leaked into output: {out}"
            );
            assert!(out.contains("M0 0"));
        }
    }

    #[test]
    fn decoding_attribute_values_round_trips_ordinary_entities_correctly() {
        // "&#35;" is the numeric character reference for '#' — a legal, inert
        // spelling of a hex colour. Decoding before scanning must not mangle
        // it: it should come out as the literal colour, not the still-escaped
        // entity text (which would double-escape on any later re-sanitize).
        let input = r##"<svg viewBox="0 0 1 1"><path d="M0 0" fill="&#35;fff"/></svg>"##;
        let out = sanitize_svg(input).expect("path survives");
        assert!(out.contains(r##"fill="#fff""##));
    }

    #[test]
    fn keeps_a_well_formed_icon() {
        let out = sanitize_svg(GOOD).expect("a clean icon must survive");
        assert!(out.contains("<svg"));
        assert!(out.contains("viewBox"));
        assert!(out.contains("M3 6h18"));
        assert!(out.contains("<circle"));
        // The root must carry the SVG namespace unconditionally — without it,
        // a `data:image/svg+xml` payload parses with a root element in no
        // namespace, which is not an SVG element, and the icon renders as
        // nothing.
        assert!(out.contains(r#"xmlns="http://www.w3.org/2000/svg""#));
    }

    #[test]
    fn rejects_a_non_svg_root_even_when_svg_appears_nested_inside_it() {
        let input = r#"<g><svg viewBox="0 0 1 1"><path d="M0 0"/></svg></g>"#;
        assert!(matches!(sanitize_svg(input), Err(IconError::NotAnSvg)));
    }

    #[test]
    fn rejects_two_sibling_svg_roots() {
        let input = r#"<svg viewBox="0 0 1 1"><path d="M0 0"/></svg><svg viewBox="0 0 1 1"><path d="M1 1"/></svg>"#;
        assert!(matches!(sanitize_svg(input), Err(IconError::NotAnSvg)));
    }

    #[test]
    fn drops_trailing_text_after_the_root_closes_instead_of_rejecting_it() {
        let input = r#"<svg viewBox="0 0 1 1"><path d="M0 0"/></svg>trailing garbage"#;
        let out = sanitize_svg(input).expect("a single well-formed root still sanitizes");
        assert!(!out.contains("trailing"));
        assert!(out.contains("M0 0"));
    }

    #[test]
    fn a_self_closing_root_with_no_children_has_nothing_drawable() {
        // Previously misreported as `NotAnSvg` because the old `saw_svg` flag
        // was only ever set from the `Start` arm, never `Empty`. With root
        // tracking covering both event forms, this now gets the accurate
        // error: the root is a valid, singular `<svg>`, it just draws nothing.
        let input = r#"<svg viewBox="0 0 1 1"/>"#;
        assert!(matches!(
            sanitize_svg(input),
            Err(IconError::NothingDrawable)
        ));
    }

    #[test]
    fn strips_a_script_element_and_its_contents() {
        let input = r#"<svg viewBox="0 0 1 1"><script>fetch('/x')</script><path d="M0 0"/></svg>"#;
        let out = sanitize_svg(input).expect("still has a path");
        assert!(!out.contains("script"));
        assert!(!out.contains("fetch"));
        assert!(out.contains("M0 0"));
    }

    #[test]
    fn strips_event_handlers() {
        let input =
            r#"<svg viewBox="0 0 1 1" onload="alert(1)"><path d="M0 0" onclick="x()"/></svg>"#;
        let out = sanitize_svg(input).expect("path survives");
        assert!(!out.contains("onload"));
        assert!(!out.contains("onclick"));
        assert!(!out.contains("alert"));
    }

    #[test]
    fn strips_hrefs_including_the_xlink_form() {
        let input = r#"<svg viewBox="0 0 1 1"><path d="M0 0" href="javascript:alert(1)" xlink:href="http://evil/x.svg"/></svg>"#;
        let out = sanitize_svg(input).expect("path survives");
        assert!(!out.contains("href"));
        assert!(!out.contains("evil"));
        assert!(!out.contains("javascript"));
    }

    #[test]
    fn strips_foreign_object_use_and_image() {
        let input = r##"<svg viewBox="0 0 1 1">
            <foreignObject><div>hi</div></foreignObject>
            <use href="#x"/>
            <image href="http://evil/x.png"/>
            <path d="M0 0"/>
        </svg>"##;
        let out = sanitize_svg(input).expect("path survives");
        assert!(!out.contains("foreignObject"));
        assert!(!out.contains("<use"));
        assert!(!out.contains("<image"));
        assert!(!out.contains("evil"));
        assert!(out.contains("M0 0"));
    }

    #[test]
    fn strips_style_elements() {
        let input = r#"<svg viewBox="0 0 1 1"><style>@import url(http://evil/x.css);</style><path d="M0 0"/></svg>"#;
        let out = sanitize_svg(input).expect("path survives");
        assert!(!out.contains("style"));
        assert!(!out.contains("evil"));
    }

    #[test]
    fn drops_attribute_values_carrying_url_or_javascript() {
        let input = r#"<svg viewBox="0 0 1 1"><path d="M0 0" fill="url(#evil)" stroke="JavaScript:alert(1)"/></svg>"#;
        let out = sanitize_svg(input).expect("path survives");
        assert!(!out.contains("url("));
        assert!(!out.to_lowercase().contains("javascript"));
        assert!(out.contains("M0 0"));
    }

    #[test]
    fn drops_cdata_payloads() {
        let input =
            r#"<svg viewBox="0 0 1 1"><path d="M0 0"/><![CDATA[<script>alert(1)</script>]]></svg>"#;
        let out = sanitize_svg(input).expect("path survives");
        assert!(!out.contains("alert"));
        assert!(!out.contains("script"));
    }

    #[test]
    fn rejects_input_over_the_size_cap() {
        let huge = format!(
            r#"<svg viewBox="0 0 1 1"><path d="{}"/></svg>"#,
            "M0 0 ".repeat(MAX_SVG_BYTES / 4)
        );
        assert!(matches!(
            sanitize_svg(&huge),
            Err(IconError::TooLarge { .. })
        ));
    }

    #[test]
    fn rejects_something_that_is_not_an_svg() {
        let input = r#"<html><body>nope</body></html>"#;
        assert!(matches!(sanitize_svg(input), Err(IconError::NotAnSvg)));
    }

    #[test]
    fn rejects_an_svg_with_nothing_drawable_left() {
        let input = r#"<svg viewBox="0 0 1 1"><script>alert(1)</script></svg>"#;
        assert!(matches!(
            sanitize_svg(input),
            Err(IconError::NothingDrawable)
        ));
    }

    #[test]
    fn rejects_malformed_xml() {
        let input = r#"<svg viewBox="0 0 1 1"><path d="M0 0"></svg>"#;
        assert!(sanitize_svg(input).is_err());
    }

    #[test]
    fn rejects_an_undefined_entity_instead_of_emitting_it_raw() {
        // "&xxe;" is undefined once its declaring DOCTYPE is stripped, which
        // it always is. Forwarding it as raw text (the old behavior) would
        // produce output that is fatal for any consumer that re-parses it —
        // a real `data:image/svg+xml` loader raises "unrecognized entity
        // 'xxe'". Failing closed here, instead of emitting it, is what keeps
        // "the output is a single well-formed document" true rather than
        // merely usually true.
        let input = r#"<svg viewBox="0 0 1 1"><path d="M0 0"/><title>&xxe;</title></svg>"#;
        assert!(matches!(sanitize_svg(input), Err(IconError::Malformed(_))));
    }

    #[test]
    fn rejects_a_bare_ampersand_in_text_instead_of_emitting_it_raw() {
        // A bare '&' not part of a valid `&name;` or `&#N;` reference is not
        // well-formed XML text either. The same decode step that closes the
        // undefined-entity case above closes this one too: `unescape()` fails
        // on it just as it fails on an unknown named entity.
        let input = r#"<svg viewBox="0 0 1 1"><path d="M0 0"/><title>A & B</title></svg>"#;
        assert!(matches!(sanitize_svg(input), Err(IconError::Malformed(_))));
    }

    #[test]
    fn drops_a_duplicate_attribute_and_the_result_is_well_formed() {
        let input = r#"<svg viewBox="0 0 1 1"><path d="M0 0" d="M9 9"/></svg>"#;
        let out = sanitize_svg(input).expect("the first occurrence of a duplicate attribute wins");
        assert_reparses_as_well_formed_xml(&out);
        assert!(out.contains("M0 0"));
        assert!(!out.contains("M9 9"));
    }

    #[test]
    fn a_realistic_two_tone_icon_reparses_as_well_formed_xml() {
        // A lucide-style icon: a wrapping <g>, several drawable primitives,
        // ordinary presentation attributes, no funny business. The re-parse
        // is what actually proves "well-formed" — a substring check on GOOD
        // above would not have caught any of the three bugs this round of
        // review found.
        let input = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <g stroke-linecap="round" stroke-linejoin="round">
                <path d="M12 2v20"/>
                <circle cx="12" cy="12" r="10"/>
            </g>
        </svg>"#;
        let out = sanitize_svg(input).expect("a clean icon must survive");
        assert_reparses_as_well_formed_xml(&out);
    }

    #[test]
    fn keeps_inert_presentation_attributes() {
        let input = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
            <path d="M1 1 L2 2" stroke-dasharray="4 2" stroke-dashoffset="1"
                  stroke-miterlimit="2" fill-opacity="0.5" stroke-opacity="0.25"/>
        </svg>"#;
        let out = sanitize_svg(input).expect("well-formed icon should survive");
        for attr in [
            "stroke-dasharray",
            "stroke-dashoffset",
            "stroke-miterlimit",
            "fill-opacity",
            "stroke-opacity",
        ] {
            assert!(out.contains(attr), "sanitizer dropped {attr}: {out}");
        }
        assert_reparses_as_well_formed_xml(&out);
    }

    #[test]
    fn still_rejects_url_values_in_the_newly_allowed_attributes() {
        let input = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
            <path d="M1 1 L2 2" fill-opacity="url(#evil)"/>
        </svg>"#;
        let out = sanitize_svg(input).expect("should sanitize rather than fail");
        assert!(
            !out.contains("url("),
            "url() survived in an allowed attribute: {out}"
        );
    }
}
