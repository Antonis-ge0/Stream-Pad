import { invoke } from "@tauri-apps/api/core";
import type { ImportedButtonData } from "@/domain/deck";

export async function describeDroppedFile(path: string) {
  return invoke<ImportedButtonData>("describe_dropped_file", { path });
}
