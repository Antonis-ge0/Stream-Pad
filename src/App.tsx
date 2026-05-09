import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, ImageIcon, Menu, Plus, Trash } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { disable, enable } from "@tauri-apps/plugin-autostart";
import { DeckPanel } from "@/components/deck/DeckPanel";
import { AppDrawer } from "@/components/drawer/AppDrawer";
import { ErrorBanner } from "@/components/feedback/ErrorBanner";
import { ButtonInspector } from "@/components/inspector/ButtonInspector";
import { ConfirmDeleteModal } from "@/components/modals/ConfirmDeleteModal";
import { EditProfileModal } from "@/components/modals/EditProfileModal";
import { IconPickerModal } from "@/components/modals/IconPickerModal";
import { NewProfileModal } from "@/components/modals/NewProfileModal";
import { ProfileDropdown } from "@/components/profiles/ProfileDropdown";
import { LoadingState } from "@/components/states/LoadingState";
import type { ImportedButtonData } from "@/domain/deck";
import { useDeckConfig } from "@/hooks/useDeckConfig";
import { useTheme } from "@/hooks/useTheme";
import { describeDroppedFile } from "@/services/deckImportService";
import { importFromDataTransfer } from "@/utils/dropImport";
import { CustomTitleBar } from "@/components/CustomTitleBar";
import titleBarIcon from "./assets/titlebar-icon.png";

type DroppedFileWithPath = File & {
  path?: string;
};

type NativeDropImportPayload = {
  position: {
    x: number;
    y: number;
  };
  importData: ImportedButtonData;
};

