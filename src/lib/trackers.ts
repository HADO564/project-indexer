// Generic helpers over `Tracker` that work for any variant — including ones
// added after this file was written. `Tracker`'s serde shape (see
// api/types.ts) makes the variant name the tracker's single object key when
// it carries data (`{ Git: {...} }`), or the bare string itself when it
// doesn't (`"Unity"`), so both the label and the field list can be read off
// that shape directly instead of switching on known variant names.
import type { Tracker } from "./api/types";

export function trackerKind(tracker: Tracker): string {
  return typeof tracker === "string" ? tracker : Object.keys(tracker)[0];
}

function trackerPayload(tracker: Tracker): Record<string, unknown> | null {
  if (typeof tracker === "string") return null;
  const kind = trackerKind(tracker);
  return (tracker as Record<string, unknown>)[kind] as Record<string, unknown>;
}

export interface TrackerField {
  label: string;
  value: string;
  isLink: boolean;
}

function humanizeKey(key: string): string {
  const spaced = key.replace(/_/g, " ");
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

function formatValue(value: unknown): string | null {
  if (value === null || value === undefined) return null;
  if (typeof value === "boolean") return value ? "Yes" : "No";
  if (Array.isArray(value)) return value.length > 0 ? value.join(", ") : null;
  const str = String(value);
  return str.length > 0 ? str : null;
}

// Field list for a tracker's detail view: every non-empty key in its
// payload, humanized into a label/value pair. A bare unit-variant tracker
// (no payload yet) yields an empty list rather than an error.
export function trackerFields(tracker: Tracker): TrackerField[] {
  const payload = trackerPayload(tracker);
  if (!payload) return [];

  const fields: TrackerField[] = [];
  for (const [key, value] of Object.entries(payload)) {
    const formatted = formatValue(value);
    if (formatted === null) continue;

    fields.push({
      label: humanizeKey(key),
      value: formatted,
      isLink: /url$/i.test(key) && /^https?:\/\//i.test(formatted),
    });
  }
  return fields;
}
