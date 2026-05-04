import { DragEndEvent, PointerSensor, useSensor, useSensors } from "@dnd-kit/core";
import { arrayMove } from "@dnd-kit/sortable";
import { useEffect, useMemo, useState } from "react";
import type {
  AppError,
  ConfirmDeleteState,
  DeckButton,
  DeckConfig,
  ImportedButtonData,
  Profile,
} from "@/domain/deck";
import { triggerDeckButton } from "@/services/deckActionService";
import { loadDeckConfig, saveDeckConfig } from "@/services/deckConfigService";
import { getErrorMessage } from "@/utils/getErrorMessage";
import { logger } from "@/utils/logger";
import { uid } from "@/utils/uid";

const MAX_BUTTONS_PER_PROFILE = 20;
const MAX_PROFILES = 15;
const DEFAULT_SETTINGS = {
  launchOnStartup: false,
  startMinimizedToTray: false,
  confirmBeforeDelete: true,
  defaultDeckView: "tile",
  buttonTriggerMode: "singleClick",
} as const;

export function useDeckConfig() {
  const [config, setConfig] = useState<DeckConfig | null>(null);
  const [selectedButtonId, setSelectedButtonId] = useState<string | null>(null);
  const [selectedButtonIds, setSelectedButtonIds] = useState<string[]>([]);
  const [isButtonSelectionMode, setIsButtonSelectionMode] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<ConfirmDeleteState>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8,
      },
    }),
  );

  useEffect(() => {
    async function load() {
      try {
        setIsLoading(true);
        const loadedConfig = await loadDeckConfig();
        const normalizedTriggerMode =
          loadedConfig.settings?.buttonTriggerMode === "doubleClick"
            ? "doubleClick"
            : "singleClick";

        setConfig({
          ...loadedConfig,
          settings: {
            ...DEFAULT_SETTINGS,
            ...loadedConfig.settings,
            buttonTriggerMode: normalizedTriggerMode,
          },
        });
      } catch (loadError) {
        logger.error("Failed to load deck config", loadError);

        setError({
          title: "Could not load deck",
          message: getErrorMessage(loadError),
        });
      } finally {
        setIsLoading(false);
      }
    }

    load();
  }, []);

  const activeProfile = useMemo(() => {
    if (!config || config.profiles.length === 0) return null;

    return (
      config.profiles.find((profile) => profile.id === config.activeProfileId) ??
      config.profiles[0]
    );
  }, [config]);

  const selectedButton = activeProfile?.buttons.find(
    (button) => button.id === selectedButtonId,
  );
  const selectedButtons = activeProfile
    ? activeProfile.buttons.filter((button) => selectedButtonIds.includes(button.id))
    : [];

  async function persist(next: DeckConfig) {
    const previousConfig = config;

    setConfig(next);

    try {
      await saveDeckConfig(next);
    } catch (saveError) {
      logger.error("Failed to save deck config", saveError);

      if (previousConfig) {
        setConfig(previousConfig);
      }

      setError({
        title: "Could not save changes",
        message: getErrorMessage(saveError),
      });
    }
  }

  function clearError() {
    setError(null);
  }

  function updateSettings(updater: (settings: DeckConfig["settings"]) => DeckConfig["settings"]) {
    if (!config) {
      return;
    }

    persist({
      ...config,
      settings: updater(config.settings ?? DEFAULT_SETTINGS),
    });
  }

  function updateActiveProfile(updater: (profile: Profile) => Profile) {
    if (!config) return;

    persist({
      ...config,
      profiles: config.profiles.map((profile) =>
        profile.id === config.activeProfileId ? updater(profile) : profile,
      ),
    });
  }

  function addProfile(name: string) {
    if (!config || config.profiles.length >= MAX_PROFILES) {
      return false;
    }

    const profile: Profile = {
      id: uid(),
      name: name.trim() || `Profile ${config.profiles.length + 1}`,
      buttons: [],
    };

    persist({
      ...config,
      activeProfileId: profile.id,
      profiles: [...config.profiles, profile],
    });

    setSelectedButtonId(null);
    setSelectedButtonIds([]);
    setIsButtonSelectionMode(false);

    return true;
  }

  function editProfileName(name: string) {
    if (!config || !activeProfile) return;

    persist({
      ...config,
      profiles: config.profiles.map((profile) =>
        profile.id === activeProfile.id
          ? { ...profile, name: name.trim() || profile.name }
          : profile,
      ),
    });
  }

  function requestDeleteProfile() {
    if (!config || !activeProfile) return;

    if (!config.settings.confirmBeforeDelete) {
      const remainingProfiles = config.profiles.filter(
        (profile) => profile.id !== activeProfile.id,
      );

      persist({
        ...config,
        activeProfileId: remainingProfiles[0]?.id ?? "",
        profiles: remainingProfiles,
      });

      setSelectedButtonId(null);
      setSelectedButtonIds([]);
      setIsButtonSelectionMode(false);
      return;
    }

    setConfirmDelete({
      type: "profile",
      id: activeProfile.id,
      name: activeProfile.name,
    });
  }

  function addButton() {
    if (!activeProfile || activeProfile.buttons.length >= MAX_BUTTONS_PER_PROFILE) {
      return;
    }

    const button: DeckButton = {
      id: uid(),
      label: "New Button",
      icon: null,
      actions: [],
    };

    updateActiveProfile((profile) => ({
      ...profile,
      buttons: [...profile.buttons, button],
    }));

    setSelectedButtonId(button.id);
    setSelectedButtonIds([]);
    setIsButtonSelectionMode(false);
  }

  function updateButton(button: DeckButton) {
    updateActiveProfile((profile) => ({
      ...profile,
      buttons: profile.buttons.map((currentButton) =>
        currentButton.id === button.id ? button : currentButton,
      ),
    }));
  }

  function importButtonData(buttonId: string, importData: ImportedButtonData) {
    updateActiveProfile((profile) => ({
      ...profile,
      buttons: profile.buttons.map((button) =>
        button.id === buttonId
          ? {
              ...button,
              label: importData.label,
              icon: importData.icon ?? null,
              actions: importData.actions,
            }
          : button,
      ),
    }));

    setSelectedButtonId(buttonId);
    setSelectedButtonIds([]);
    setIsButtonSelectionMode(false);
  }

  function duplicateButton(button: DeckButton) {
    if (!activeProfile || activeProfile.buttons.length >= MAX_BUTTONS_PER_PROFILE) {
      return;
    }

    const copy: DeckButton = {
      ...button,
      id: uid(),
      label: `${button.label} Copy`,
    };

    updateActiveProfile((profile) => ({
      ...profile,
      buttons: [...profile.buttons, copy],
    }));

    setSelectedButtonId(copy.id);
    setSelectedButtonIds([]);
    setIsButtonSelectionMode(false);
  }

  function duplicateSelectedButtons() {
    if (!activeProfile || selectedButtons.length === 0) {
      return;
    }

    const remainingCapacity = MAX_BUTTONS_PER_PROFILE - activeProfile.buttons.length;

    if (remainingCapacity <= 0) {
      return;
    }

    const buttonsToCopy = selectedButtons.slice(0, remainingCapacity).map((button) => ({
      ...button,
      id: uid(),
      label: `${button.label} Copy`,
    }));

    updateActiveProfile((profile) => ({
      ...profile,
      buttons: [...profile.buttons, ...buttonsToCopy],
    }));
  }

  function requestDeleteButton(id: string) {
    const button = activeProfile?.buttons.find((item) => item.id === id);
    if (!button) return;

    if (!config?.settings.confirmBeforeDelete) {
      updateActiveProfile((profile) => ({
        ...profile,
        buttons: profile.buttons.filter((currentButton) => currentButton.id !== id),
      }));

      setSelectedButtonId(null);
      return;
    }

    setConfirmDelete({
      type: "button",
      id,
      name: button.label,
    });
  }

  function requestDeleteButtons(ids: string[]) {
    if (!activeProfile || ids.length === 0) return;

    if (!config?.settings.confirmBeforeDelete) {
      const idsToDelete = new Set(ids);

      updateActiveProfile((profile) => ({
        ...profile,
        buttons: profile.buttons.filter((button) => !idsToDelete.has(button.id)),
      }));

      setSelectedButtonId(null);
      setSelectedButtonIds([]);
      setIsButtonSelectionMode(false);
      return;
    }

    setConfirmDelete({
      type: "buttons",
      ids,
      count: ids.length,
    });
  }

  function selectButton(buttonId: string) {
    setSelectedButtonId(buttonId);
    setSelectedButtonIds([]);
    setIsButtonSelectionMode(false);
  }

  function toggleButtonSelection(buttonId: string) {
    setSelectedButtonId(null);

    setSelectedButtonIds((currentIds) =>
      currentIds.includes(buttonId)
        ? currentIds.filter((id) => id !== buttonId)
        : [...currentIds, buttonId],
    );
  }

  function clearButtonSelection() {
    setSelectedButtonId(null);
    setSelectedButtonIds([]);
  }

  function toggleButtonSelectionMode() {
    setIsButtonSelectionMode((currentValue) => {
      const nextValue = !currentValue;

      if (nextValue) {
        setSelectedButtonId(null);
      } else {
        setSelectedButtonIds([]);
      }

      return nextValue;
    });
  }

  function confirmDeleteNow() {
    if (!confirmDelete || !config) return;

    if (confirmDelete.type === "button") {
      updateActiveProfile((profile) => ({
        ...profile,
        buttons: profile.buttons.filter((button) => button.id !== confirmDelete.id),
      }));

      setSelectedButtonId(null);
    }

    if (confirmDelete.type === "buttons") {
      const idsToDelete = new Set(confirmDelete.ids);

      updateActiveProfile((profile) => ({
        ...profile,
        buttons: profile.buttons.filter((button) => !idsToDelete.has(button.id)),
      }));

      setSelectedButtonId(null);
      setSelectedButtonIds([]);
      setIsButtonSelectionMode(false);
    }

    if (confirmDelete.type === "profile") {
      if (!activeProfile) return;

      const remainingProfiles = config.profiles.filter(
        (profile) => profile.id !== confirmDelete.id,
      );

      persist({
        ...config,
        activeProfileId: remainingProfiles[0]?.id ?? "",
        profiles: remainingProfiles,
      });

      setSelectedButtonId(null);
      setSelectedButtonIds([]);
      setIsButtonSelectionMode(false);
    }

    setConfirmDelete(null);
  }

  function reorderProfiles(oldIndex: number, newIndex: number) {
    if (!config) return;

    persist({
      ...config,
      profiles: arrayMove(config.profiles, oldIndex, newIndex),
    });
  }

  function reorderButtons(event: DragEndEvent) {
    if (!activeProfile) return;

    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const oldIndex = activeProfile.buttons.findIndex((button) => button.id === active.id);
    const newIndex = activeProfile.buttons.findIndex((button) => button.id === over.id);

    if (oldIndex < 0 || newIndex < 0) return;

    updateActiveProfile((profile) => ({
      ...profile,
      buttons: arrayMove(profile.buttons, oldIndex, newIndex),
    }));
  }

  async function triggerButton(buttonId: string) {
    try {
      await triggerDeckButton(buttonId, config?.activeProfileId ?? null);
    } catch (triggerError) {
      logger.error("Failed to trigger button", triggerError);

      setError({
        title: "Could not run button",
        message: getErrorMessage(triggerError),
      });
    }
  }

  return {
    config,
    activeProfile,
    selectedButton,
    selectedButtonId,
    confirmDelete,
    error,
    isLoading,
    isButtonSelectionMode,
    isProfileLimitReached: (config?.profiles.length ?? 0) >= MAX_PROFILES,
    settings: config?.settings ?? DEFAULT_SETTINGS,
    sensors,
    selectedButtonIds,
    selectedButtons,
    setSelectedButtonId,
    setConfirmDelete,
    clearError,
    persist,
    updateSettings,
    addProfile,
    editProfileName,
    requestDeleteProfile,
    confirmDeleteNow,
    addButton,
    clearButtonSelection,
    requestDeleteButtons,
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
  };
}
