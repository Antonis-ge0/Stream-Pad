import { APP_CONFIG } from "@/config/appConfig";

export function isValidUrl(value: string) {
  try {
    const url = new URL(value);
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}

export function validateProfileName(name: string) {
  if (!name.trim()) {
    return "Profile name cannot be empty.";
  }

  if (name.trim().length > 40) {
    return "Profile name must be 40 characters or shorter.";
  }

  return null;
}

export function validateButtonLabel(label: string) {
  if (!label.trim()) {
    return "Button name cannot be empty.";
  }

  if (label.trim().length > 40) {
    return "Button name must be 40 characters or shorter.";
  }

  return null;
}

export function isSupportedAudioType(file: File) {
  return APP_CONFIG.supportedAudioTypes.includes(
    file.type as (typeof APP_CONFIG.supportedAudioTypes)[number],
  );
}
