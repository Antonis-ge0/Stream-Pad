import type { DragEndEvent, SensorDescriptor, SensorOptions } from "@dnd-kit/core";
import { DndContext } from "@dnd-kit/core";
import { SortableContext, rectSortingStrategy } from "@dnd-kit/sortable";
import { CheckSquare2, Edit2, Grid2X2, List, Plus, Settings, X } from "lucide-react";
import { useMemo, useState } from "react";
import type { Profile } from "../../types";
import { SortableDeckButton } from "./SortableDeckButton";

const MAX_BUTTONS_PER_PROFILE = 20;
const LIST_BUTTONS_PER_PAGE = 10;

type ViewMode = "tile" | "list";
type TriggerMode = "singleClick" | "doubleClick";

type DeckPanelProps = {
  profile: Profile | null;
  selectedButtonId: string | null;
  selectedButtonIds: string[];
  selectionMode: boolean;
  defaultViewMode?: ViewMode;
  triggerMode?: TriggerMode;
  sensors: SensorDescriptor<SensorOptions>[];
  onAddButton: () => void;
  onCreateProfile?: () => void;
  onEditProfile: () => void;
  onDragEnd: (event: DragEndEvent) => void;
  onDropImport: (buttonId: string, dataTransfer: DataTransfer) => void | Promise<void>;
  onSelectButton: (buttonId: string) => void;
  onToggleButtonSelection: (buttonId: string) => void;
  onToggleSelectionMode: () => void;
  onTriggerButton: (buttonId: string) => void | Promise<void>;
};

export function DeckPanel({
  profile,
  selectedButtonId,
  selectedButtonIds,
  selectionMode,
  defaultViewMode = "tile",
  triggerMode = "singleClick",
  sensors,
  onAddButton,
  onCreateProfile,
  onEditProfile,
  onDragEnd,
  onDropImport,
  onSelectButton,
  onToggleButtonSelection,
  onToggleSelectionMode,
  onTriggerButton,
}: DeckPanelProps) {
  const [viewMode, setViewMode] = useState<ViewMode>(defaultViewMode);
  const [currentListPage, setCurrentListPage] = useState(1);

  const isButtonLimitReached = (profile?.buttons.length ?? 0) >= MAX_BUTTONS_PER_PROFILE;
  const listPageCount = Math.max(
    1,
    Math.ceil((profile?.buttons.length ?? 0) / LIST_BUTTONS_PER_PAGE),
  );

  const visibleButtons = useMemo(() => {
    if (!profile) {
      return [];
    }

    if (viewMode === "tile") {
      return profile.buttons;
    }

    const startIndex = (currentListPage - 1) * LIST_BUTTONS_PER_PAGE;
    return profile.buttons.slice(startIndex, startIndex + LIST_BUTTONS_PER_PAGE);
  }, [currentListPage, profile, viewMode]);

  function handleViewModeChange(nextViewMode: ViewMode) {
    setViewMode(nextViewMode);
    setCurrentListPage(1);
  }

  return (
    <section className="deckPanel">
      <div className="panelHeader">
        <div>
          <h3>{profile?.name}</h3>
        </div>

        <div className="deckHeaderActions">
          <div className="viewModeToggle" aria-label="Choose deck view">
            <button
              className={viewMode === "tile" ? "active" : ""}
              type="button"
              onClick={() => handleViewModeChange("tile")}
              title="Tile view"
              aria-label="Tile view"
              aria-pressed={viewMode === "tile"}
            >
              <Grid2X2 size={18} />
            </button>

            <button
              className={viewMode === "list" ? "active" : ""}
              type="button"
              onClick={() => handleViewModeChange("list")}
              title="List view"
              aria-label="List view"
              aria-pressed={viewMode === "list"}
            >
              <List size={20} />
            </button>
          </div>

          {profile && (
            <button
              type="button"
              className={
                selectionMode ? "selectionModeButton active" : "selectionModeButton"
              }
              onClick={onToggleSelectionMode}
              title={selectionMode ? "Exit selection mode" : "Select multiple buttons"}
            >
              {selectionMode ? <X size={18} /> : <CheckSquare2 size={18} />}
              {selectionMode ? "Done" : "Select"}
            </button>
          )}

          {profile && (
            <div className="deckButtonActions">
              <button type="button" onClick={onEditProfile} title="Edit Profile Name">
                <Edit2 size={18} /> Edit
              </button>

              <div
                className={
                  isButtonLimitReached
                    ? "addButtonWrapper addButtonWrapperLimitReached"
                    : "addButtonWrapper"
                }
              >
                <button
                  className="primary"
                  onClick={onAddButton}
                  disabled={isButtonLimitReached}
                >
                  <Plus size={18} /> Add Button
                </button>
                {isButtonLimitReached && (
                  <div className="buttonLimitNotification" role="status">
                    Only {MAX_BUTTONS_PER_PROFILE} buttons per profile are allowed.
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      {!profile ? (
        <div className="emptyState deckEmptyState">
          <div>
            <Settings size={48} />
          </div>
          <h3>No profiles yet</h3>
          <p>Create a profile to start building your Stream Deck.</p>
          {onCreateProfile && (
            <button className="primary" onClick={onCreateProfile} type="button">
              <Plus size={18} /> Create Profile
            </button>
          )}
        </div>
      ) : profile.buttons.length === 0 ? (
        <div className="emptyState">
          <div>
            <Settings size={48} />
          </div>
          <h3>No buttons yet</h3>
          <p>Add a button to start building this profile.</p>
        </div>
      ) : (
        <>
          <DndContext sensors={sensors} onDragEnd={onDragEnd}>
            <SortableContext
              items={profile.buttons.map((button) => button.id)}
              strategy={rectSortingStrategy}
            >
              <div
                className={
                  viewMode === "list" ? "buttonGrid buttonGridList" : "buttonGrid"
                }
              >
                {visibleButtons.map((button) => (
                  <SortableDeckButton
                    key={button.id}
                    button={button}
                    selected={
                      selectionMode
                        ? selectedButtonIds.includes(button.id)
                        : button.id === selectedButtonId
                    }
                    selectionMode={selectionMode}
                    triggerMode={triggerMode}
                    viewMode={viewMode}
                    onDropImport={(dataTransfer) => onDropImport(button.id, dataTransfer)}
                    onSelect={() => onSelectButton(button.id)}
                    onToggleSelection={() => onToggleButtonSelection(button.id)}
                    onTrigger={() => onTriggerButton(button.id)}
                  />
                ))}
              </div>
            </SortableContext>
          </DndContext>

          {viewMode === "list" && listPageCount > 1 && (
            <div className="deckPagination">
              <button
                type="button"
                onClick={() => setCurrentListPage((page) => Math.max(1, page - 1))}
                disabled={currentListPage === 1}
              >
                Previous
              </button>

              <span>
                Page {currentListPage} of {listPageCount}
              </span>

              <button
                type="button"
                onClick={() =>
                  setCurrentListPage((page) => Math.min(listPageCount, page + 1))
                }
                disabled={currentListPage === listPageCount}
              >
                Next
              </button>
            </div>
          )}
        </>
      )}
    </section>
  );
}
