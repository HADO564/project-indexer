use quick_xml::events::{BytesEnd, BytesStart, Event};
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
    "fill-rule",
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
    let mut saw_svg = false;
    let mut saw_drawable = false;

    loop {
        let event = reader
            .read_event()
            .map_err(|e| IconError::Malformed(e.to_string()))?;

        match event {
            Event::Eof => break,

            Event::Start(e) => {
                let name = local_name(e.name().as_ref());
                if skip_depth > 0 {
                    skip_depth += 1;
                    continue;
                }
                if !ALLOWED_ELEMENTS.contains(&name.as_str()) {
                    skip_depth = 1;
                    continue;
                }
                if name == "svg" {
                    saw_svg = true;
                }
                if DRAWABLE.contains(&name.as_str()) {
                    saw_drawable = true;
                }
                writer
                    .write_event(Event::Start(filter_attributes(&name, &e)?))
                    .map_err(|e| IconError::Malformed(e.to_string()))?;
            }

            Event::Empty(e) => {
                if skip_depth > 0 {
                    continue;
                }
                let name = local_name(e.name().as_ref());
                if !ALLOWED_ELEMENTS.contains(&name.as_str()) {
                    continue;
                }
                if DRAWABLE.contains(&name.as_str()) {
                    saw_drawable = true;
                }
                writer
                    .write_event(Event::Empty(filter_attributes(&name, &e)?))
                    .map_err(|e| IconError::Malformed(e.to_string()))?;
            }

            Event::End(e) => {
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

            Event::Text(t) if skip_depth == 0 => {
                writer
                    .write_event(Event::Text(t))
                    .map_err(|e| IconError::Malformed(e.to_string()))?;
            }

            // Comments, processing instructions, doctypes and CDATA are dropped
            // outright. None of them carry anything an icon needs, and CDATA is
            // a classic way to smuggle a script payload past a naive filter.
            _ => {}
        }
    }

    if !saw_svg {
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

    for attr in e.attributes().with_checks(false) {
        let attr = attr.map_err(|err| IconError::Malformed(err.to_string()))?;
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();

        // Compared case-insensitively because SVG attribute names are
        // case-sensitive but hostile input is not obliged to be tidy.
        if !ALLOWED_ATTRS.iter().any(|a| a.eq_ignore_ascii_case(&key)) {
            continue;
        }

        let value = String::from_utf8_lossy(&attr.value).to_string();
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

    #[test]
    fn escapes_numeric_entities_so_they_cannot_smuggle_url_past_the_raw_text_scan() {
        // "&#117;" is the numeric character reference for 'u'. Our url(/javascript:
        // check runs on the raw, un-unescaped attribute bytes, so a value spelled
        // this way would not contain the literal substring "url(" at scan time.
        // The property this test locks in: the writer re-escapes the leading '&'
        // when the attribute is re-emitted, so a real XML parser's single decode
        // pass yields the literal text "&#117;rl(#evil)", never "url(#evil)". If
        // this ever stopped holding, the substring check above would be a false
        // sense of safety.
        let input = r##"<svg viewBox="0 0 1 1"><path d="M0 0" fill="&#117;rl(#evil)"/></svg>"##;
        let out = sanitize_svg(input).expect("path survives");
        assert!(!out.contains(r#"fill="url(#evil)""#));
        assert!(out.contains("&amp;#117;rl(#evil)"));
    }

    #[test]
    fn keeps_a_well_formed_icon() {
        let out = sanitize_svg(GOOD).expect("a clean icon must survive");
        assert!(out.contains("<svg"));
        assert!(out.contains("viewBox"));
        assert!(out.contains("M3 6h18"));
        assert!(out.contains("<circle"));
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
}
