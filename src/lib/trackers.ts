// Generic helpers over `Tracker` that work for any variant, including ones
// added after this file was written. Field *semantics* (is this a link? a
// path? a copyable id?) are inferred from the key name and value shape — see
// `inferType` — so no per-tracker-kind code lives here or in the UI. Naming a
// detector's `*Info` fields per the convention below is how a field gets an
// affordance:
//   *_url / *_root / *_path / *_dir  → link|path      *hash* / *commit* → code
//   arrays → chips                    booleans → flag (shown only when true)
import type { Tracker } from "./api/types";

export function trackerKind(tracker: Tracker): string {
  return typeof tracker === "string" ? tracker : Object.keys(tracker)[0];
}

function trackerPayload(tracker: Tracker): Record<string, unknown> | null {
  if (typeof tracker === "string") return null;
  const kind = trackerKind(tracker);
  return (tracker as Record<string, unknown>)[kind] as Record<string, unknown>;
}

export type FieldType = "text" | "code" | "link" | "path" | "chips" | "flag";

export interface TrackerField {
  label: string;
  type: FieldType;
  /** Display/copy text for text|code|link|path. Empty for chips|flag. */
  text: string;
  /** Chip values; empty otherwise. */
  items: string[];
}

function humanizeKey(key: string): string {
  const spaced = key.replace(/_/g, " ");
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

function inferType(key: string, value: unknown): FieldType | null {
  if (typeof value === "boolean") return "flag";
  if (Array.isArray(value)) return value.length > 0 ? "chips" : null;
  if (value === null || value === undefined || value === "") return null;
  const s = String(value);
  if (/^https?:\/\//i.test(s)) return "link";
  if (/^(git@|ssh:\/\/)/i.test(s)) return "code";
  if (/(^|_)(path|root|dir)$|directory/i.test(key)) return "path";
  if (/hash|commit/i.test(key)) return "code";
  return "text";
}

export function trackerFields(tracker: Tracker): TrackerField[] {
  const payload = trackerPayload(tracker);
  if (!payload) return [];

  const fields: TrackerField[] = [];
  for (const [key, value] of Object.entries(payload)) {
    const type = inferType(key, value);
    if (type === null) continue;

    if (type === "flag") {
      if (value === true) fields.push({ label: humanizeKey(key), type, text: "", items: [] });
      continue;
    }
    if (type === "chips") {
      fields.push({
        label: humanizeKey(key),
        type,
        text: "",
        items: (value as unknown[]).map(String),
      });
      continue;
    }
    fields.push({ label: humanizeKey(key), type, text: String(value), items: [] });
  }
  return fields;
}
