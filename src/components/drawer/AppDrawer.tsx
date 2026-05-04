import {
  Check,
  ChevronLeft,
  Download,
  HelpCircle,
  Info,
  LayoutGrid,
  List,
  MessageSquareMore,
  Moon,
  MousePointer2,
  MousePointerClick,
  Rocket,
  RefreshCw,
  Settings,
  Sun,
  X,
  type LucideIcon,
} from "lucide-react";
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { APP_CONFIG } from "@/config/appConfig";
import type { DeckSettings } from "@/domain/deck";
import type { Theme } from "@/hooks/useTheme";
import titleBarIcon from "@/assets/titlebar-icon.png";

type DrawerSection = "menu" | "settings" | "help" | "feedback" | "updates" | "about";
type UpdateStatus = "idle" | "checking" | "available" | "current" | "installing" | "error";
type FeedbackStatus = "idle" | "error";

const HELP_TIPS = [
  {
    title: "Run a button",
    description: "Left click a deck button to run its assigned action.",
  },
  {
    title: "Edit a button",
    description: "Right-click a button to select it without running, then edit it in the right panel.",
  },
  {
    title: "Reorder buttons",
    description: "Use the drag handle on each button to rearrange the deck layout.",
  },
  {
    title: "Switch views",
    description: "Toggle between Tile view and List view depending on how many buttons you want to scan at once.",
  },
  {
    title: "Select multiple buttons",
    description: "Use Select mode to choose several buttons, then duplicate or delete them together.",
  },
  {
    title: "Import with drag and drop",
    description: "Drop folders, sound files, website URLs, shortcuts, or .exe apps directly onto a button.",
  },
  {
    title: "Use profiles",
    description: "Create separate profiles for gaming, work, music, or any setup you want to switch between.",
  },
  {
    title: "Duplicate faster",
    description: "Duplicate a configured button when you want to reuse the same action and only tweak the label or icon.",
  },
] as const;

const HELP_TIPS_PER_PAGE = 5;

const DECK_VIEW_OPTIONS: {
  value: DeckSettings["defaultDeckView"];
  label: string;
  Icon: LucideIcon;
}[] = [
  { value: "tile", label: "Tile", Icon: LayoutGrid },
  { value: "list", label: "List", Icon: List },
];

const TRIGGER_MODE_OPTIONS: {
  value: DeckSettings["buttonTriggerMode"];
  label: string;
  Icon: LucideIcon;
}[] = [
  { value: "singleClick", label: "Single click", Icon: MousePointerClick },
  { value: "doubleClick", label: "Double click", Icon: MousePointer2 },
];

type AppDrawerProps = {
  onClose: () => void;
  onThemeToggle: () => void;
  onUpdateSettings: (settings: DeckSettings) => void | Promise<void>;
  settings: DeckSettings;
  theme: Theme;
};

