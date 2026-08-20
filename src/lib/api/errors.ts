// Tauri rejects with whatever the Rust side's Err(String) contains.
// Normalize that into a real Error so callers get consistent .message access.
export function toError(err: unknown): Error {
  if (err instanceof Error) return err;
  if (typeof err === "string") return new Error(err);
  return new Error(String(err));
}
