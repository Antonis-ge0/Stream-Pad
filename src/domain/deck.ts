export type DeckConfig = {
  activeProfileId: string;
  profiles: Profile[];
  settings: DeckSettings;
};

export type DeckSettings = {
  launchOnStartup: boolean;
  startMinimizedToTray: boolean;
  confirmBeforeDelete: boolean;
  defaultDeckView: "tile" | "list";
  buttonTriggerMode: "singleClick" | "doubleClick";
};

export type Profile = {
  id: string;
  name: string;
  buttons: DeckButton[];
};

export type DeckButton = {
  id: string;
  label: string;
  icon?: string | null;
  actions: DeckAction[];
};

export type DeckAction =
  | { type: "openUrl"; url: string }
  | { type: "launchApp"; path: string; args?: string[] }
  | { type: "playSound"; sound: string }
  | { type: "openFolder"; path: string };

export type ImportedButtonData = {
  label: string;
  icon?: string | null;
  actions: DeckAction[];
};

export type ConfirmDeleteState =
  | { type: "button"; id: string; name: string }
  | { type: "buttons"; ids: string[]; count: number }
  | { type: "profile"; id: string; name: string }
  | null;

export type AppError = {
  title: string;
  message: string;
};
