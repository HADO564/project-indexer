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
