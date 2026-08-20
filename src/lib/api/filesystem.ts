import { invoke } from "@tauri-apps/api/core";
import { toError } from "./errors";

export async function deleteDirectory(path: string): Promise<void> {
  try {
    await invoke<void>("delete_directory", { path });
  } catch (err) {
    throw toError(err);
  }
}
