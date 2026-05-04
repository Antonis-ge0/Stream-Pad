import { invoke } from "@tauri-apps/api/core";
import type { DeckConfig } from "@/domain/deck";

export async function loadDeckConfig() {
  return invoke<DeckConfig>("load_config");
}

export async function saveDeckConfig(config: DeckConfig) {
  return invoke<void>("save_config", { config });
}
