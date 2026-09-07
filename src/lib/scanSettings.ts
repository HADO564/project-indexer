import type { ScanRequest } from "./api/types";

// The last scan's settings, so the form comes up as you left it.
//
// This is the whole of "remembering a scan". A `scan_roots` table with its own
// ports, CRUD and a schema migration was designed and cut: the path was never
// the friction — anybody scanning ~/projects knows where ~/projects is — and a
// rescan cannot skip the disk regardless, since a project that appeared
// yesterday is discoverable only by looking. What is actually worth keeping is
// the settings, because a rescan that quietly ran depth 2 instead of depth 4
// gives a different answer with nothing on screen saying why.
//
// localStorage rather than projects.db for the same reason viewState.ts uses
// it: the database is a cross-app contract devmon attaches read-only, and one
// user's scan folder is none of its business.
export interface ScanSettings {
  scanRoot: string;
  mode: "quick" | "deep";
  depth: number;
  detectors: string[];
  includeIgnored: boolean;
}

const KEY = "pi.scanSettings";

// Depth is a user-chosen number, so it needs a floor and a ceiling. Ten is
// well past any sane project layout and keeps a typo from turning a scan into
// a full-disk walk that only MAX_DIRECTORIES ends.
const MIN_DEPTH = 1;
const MAX_DEPTH = 10;

// `detectors` is empty here and filled from the build's registered kinds by
// restoreScanSettings — this module does not know what detectors exist.
export const DEFAULT_SCAN_SETTINGS: ScanSettings = {
  scanRoot: "",
  mode: "quick",
  depth: 2,
  detectors: [],
  includeIgnored: false,
};

function clampDepth(value: unknown): number {
  if (typeof value !== "number" || !Number.isFinite(value)) return DEFAULT_SCAN_SETTINGS.depth;
  return Math.min(MAX_DEPTH, Math.max(MIN_DEPTH, Math.trunc(value)));
}

// Pure. `raw` is whatever was in storage — possibly written by an older build,
// possibly hand-edited, possibly absent. `availableKinds` is what this build
// registers, which is authoritative: a stored detector the binary no longer
// has cannot be shown as a tick, so it is dropped.
export function restoreScanSettings(raw: string | null, availableKinds: string[]): ScanSettings {
  const fallback: ScanSettings = { ...DEFAULT_SCAN_SETTINGS, detectors: [...availableKinds] };
  if (!raw) return fallback;

  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return fallback;
  }
  if (typeof parsed !== "object" || parsed === null) return fallback;

  const value = parsed as Partial<Record<keyof ScanSettings, unknown>>;
  if (typeof value.scanRoot !== "string") return fallback;

  const detectors = Array.isArray(value.detectors)
    ? value.detectors.filter((k): k is string => typeof k === "string" && availableKinds.includes(k))
    : [];

  return {
    scanRoot: value.scanRoot,
    mode: value.mode === "deep" ? "deep" : "quick",
    depth: clampDepth(value.depth),
    // Every stored detector having been removed would mean a scan that
    // silently finds nothing, so fall back to all of them rather than none.
    detectors: detectors.length > 0 ? detectors : [...availableKinds],
    includeIgnored: value.includeIgnored === true,
  };
}

// Pure. ScanMode is an internally-tagged serde enum flattened into
// ScanRequest, so a quick scan must carry no `depth` key at all — serde
// rejects unknown fields on the deny-unknown side of a flattened enum.
export function toScanRequest(settings: ScanSettings): ScanRequest {
  const base = {
    scan_root: settings.scanRoot,
    detectors: settings.detectors,
    include_ignored: settings.includeIgnored,
  };
  return settings.mode === "deep"
    ? { ...base, mode: "deep", depth: settings.depth }
    : { ...base, mode: "quick" };
}

// Storage access, split from the rules above so the rules are testable in
// vitest's node environment. Guarded because a browser with site data blocked
// throws on the accessor itself.
export function loadScanSettings(availableKinds: string[]): ScanSettings {
  try {
    const raw = typeof localStorage === "undefined" ? null : localStorage.getItem(KEY);
    return restoreScanSettings(raw, availableKinds);
  } catch {
    return restoreScanSettings(null, availableKinds);
  }
}

export function saveScanSettings(settings: ScanSettings): void {
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(KEY, JSON.stringify(settings));
  } catch {
    // A preference that cannot be saved is not worth an error banner.
  }
}