export default function App() {
  const [showIconPicker, setShowIconPicker] = useState(false);
  const [showNewProfileModal, setShowNewProfileModal] = useState(false);
  const [showEditProfileModal, setShowEditProfileModal] = useState(false);
  const [showAppDrawer, setShowAppDrawer] = useState(false);

  const { theme, toggleTheme } = useTheme();

  const {
    config,
    activeProfile,
    selectedButton,
    selectedButtonId,
    selectedButtonIds,
    selectedButtons,
    confirmDelete,
    error,
    isButtonSelectionMode,
    isLoading,
    isProfileLimitReached,
    settings,
    sensors,
    setConfirmDelete,
    clearError,
    persist,
    updateSettings,
    addProfile,
    editProfileName,
    clearButtonSelection,
    requestDeleteProfile,
    requestDeleteButtons,
    confirmDeleteNow,
    addButton,
    updateButton,
    importButtonData,
    duplicateButton,
    duplicateSelectedButtons,
    requestDeleteButton,
    reorderProfiles,
    reorderButtons,
    selectButton,
    toggleButtonSelection,
    toggleButtonSelectionMode,
    triggerButton,
  } = useDeckConfig();
  const importButtonDataRef = useRef(importButtonData);
  const dragDropHandledRef = useRef(false);

  useEffect(() => {
    importButtonDataRef.current = importButtonData;
  }, [importButtonData]);

  useEffect(() => {
    function isUrlLikeTransfer(dataTransfer: DataTransfer) {
      return [
        "text/uri-list",
        "text/plain",
        "text/html",
        "text/x-moz-url",
        "text/x-moz-url-data",
      ].some((type) => dataTransfer.types.includes(type));
    }

    function handleWindowDragOver(event: DragEvent) {
      if (!event.dataTransfer || !isUrlLikeTransfer(event.dataTransfer)) {
        return;
      }

      event.preventDefault();
      event.dataTransfer.dropEffect = "copy";
    }

    async function handleWindowDrop(event: DragEvent) {
      if (!event.dataTransfer || !isUrlLikeTransfer(event.dataTransfer)) {
        return;
      }

      const buttonId = findButtonIdAtCssPosition(event.clientX, event.clientY);

      if (!buttonId) {
        return;
      }

      event.preventDefault();
      dragDropHandledRef.current = true;

      const importData = await importDroppedData(event.dataTransfer);

      if (importData) {
        importButtonDataRef.current(buttonId, importData);
      }

      window.setTimeout(() => {
        dragDropHandledRef.current = false;
      }, 0);
    }

    window.addEventListener("dragover", handleWindowDragOver);
    window.addEventListener("drop", handleWindowDrop);

    return () => {
      window.removeEventListener("dragover", handleWindowDragOver);
      window.removeEventListener("drop", handleWindowDrop);
    };
  }, []);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        setShowIconPicker(false);
        setConfirmDelete(null);
        setShowNewProfileModal(false);
        setShowEditProfileModal(false);
        setShowAppDrawer(false);
        clearButtonSelection();
        if (isButtonSelectionMode) {
          toggleButtonSelectionMode();
        }
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    clearButtonSelection,
    isButtonSelectionMode,
    setConfirmDelete,
    toggleButtonSelectionMode,
  ]);

  useEffect(() => {
    async function bindNativeDropImport() {
      try {
        return await getCurrentWindow().listen<NativeDropImportPayload>(
          "native-drop-import",
          (event) => {
            const buttonId = findButtonIdAtPosition(
              event.payload.position.x,
              event.payload.position.y,
            );

            if (!buttonId) {
              return;
            }

            dragDropHandledRef.current = true;
            importButtonDataRef.current(buttonId, event.payload.importData);

            window.setTimeout(() => {
              dragDropHandledRef.current = false;
            }, 0);
          },
        );
      } catch {
        return undefined;
      }
    }

    let unlisten: (() => void) | undefined;
    let disposed = false;

    bindNativeDropImport().then((handler) => {
      if (disposed) {
        handler?.();
        return;
      }

      unlisten = handler;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  if (isLoading) {
    return <LoadingState message="Loading deck editor…" />;
  }

  if (!config) {
    return (
      <main className="appShell">
        <CustomTitleBar />
        <ErrorBanner
          title="Could not load app"
          message="The deck configuration could not be loaded."
          onDismiss={clearError}
        />
      </main>
    );
  }

  const showBulkDeleteInspector = isButtonSelectionMode && selectedButtons.length > 0;

  function handleWorkspaceMouseDown(event: React.MouseEvent<HTMLElement>) {
    if (!selectedButtonId || isButtonSelectionMode) {
      return;
    }

    const target = event.target as HTMLElement;

    if (
      target.closest(".deckButton") ||
      target.closest(".inspector") ||
      target.closest(".panelHeader") ||
      target.closest(".deckPagination")
    ) {
      return;
    }

    clearButtonSelection();
  }

  function findButtonIdAtPosition(physicalX: number, physicalY: number) {
    const scaleFactor = window.devicePixelRatio || 1;
    return findButtonIdAtCssPosition(physicalX / scaleFactor, physicalY / scaleFactor);
  }

  function findButtonIdAtCssPosition(cssX: number, cssY: number) {
    const element = document.elementFromPoint(cssX, cssY);
    return element?.closest<HTMLElement>("[data-deck-button-id]")?.dataset.deckButtonId;
  }

  async function importDroppedData(dataTransfer: DataTransfer) {
    const firstFile = dataTransfer.files.item(0) as DroppedFileWithPath | null;
    const droppedPath =
      typeof firstFile?.path === "string" && firstFile.path.trim() ? firstFile.path : null;

    if (droppedPath) {
      try {
        return await describeDroppedFile(droppedPath);
      } catch {
        return null;
      }
    }

    return importFromDataTransfer(dataTransfer);
  }

  async function handleButtonDataTransferDrop(
    buttonId: string,
    dataTransfer: DataTransfer,
  ) {
    if (dragDropHandledRef.current) {
      return;
    }

    const importData = await importDroppedData(dataTransfer);

    if (importData) {
      importButtonData(buttonId, importData);
    }
  }

  async function handleUpdateSettings(nextSettings: typeof settings) {
    const previousSettings = settings;

    updateSettings(() => nextSettings);

    if (previousSettings.launchOnStartup === nextSettings.launchOnStartup) {
      return;
    }

    try {
      if (nextSettings.launchOnStartup) {
        await enable();
      } else {
        await disable();
      }
    } catch {
      updateSettings(() => previousSettings);
    }
  }

  return (
    <main className="appShell">
      <CustomTitleBar />
      {error && (
        <ErrorBanner title={error.title} message={error.message} onDismiss={clearError} />
      )}

      <header className="topbar">
        <div className="topbarBrand">
          <img
            className="titleBarIcon"
            src={titleBarIcon}
            alt="Stream Pad"
            data-tauri-drag-region
          />
          <h1>Stream Pad</h1>
        </div>

        <div className="profileControls">
          <div
            className={
              isProfileLimitReached
                ? "addButtonWrapper addButtonWrapperLimitReached"
                : "addButtonWrapper"
            }
          >
            <button
              className="topbarTextButton"
              onClick={() => setShowNewProfileModal(true)}
              disabled={isProfileLimitReached}
              title="Add a Profile"
            >
              <Plus size={18} /> Add Profile
            </button>
            {isProfileLimitReached && (
              <div className="buttonLimitNotification" role="status">
                Only 15 profiles can be created.
              </div>
            )}
          </div>

          {activeProfile && (
            <button
              className="topbarTextButton topbarDangerButton"
              onClick={requestDeleteProfile}
              title="Delete Profile"
            >
              <Trash size={18} /> Delete
            </button>
          )}

          {activeProfile && (
            <ProfileDropdown
              profiles={config.profiles}
              activeProfileId={config.activeProfileId}
              onSelect={(profileId) => {
                clearButtonSelection();
                if (isButtonSelectionMode) {
                  toggleButtonSelectionMode();
                }
                persist({ ...config, activeProfileId: profileId });
              }}
              onReorder={reorderProfiles}
            />
          )}

          <button
            className="iconOnlyButton topbarIconButton"
            onClick={() => setShowAppDrawer(true)}
            title="Menu"
          >
            <Menu size={18} />
          </button>
        </div>
      </header>

      <section
        className={activeProfile ? "workspace" : "workspace workspaceSingle"}
        onMouseDown={handleWorkspaceMouseDown}
      >
        <DeckPanel
          defaultViewMode={settings.defaultDeckView}
          profile={activeProfile}
          selectedButtonId={selectedButtonId}
          selectedButtonIds={selectedButtonIds}
          selectionMode={isButtonSelectionMode}
          triggerMode={settings.buttonTriggerMode}
          sensors={sensors}
          onAddButton={addButton}
          onCreateProfile={() => setShowNewProfileModal(true)}
          onEditProfile={() => setShowEditProfileModal(true)}
          onDragEnd={reorderButtons}
          onDropImport={handleButtonDataTransferDrop}
          onSelectButton={selectButton}
          onToggleButtonSelection={toggleButtonSelection}
          onToggleSelectionMode={toggleButtonSelectionMode}
          onTriggerButton={triggerButton}
        />

        {activeProfile && (
          <aside className="inspector">
            {showBulkDeleteInspector ? (
              <div className="bulkSelectionInspector">
                <div className="inspectorHeader">
                  <h2>Edit Page</h2>
                </div>

                <div className="previewCard" title="Selected Buttons">
                  <strong>{selectedButtons.length} buttons selected</strong>
                </div>

                <div className="form">
                  <p className="bulkSelectionCopy">
                    Choose the buttons you want to remove, then delete them together here.
                  </p>

                  <div className="inspectorActions inspectorActionsSingle">
                    <button
                      type="button"
                      onClick={clearButtonSelection}
                      title="Clear Selection"
                    >
                      Clear Selection
                    </button>

                    <button
                      type="button"
                      onClick={duplicateSelectedButtons}
                      title="Duplicate Selected Buttons"
                    >
                      <Copy size={16} /> Duplicate Selected
                    </button>

                    <button
                      className="danger"
                      type="button"
                      onClick={() => requestDeleteButtons(selectedButtonIds)}
                      title="Delete Selected Buttons"
                    >
                      <Trash size={16} /> Delete Selected
                    </button>
                  </div>
                </div>
              </div>
            ) : !selectedButton ? (
              <div className="inspectorEmpty">
                <div>
                  <ImageIcon size={38} />
                </div>
                <h2>{isButtonSelectionMode ? "Select buttons" : "Select a button"}</h2>
                <p>
                  {isButtonSelectionMode
                    ? "Choose the buttons you want to remove, then use Delete in Edit Page."
                    : "Left click to run/select, or use the drag handle to reorder."}
                </p>
              </div>
            ) : (
              <ButtonInspector
                button={selectedButton}
                onChange={updateButton}
                onDelete={() => requestDeleteButton(selectedButton.id)}
                onDuplicate={() => duplicateButton(selectedButton)}
                onOpenIconPicker={() => setShowIconPicker(true)}
              />
            )}
          </aside>
        )}
      </section>

      {showNewProfileModal && (
        <NewProfileModal
          onCancel={() => setShowNewProfileModal(false)}
          onCreate={(name) => {
            const created = addProfile(name);

            if (created) {
              setShowNewProfileModal(false);
            }
          }}
        />
      )}

      {showEditProfileModal && (
        <EditProfileModal
          currentName={activeProfile?.name ?? ""}
          onCancel={() => setShowEditProfileModal(false)}
          onSave={(name) => {
            editProfileName(name);
            setShowEditProfileModal(false);
          }}
        />
      )}

      {showIconPicker && selectedButton && (
        <IconPickerModal
          button={selectedButton}
          onClose={() => setShowIconPicker(false)}
          onChange={updateButton}
        />
      )}

      {confirmDelete && (
        <ConfirmDeleteModal
          type={confirmDelete.type}
          name={"name" in confirmDelete ? confirmDelete.name : ""}
          count={"count" in confirmDelete ? confirmDelete.count : undefined}
          onCancel={() => setConfirmDelete(null)}
          onConfirm={confirmDeleteNow}
        />
      )}

      {showAppDrawer && (
        <AppDrawer
          settings={settings}
          theme={theme}
          onClose={() => setShowAppDrawer(false)}
          onUpdateSettings={handleUpdateSettings}
          onThemeToggle={toggleTheme}
        />
      )}
    </main>
  );
}
