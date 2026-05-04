import { invoke } from "@tauri-apps/api/core";

export async function triggerDeckButton(buttonId: string, profileId: string | null) {
  return invoke<void>("trigger_button", {
    buttonId,
    profileId,
  });
}
