import { describe, expect, it } from "vitest";
import {
  DEFAULT_SCAN_SETTINGS,
  restoreScanSettings,
  toScanRequest,
  type ScanSettings,
} from "./scanSettings";

const KINDS = ["git", "unreal"];

const stored: ScanSettings = {
  scanRoot: "/home/user/projects",
  mode: "deep",
  depth: 3,
  detectors: ["git"],
  includeIgnored: true,
};

describe("restoreScanSettings", () => {
  it("returns defaults when nothing is stored", () => {
    expect(restoreScanSettings(null, KINDS)).toEqual({
      ...DEFAULT_SCAN_SETTINGS,
      detectors: KINDS,
    });
  });

  it("round-trips stored settings", () => {
    expect(restoreScanSettings(JSON.stringify(stored), KINDS)).toEqual(stored);
  });

  // Hand-edited, written by an older build, or truncated. A broken preference
  // is not worth an error — it falls back like an absent one.
  it("falls back to defaults on unparseable json", () => {
    expect(restoreScanSettings("{not json", KINDS)).toEqual({
      ...DEFAULT_SCAN_SETTINGS,
      detectors: KINDS,
    });
  });

  it("falls back on a value of the wrong shape", () => {
    expect(restoreScanSettings(JSON.stringify({ scanRoot: 42 }), KINDS)).toEqual({
      ...DEFAULT_SCAN_SETTINGS,
      detectors: KINDS,
    });
  });

  // A detector removed from the build must not stay ticked in a form that
  // cannot show it — the tick-list is built from what the binary registers.
  it("drops stored detectors the build no longer has", () => {
    const raw = JSON.stringify({ ...stored, detectors: ["git", "gone"] });
    expect(restoreScanSettings(raw, KINDS).detectors).toEqual(["git"]);
  });

  // Every ticked detector having been removed would mean a scan that silently
  // finds nothing, so fall back to all of them rather than none.
  it("falls back to every kind when none of the stored ones survive", () => {
    const raw = JSON.stringify({ ...stored, detectors: ["gone"] });
    expect(restoreScanSettings(raw, KINDS).detectors).toEqual(KINDS);
  });

  it("clamps a nonsensical depth", () => {
    const shallow = JSON.stringify({ ...stored, depth: 0 });
    const absurd = JSON.stringify({ ...stored, depth: 999 });
    expect(restoreScanSettings(shallow, KINDS).depth).toBe(1);
    expect(restoreScanSettings(absurd, KINDS).depth).toBe(10);
  });
});

describe("toScanRequest", () => {
  // ScanMode is an internally-tagged enum flattened into ScanRequest. A quick
  // scan omits the depth field simply because that is the honest shape of a
  // quick scan for the type it deserializes into — not because a stray key
  // would be rejected (it would not: nothing here denies unknown fields).
  it("omits depth for a quick scan", () => {
    const request = toScanRequest({ ...stored, mode: "quick" });
    expect(request).toEqual({
      scan_root: stored.scanRoot,
      mode: "quick",
      detectors: ["git"],
      include_ignored: true,
    });
    expect("depth" in request).toBe(false);
  });

  it("carries depth for a deep scan", () => {
    expect(toScanRequest(stored)).toEqual({
      scan_root: stored.scanRoot,
      mode: "deep",
      depth: 3,
      detectors: ["git"],
      include_ignored: true,
    });
  });

  // The modal binds depth to a bare <input type="number"> outside form
  // validation, so these are all reachable at submit time, not just
  // hypothetical. Rust's `ScanMode::Deep { depth: u32 }` rejects every one of
  // them with a raw serde error, so clamping has to happen on the way out.
  it("clamps a cleared depth (null) to the default for a deep scan", () => {
    const settings = { ...stored, mode: "deep" as const, depth: null as unknown as number };
    expect(toScanRequest(settings)).toMatchObject({ mode: "deep", depth: DEFAULT_SCAN_SETTINGS.depth });
  });

  it("clamps a NaN depth to the default for a deep scan", () => {
    const settings = { ...stored, mode: "deep" as const, depth: NaN };
    expect(toScanRequest(settings)).toMatchObject({ mode: "deep", depth: DEFAULT_SCAN_SETTINGS.depth });
  });

  it("truncates a fractional depth for a deep scan", () => {
    const settings = { ...stored, mode: "deep" as const, depth: 2.5 };
    expect(toScanRequest(settings)).toMatchObject({ mode: "deep", depth: 2 });
  });

  it("clamps a zero depth up to the minimum for a deep scan", () => {
    const settings = { ...stored, mode: "deep" as const, depth: 0 };
    expect(toScanRequest(settings)).toMatchObject({ mode: "deep", depth: 1 });
  });

  it("clamps an over-max depth down to the ceiling for a deep scan", () => {
    const settings = { ...stored, mode: "deep" as const, depth: 500 };
    expect(toScanRequest(settings)).toMatchObject({ mode: "deep", depth: 10 });
  });
});
