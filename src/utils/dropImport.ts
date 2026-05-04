import type { ImportedButtonData } from "@/domain/deck";

const AUDIO_EXTENSIONS = new Set(["mp3", "wav", "ogg", "m4a", "flac", "aac"]);
const HTTP_URL_PATTERN = /\bhttps?:\/\/[^\s<>"']+/i;
const HOST_URL_PATTERN =
  /^(?:www\.)?[a-z0-9-]+(?:\.[a-z0-9-]+)+(?:\/[^\s<>"']*)?$/i;

function stripExtension(name: string) {
  return name.replace(/\.[^/.]+$/, "");
}

function extensionOf(name: string) {
  return name.split(".").pop()?.toLowerCase() ?? "";
}

function readFileAsDataUrl(file: File) {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader();

    reader.onload = () => resolve(reader.result as string);
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}

function cleanUrlCandidate(value: string) {
  return value.trim().replace(/^<|>$/g, "").replace(/[),.;]+$/g, "");
}

function normalizeUrl(value: string) {
  const candidate = cleanUrlCandidate(value);

  if (!candidate) {
    return null;
  }

  try {
    const parsed = new URL(candidate);

    if (parsed.protocol === "http:" || parsed.protocol === "https:") {
      return parsed.toString();
    }
  } catch {
    if (HOST_URL_PATTERN.test(candidate)) {
      return new URL(`https://${candidate}`).toString();
    }
  }

  return null;
}

function firstUrlFromText(text: string) {
  if (!text) {
    return null;
  }

  for (const line of text.split(/\r?\n/)) {
    const normalized = normalizeUrl(line);

    if (normalized) {
      return normalized;
    }
  }

  const match = text.match(HTTP_URL_PATTERN);

  if (match) {
    return normalizeUrl(match[0]);
  }

  return null;
}

function labelFromUrlDragText(text: string) {
  const [, label] = text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);

  return label || null;
}

function absoluteImageUrl(value: string, baseUrl: string) {
  try {
    return new URL(value, baseUrl).toString();
  } catch {
    return null;
  }
}

function urlDropFromHtml(html: string) {
  if (!html) {
    return null;
  }

  const parsed = new DOMParser().parseFromString(html, "text/html");
  const anchor = parsed.querySelector("a[href]");
  const href = anchor?.getAttribute("href");
  const url = href ? normalizeUrl(href) : null;

  if (!url) {
    return null;
  }

  const label =
    anchor?.textContent?.trim() ||
    parsed.querySelector("title")?.textContent?.trim() ||
    null;
  const iconSrc = parsed.querySelector("img[src]")?.getAttribute("src");
  const icon = iconSrc ? absoluteImageUrl(iconSrc, url) : null;

  return { url, label, icon };
}

function titleFromHtml(html: string) {
  if (!html) {
    return null;
  }

  const parsed = new DOMParser().parseFromString(html, "text/html");
  const anchorText = parsed.querySelector("a")?.textContent?.trim();
  const titleText = parsed.querySelector("title")?.textContent?.trim();

  return anchorText || titleText || null;
}

function labelFromUrl(url: string, html: string) {
  const title = titleFromHtml(html);

  if (title) {
    return title;
  }

  const parsed = new URL(url);
  return parsed.hostname.replace(/^www\./, "");
}

function faviconFromUrl(url: string) {
  const parsed = new URL(url);
  return `https://www.google.com/s2/favicons?domain=${parsed.hostname}&sz=64`;
}

async function importFromFile(file: File): Promise<ImportedButtonData | null> {
  const extension = extensionOf(file.name);

  if (!file.type.startsWith("audio/") && !AUDIO_EXTENSIONS.has(extension)) {
    return null;
  }

  return {
    label: stripExtension(file.name),
    icon: "\u{1F50A}",
    actions: [
      {
        type: "playSound",
        sound: await readFileAsDataUrl(file),
      },
    ],
  };
}

function importFromUrlDrop(dataTransfer: DataTransfer): ImportedButtonData | null {
  const uriList = dataTransfer.getData("text/uri-list");
  const mozUrl = dataTransfer.getData("text/x-moz-url");
  const mozUrlData = dataTransfer.getData("text/x-moz-url-data");
  const mozUrlDesc = dataTransfer.getData("text/x-moz-url-desc");
  const plainText = dataTransfer.getData("text/plain");
  const html = dataTransfer.getData("text/html");
  const htmlDrop = urlDropFromHtml(html);
  const url =
    firstUrlFromText(uriList) ??
    firstUrlFromText(mozUrlData) ??
    firstUrlFromText(mozUrl) ??
    htmlDrop?.url ??
    firstUrlFromText(plainText);

  if (!url) {
    return null;
  }

  const label =
    htmlDrop?.label ||
    mozUrlDesc.trim() ||
    labelFromUrlDragText(mozUrl) ||
    labelFromUrl(url, html);

  return {
    label,
    icon: htmlDrop?.icon ?? faviconFromUrl(url),
    actions: [
      {
        type: "openUrl",
        url,
      },
    ],
  };
}

export async function importFromDataTransfer(dataTransfer: DataTransfer) {
  const firstFile = dataTransfer.files.item(0);

  if (firstFile) {
    const fileImport = await importFromFile(firstFile);

    if (fileImport) {
      return fileImport;
    }
  }

  return importFromUrlDrop(dataTransfer);
}
