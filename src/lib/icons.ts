// The bundled icon set: 25 line glyphs, each a list of SVG path `d` strings on
// a 0 0 24 24 viewBox, stroked in currentColor. Bundled icons are inline and
// therefore tintable, which is why a group's sidebar entry can take its
// group's colour and a project's icon can take the project's.
//
// Paths only — no <circle> or <rect> — so the renderer is a single {#each} and
// no component ever needs {@html}.
//
// An unknown name resolves to FALLBACK_ICON rather than failing. `core`
// deliberately does not own this list (it validates only that a group's icon
// name is non-empty), so a record naming an icon this build has never heard of
// is a normal state, not an error.

const GLYPHS: Record<string, readonly string[]> = {
  folder: ["M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"],
  briefcase: [
    "M3 9a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
    "M9 7V5a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2",
    "M3 13h18",
  ],
  code: ["M8 6l-5 6 5 6", "M16 6l5 6-5 6", "M14 4l-4 16"],
  terminal: ["M4 6l6 6-6 6", "M13 18h7"],
  gamepad: [
    "M7 8h10a5 5 0 0 1 5 5v1a4 4 0 0 1-7 2.6l-1-1H8l-1 1A4 4 0 0 1 2 14v-1a5 5 0 0 1 5-5z",
    "M6 12h3",
    "M7.5 10.5v3",
    "M16 11.5h.01",
    "M18 13.5h.01",
  ],
  palette: [
    "M12 3a9 9 0 1 0 0 18 2 2 0 0 0 1.6-3.2 2 2 0 0 1 1.6-3.2H18a3 3 0 0 0 3-3A9 9 0 0 0 12 3z",
    "M7.5 11.5h.01",
    "M10 8h.01",
    "M14.5 8h.01",
  ],
  book: ["M4 5a2 2 0 0 1 2-2h13v16H6a2 2 0 0 0-2 2z", "M6 17h13"],
  music: [
    "M9 18V6l10-2v12",
    "M6 18a3 3 0 1 0 3 3 3 3 0 0 0-3-3z",
    "M16 16a3 3 0 1 0 3 3 3 3 0 0 0-3-3z",
  ],
  camera: [
    "M3 9a2 2 0 0 1 2-2h2l1.5-2h7L17 7h2a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
    "M12 10a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7z",
  ],
  film: [
    "M3 4h18v16H3z",
    "M7 4v16",
    "M17 4v16",
    "M3 12h18",
    "M3 8h4",
    "M17 8h4",
    "M3 16h4",
    "M17 16h4",
  ],
  flask: ["M9 3v6L4 19a2 2 0 0 0 1.8 2h12.4A2 2 0 0 0 20 19l-5-10V3", "M8 3h8", "M7 14h10"],
  rocket: [
    "M12 2c3 2.5 5 6.5 5 11l-2.5 3h-5L7 13c0-4.5 2-8.5 5-11z",
    "M9.5 19c-1 1.5-1 3 0 3s2-1 2-3",
    "M14.5 19c1 1.5 1 3 0 3s-2-1-2-3",
    "M12 10h.01",
  ],
  globe: [
    "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z",
    "M3 12h18",
    "M12 3c2.5 2.6 3.8 5.7 3.8 9S14.5 18.4 12 21c-2.5-2.6-3.8-5.7-3.8-9S9.5 5.6 12 3z",
  ],
  database: [
    "M4 6c0-1.7 3.6-3 8-3s8 1.3 8 3-3.6 3-8 3-8-1.3-8-3z",
    "M4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6",
    "M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3",
  ],
  server: ["M3 4h18v6H3z", "M3 14h18v6H3z", "M7 7h.01", "M7 17h.01"],
  cpu: [
    "M7 7h10v10H7z",
    "M4 9h3",
    "M4 15h3",
    "M17 9h3",
    "M17 15h3",
    "M9 4v3",
    "M15 4v3",
    "M9 17v3",
    "M15 17v3",
  ],
  box: ["M12 3l8 4.5v9L12 21l-8-4.5v-9z", "M4 7.5l8 4.5 8-4.5", "M12 12v9"],
  layers: ["M12 3l9 5-9 5-9-5z", "M3 13l9 5 9-5", "M3 17l9 5 9-5"],
  pen: ["M4 20l4-1 11-11a2.1 2.1 0 0 0-3-3L5 16z", "M14 6l4 4"],
  wrench: ["M20 6a5 5 0 0 1-6.6 6.6L6 20a2.1 2.1 0 0 1-3-3l7.4-7.4A5 5 0 0 1 17 4z"],
  heart: ["M12 20S3.5 14.5 3.5 9A4.5 4.5 0 0 1 12 6.8 4.5 4.5 0 0 1 20.5 9c0 5.5-8.5 11-8.5 11z"],
  star: ["M12 3.5l2.6 5.3 5.8.8-4.2 4.1 1 5.8-5.2-2.7-5.2 2.7 1-5.8L3.6 9.6l5.8-.8z"],
  flag: ["M5 21V4", "M5 4h11l-2 3.5L16 11H5"],
  home: ["M4 11l8-7 8 7", "M6 9.5V20h12V9.5", "M10 20v-5h4v5"],
  trash: [
    "M3 6h18",
    "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2",
    "M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6",
    "M10 11v6",
    "M14 11v6",
  ],
};

export const ICON_NAMES: readonly string[] = Object.keys(GLYPHS);

export const FALLBACK_ICON = "folder";

const CUSTOM_PREFIX = "custom:";

// A custom icon is stored as "custom:<slug>". It is never drawn inline, so
// iconPaths falls back for it — the caller checks isCustomIcon first and
// renders an <img> instead.
export function isCustomIcon(name: string | null | undefined): boolean {
  return typeof name === "string" && name.startsWith(CUSTOM_PREFIX);
}

export function customIconName(name: string): string {
  return name.startsWith(CUSTOM_PREFIX) ? name.slice(CUSTOM_PREFIX.length) : name;
}

export function iconPaths(name: string | null | undefined): readonly string[] {
  if (typeof name === "string" && !isCustomIcon(name)) {
    const paths = GLYPHS[name];
    if (paths) return paths;
  }
  return GLYPHS[FALLBACK_ICON];
}

// The one way a custom icon reaches the DOM. `core` returns sanitized SVG
// source rather than a data URI so it needs no base64 dependency for
// something the browser assembles in one expression.
//
// This is deliberately a data: URI for an <img>, never inlined markup. The
// sanitizer is the first security layer; an <img> is the second, and it
// cannot execute script or fetch anything regardless of what the first one
// missed. `img-src 'self' data:` is already in both content security
// policies. Reaching for {@html} here collapses two independent layers into
// one — do not.
export function customIconSrc(svg: string): string {
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}