export function AppDrawer({
  onClose,
  onThemeToggle,
  onUpdateSettings,
  settings,
  theme,
}: AppDrawerProps) {
  const [section, setSection] = useState<DrawerSection>("menu");
  const [availableUpdate, setAvailableUpdate] = useState<Update | null>(null);
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>("idle");
  const [updateMessage, setUpdateMessage] = useState("Check if a new version is available.");
  const [downloadProgress, setDownloadProgress] = useState<number | null>(null);
  const [currentHelpPage, setCurrentHelpPage] = useState(1);
  const [feedbackStatus, setFeedbackStatus] = useState<FeedbackStatus>("idle");
  const sectionTitle =
    section === "menu"
      ? "Options"
      : section === "settings"
        ? "General"
        : section === "help"
          ? "Help"
          : section === "feedback"
            ? "Share Feedback"
            : section === "updates"
              ? "Check for Updates"
              : "About";
  const helpPageCount = Math.ceil(HELP_TIPS.length / HELP_TIPS_PER_PAGE);
  const visibleHelpTips = HELP_TIPS.slice(
    (currentHelpPage - 1) * HELP_TIPS_PER_PAGE,
    currentHelpPage * HELP_TIPS_PER_PAGE,
  );

  function handleSectionChange(nextSection: DrawerSection) {
    setSection(nextSection);

    if (nextSection === "help") {
      setCurrentHelpPage(1);
    }

    if (nextSection === "feedback") {
      setFeedbackStatus("idle");
    }
  }

  function patchSettings(patch: Partial<DeckSettings>) {
    void onUpdateSettings({
      ...settings,
      ...patch,
    });
  }

  function updateErrorMessage(error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    const lowerMessage = message.toLowerCase();

    if (
      lowerMessage.includes("endpoint") ||
      lowerMessage.includes("pubkey") ||
      lowerMessage.includes("public key")
    ) {
      return "No Updates available";
    }

    return "Could not check for updates. Try again later.";
  }

  async function handleCheckForUpdates() {
    setSection("updates");
    setAvailableUpdate(null);
    setDownloadProgress(null);
    setUpdateStatus("checking");
    setUpdateMessage("Checking for updates...");

    try {
      const update = await check({ timeout: 30_000 });

      if (!update) {
        setUpdateStatus("current");
        setUpdateMessage("You are running the latest version.");
        return;
      }

      setAvailableUpdate(update);
      setUpdateStatus("available");
      setUpdateMessage(`Version ${update.version} is available.`);
    } catch (error) {
      setUpdateStatus("error");
      setUpdateMessage(updateErrorMessage(error));
    }
  }

  async function handleInstallUpdate() {
    if (!availableUpdate) {
      return;
    }

    let downloaded = 0;
    let contentLength = 0;

    setUpdateStatus("installing");
    setDownloadProgress(null);
    setUpdateMessage("Downloading update...");

    function onDownloadEvent(event: DownloadEvent) {
      if (event.event === "Started") {
        downloaded = 0;
        contentLength = event.data.contentLength ?? 0;
        setDownloadProgress(contentLength > 0 ? 0 : null);
      }

      if (event.event === "Progress") {
        downloaded += event.data.chunkLength;

        if (contentLength > 0) {
          setDownloadProgress(Math.min(100, Math.round((downloaded / contentLength) * 100)));
        }
      }

      if (event.event === "Finished") {
        setDownloadProgress(100);
        setUpdateMessage("Installing update...");
      }
    }

    try {
      await availableUpdate.downloadAndInstall(onDownloadEvent);
      setUpdateMessage("Update installed. Restarting...");
      await relaunch();
    } catch (error) {
      setUpdateStatus("error");
      setDownloadProgress(null);
      setUpdateMessage(updateErrorMessage(error));
    }
  }

  async function handleOpenFeedbackForm() {
    setFeedbackStatus("idle");

    if (!APP_CONFIG.feedbackFormUrl) {
      setFeedbackStatus("error");
      return;
    }

    try {
      await invoke<void>("open_external_url", {
        url: APP_CONFIG.feedbackFormUrl,
      });
      return;
    } catch {
      // Fall through to the plugin/browser fallbacks.
    }

    try {
      await openUrl(APP_CONFIG.feedbackFormUrl);
      return;
    } catch {
      if (typeof window !== "undefined") {
        const popup = window.open(APP_CONFIG.feedbackFormUrl, "_blank", "noopener,noreferrer");

        if (popup) {
          return;
        }
      }
    }

    setFeedbackStatus("error");
  }

  async function handleOpenDefaultAppsSettings() {
    try {
      await invoke<void>("open_default_apps_settings");
    } catch {
      setFeedbackStatus("error");
    }
  }

  return (
    <div className="drawerBackdrop" onMouseDown={onClose}>
      <aside className="appDrawer" onMouseDown={(e) => e.stopPropagation()}>
        <div className="drawerHeader">
          <div>
            <h2>
              {section !== "menu" && (
                <button
                  className="drawerBack"
                  title="Back"
                  onClick={() => handleSectionChange("menu")}
                >
                  <ChevronLeft size={18} />
                </button>
              )}{" "}
              {sectionTitle}
            </h2>
            {section === "menu" && <p>App options and information</p>}
          </div>

          <button className="iconOnlyButton" title="Close" onClick={onClose}>
            <X size={18} />
          </button>
        </div>

        {section === "menu" && (
          <div className="drawerSection">
            <button className="drawerMenuItem" onClick={() => handleSectionChange("settings")}>
              <Settings size={18} /> General
            </button>

            <button className="drawerMenuItem" onClick={() => handleSectionChange("help")}>
              <HelpCircle size={18} /> Help
            </button>

            <button className="drawerMenuItem" onClick={() => handleSectionChange("feedback")}>
              <MessageSquareMore size={18} /> Share Feedback
            </button>

            <button
              className="drawerMenuItem"
              disabled={updateStatus === "checking" || updateStatus === "installing"}
              onClick={handleCheckForUpdates}
            >
              <RefreshCw
                className={
                  updateStatus === "checking" || updateStatus === "installing"
                    ? "drawerSpinningIcon"
                    : undefined
                }
                size={18}
              />{" "}
              Check for Updates
            </button>

            <button className="drawerMenuItem" onClick={() => setSection("about")}>
              <Info size={18} /> About
            </button>
          </div>
        )}

        {section === "settings" && (
          <div className="drawerSection">
            <h3>Appearance</h3>
            <p className="drawerHint">Switch between Dark and Light mode.</p>

            <div className="drawerOption switchRow">
              <div className="switchLabel" title="Theme Mode Switch">
                {theme === "light" ? <Moon size={16} /> : <Sun size={16} />}

                <button
                  className={`switch ${theme === "dark" ? "active" : ""}`}
                  onClick={onThemeToggle}
                >
                  <div className="switchThumb" />
                </button>
              </div>
            </div>

            <h3>Deck</h3>
            <div className="drawerField">
              <span>Default deck view</span>
              <div className="drawerChoiceList" role="radiogroup" aria-label="Default deck view">
                {DECK_VIEW_OPTIONS.map(({ value, label, Icon }) => {
                  const isActive = settings.defaultDeckView === value;

                  return (
                    <button
                      aria-checked={isActive}
                      className={`drawerChoice ${isActive ? "active" : ""}`}
                      key={value}
                      onClick={() =>
                        patchSettings({
                          defaultDeckView: value,
                        })
                      }
                      role="radio"
                      type="button"
                    >
                      <Icon size={16} />
                      <span>{label}</span>
                      {isActive && <Check className="drawerChoiceCheck" size={15} />}
                    </button>
                  );
                })}
              </div>
            </div>

            <div className="drawerField">
              <span>Button trigger mode</span>
              <div
                className="drawerChoiceList"
                role="radiogroup"
                aria-label="Button trigger mode"
              >
                {TRIGGER_MODE_OPTIONS.map(({ value, label, Icon }) => {
                  const isActive = settings.buttonTriggerMode === value;

                  return (
                    <button
                      aria-checked={isActive}
                      className={`drawerChoice ${isActive ? "active" : ""}`}
                      key={value}
                      onClick={() =>
                        patchSettings({
                          buttonTriggerMode: value,
                        })
                      }
                      role="radio"
                      type="button"
                    >
                      <Icon size={16} />
                      <span>{label}</span>
                      {isActive && <Check className="drawerChoiceCheck" size={15} />}
                    </button>
                  );
                })}
              </div>
            </div>

            <h3>Startup</h3>
            <label className="drawerSettingRow">
              <span>Launch on startup</span>
              <button
                aria-checked={settings.launchOnStartup}
                className={`drawerCheckbox ${settings.launchOnStartup ? "active" : ""}`}
                onClick={() =>
                  patchSettings({ launchOnStartup: !settings.launchOnStartup })
                }
                role="checkbox"
                type="button"
              >
                <span className="drawerCheckboxIndicator">
                  {settings.launchOnStartup && <Check size={12} strokeWidth={3} />}
                </span>
              </button>
            </label>

            <label className="drawerSettingRow">
              <span>Start minimized to tray</span>
              <button
                aria-checked={settings.startMinimizedToTray}
                className={`drawerCheckbox ${settings.startMinimizedToTray ? "active" : ""}`}
                onClick={() =>
                  patchSettings({ startMinimizedToTray: !settings.startMinimizedToTray })
                }
                role="checkbox"
                type="button"
              >
                <span className="drawerCheckboxIndicator">
                  {settings.startMinimizedToTray && <Check size={12} strokeWidth={3} />}
                </span>
              </button>
            </label>
            <p className="drawerHint">Applies on the next app launch.</p>

            <h3>Safety</h3>
            <label className="drawerSettingRow">
              <span>Confirm before deleting</span>
              <button
                aria-checked={settings.confirmBeforeDelete}
                className={`drawerCheckbox ${settings.confirmBeforeDelete ? "active" : ""}`}
                onClick={() =>
                  patchSettings({ confirmBeforeDelete: !settings.confirmBeforeDelete })
                }
                role="checkbox"
                type="button"
              >
                <span className="drawerCheckboxIndicator">
                  {settings.confirmBeforeDelete && <Check size={12} strokeWidth={3} />}
                </span>
              </button>
            </label>
          </div>
        )}

        {section === "help" && (
          <div className="drawerSection">
            {visibleHelpTips.map((tip) => (
              <div key={tip.title} className="helpCard">
                <strong>{tip.title}</strong>
                <p>{tip.description}</p>
              </div>
            ))}

            {helpPageCount > 1 && (
              <div className="drawerActionRow drawerPagerRow">
                <button
                  disabled={currentHelpPage === 1}
                  onClick={() => setCurrentHelpPage((page) => Math.max(1, page - 1))}
                  type="button"
                >
                  Previous
                </button>

                <span className="drawerPagerLabel">
                  Page {currentHelpPage} of {helpPageCount}
                </span>

                <button
                  disabled={currentHelpPage === helpPageCount}
                  onClick={() => setCurrentHelpPage((page) => Math.min(helpPageCount, page + 1))}
                  type="button"
                >
                  Next
                </button>
              </div>
            )}
          </div>
        )}

        {section === "feedback" && (
          <div className="drawerSection">
            <div className="aboutCard">
              <h2>Share Feedback</h2>
              <p>Send ideas, bug reports, or UI notes through a hosted feedback form.</p>
              <p className="drawerHint">
                Send ideas, bug reports, or UI notes straight to the hosted form.
              </p>
            </div>

            <div className="drawerActionRow">
              <button
                className="drawerMenuItem"
                disabled={!APP_CONFIG.feedbackFormUrl}
                onClick={handleOpenFeedbackForm}
              >
                <Rocket size={18} /> Open Feedback Form
              </button>
            </div>

            {feedbackStatus === "error" && (
              <>
                <p className="drawerHint">
                  The form could not be opened automatically. Check that Windows has a default
                  browser selected for web links.
                </p>

                <button
                  className="drawerMenuItem"
                  onClick={handleOpenDefaultAppsSettings}
                  type="button"
                >
                  <Settings size={18} /> Open Default Apps Settings
                </button>
              </>
            )}
          </div>
        )}

        {section === "updates" && (
          <div className="drawerSection">
            <div className="aboutCard">
              <h2>{APP_CONFIG.name}</h2>
              <p>{updateMessage}</p>

              {availableUpdate?.body && (
                <p className="drawerHint">{availableUpdate.body}</p>
              )}

              <p className="drawerHint">Current version {APP_CONFIG.version}</p>
            </div>

            {updateStatus === "installing" && (
              <div className="updateProgressTrack">
                <div
                  className="updateProgressFill"
                  style={{ width: `${downloadProgress ?? 100}%` }}
                />
              </div>
            )}

            <div className="drawerActionRow">
              <button
                className="drawerMenuItem"
                disabled={updateStatus === "checking" || updateStatus === "installing"}
                onClick={handleCheckForUpdates}
              >
                <RefreshCw
                  className={updateStatus === "checking" ? "drawerSpinningIcon" : undefined}
                  size={18}
                />{" "}
                Check Again
              </button>

              {updateStatus === "available" && (
                <button className="drawerPrimaryAction" onClick={handleInstallUpdate}>
                  <Download size={18} /> Install Update
                </button>
              )}
            </div>
          </div>
        )}

        {section === "about" && (
          <div className="drawerSection">
            <div className="aboutCard aboutHeroCard">
              <img className="aboutAppIcon" src={titleBarIcon} alt={APP_CONFIG.name} />
              <h2>{APP_CONFIG.name}</h2>
              <p className="aboutVersion">Version {APP_CONFIG.version}</p>
              <p className="aboutDescription">
                A customizable desktop control surface for launching apps, opening links,
                playing sounds, and organizing quick actions into profiles.
              </p>
              <p className="authorMe">Antonis Georgosopoulos</p>
            </div>
          </div>
        )}
      </aside>
    </div>
  );
}
