import {
  ChevronLeft,
  Copy,
  Download,
  HelpCircle,
  Info,
  Mail,
  MessageSquareMore,
  Moon,
  RefreshCw,
  Settings,
  Sun,
  X,
} from "lucide-react";
import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { APP_CONFIG } from "@/config/appConfig";
import type { Theme } from "@/hooks/useTheme";

type DrawerSection = "menu" | "settings" | "help" | "feedback" | "updates" | "about";
type UpdateStatus = "idle" | "checking" | "available" | "current" | "installing" | "error";
type FeedbackStatus = "idle" | "copied" | "error";

const HELP_TIPS = [
  {
    title: "Run a button",
    description: "Left click a deck button to run its assigned action.",
  },
  {
    title: "Edit a button",
    description: "Right a button to edit its name, icon, and action in the right panel.",
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

type AppDrawerProps = {
  onClose: () => void;
  onThemeToggle: () => void;
  theme: Theme;
};

export function AppDrawer({ onClose, onThemeToggle, theme }: AppDrawerProps) {
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
        ? "General Settings"
        : section === "help"
          ? "Help"
          : section === "feedback"
            ? "Share Feedback"
            : section === "updates"
              ? "Check for Updates"
              : "About Us";
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

  async function handleOpenFeedbackEmail() {
    try {
      const subject = encodeURIComponent(`${APP_CONFIG.name} Feedback`);
      const body = encodeURIComponent(
        [
          `App version: ${APP_CONFIG.version}`,
          "",
          "What happened:",
          "",
          "What you expected:",
          "",
          "Steps to reproduce:",
        ].join("\n"),
      );

      await openUrl(`mailto:?subject=${subject}&body=${body}`);
    } catch {
      setFeedbackStatus("error");
    }
  }

  async function handleCopyFeedbackTemplate() {
    try {
      await navigator.clipboard.writeText(
        [
          `${APP_CONFIG.name} Feedback`,
          `App version: ${APP_CONFIG.version}`,
          "",
          "What happened:",
          "",
          "What you expected:",
          "",
          "Steps to reproduce:",
        ].join("\n"),
      );

      setFeedbackStatus("copied");
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
              <Settings size={18} /> General Settings
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
              <Info size={18} /> About Us
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
              <p>
                Send ideas, bug reports, or UI notes to help improve the Stream Deck
                experience.
              </p>
              <p className="drawerHint">Pick the option that feels easier in the moment.</p>
            </div>

            <div className="drawerActionRow">
              <button onClick={handleOpenFeedbackEmail}>
                <Mail size={18} /> Open Email Draft
              </button>

              <button onClick={handleCopyFeedbackTemplate}>
                <Copy size={18} /> Copy Feedback Template
              </button>
            </div>

            {feedbackStatus === "copied" && (
              <p className="drawerHint">Feedback template copied to your clipboard.</p>
            )}

            {feedbackStatus === "error" && (
              <p className="drawerHint">Clipboard access was not available for this action.</p>
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
            <div className="aboutCard">
              <h2>{APP_CONFIG.name}</h2>
              <p>
                A customizable desktop control surface built with React, Tauri, and Rust.
              </p>
              <p className="drawerHint">Version {APP_CONFIG.version}</p>
              <p className="authorMe">Antonis Georgosopoulos</p>
            </div>
          </div>
        )}
      </aside>
    </div>
  );
}
