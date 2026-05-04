import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Check, GripVertical } from "lucide-react";
import { useState } from "react";
import type { DragEvent, MouseEvent } from "react";
import type { DeckButton } from "../../types";
import { ButtonIcon } from "./ButtonIcon";

type SortableDeckButtonProps = {
  button: DeckButton;
  selected: boolean;
  selectionMode?: boolean;
  viewMode?: "tile" | "list";
  onSelect: () => void;
  onDropImport: (dataTransfer: DataTransfer) => void | Promise<void>;
  onToggleSelection: () => void;
  onTrigger: () => void | Promise<void>;
};

export function SortableDeckButton({
  button,
  selected,
  selectionMode = false,
  viewMode = "tile",
  onSelect,
  onDropImport,
  onToggleSelection,
  onTrigger,
}: SortableDeckButtonProps) {
  const sortable = useSortable({ id: button.id });
  const [dropActive, setDropActive] = useState(false);

  async function handleLeftClick() {
    if (selectionMode) {
      onToggleSelection();
      return;
    }

    onSelect();
    await onTrigger();
  }

  function handleRightClick(e: MouseEvent<HTMLButtonElement>) {
    e.preventDefault();

    if (selectionMode) {
      onToggleSelection();
      return;
    }

    onSelect();
  }

  function handleDragOver(event: DragEvent<HTMLButtonElement>) {
    event.preventDefault();
    event.dataTransfer.dropEffect = "copy";
    setDropActive(true);
  }

  function handleDragLeave(event: DragEvent<HTMLButtonElement>) {
    if (!event.currentTarget.contains(event.relatedTarget as Node | null)) {
      setDropActive(false);
    }
  }

  async function handleDrop(event: DragEvent<HTMLButtonElement>) {
    event.preventDefault();
    event.stopPropagation();
    setDropActive(false);
    await onDropImport(event.dataTransfer);
  }

  return (
    <button
      ref={sortable.setNodeRef}
      className={`deckButton ${viewMode === "list" ? "deckButtonList" : ""} ${
        selectionMode ? "selectionEnabled" : ""
      } ${dropActive ? "dropActive" : ""} ${selected ? "selected" : ""}`}
      data-deck-button-id={button.id}
      style={{
        transform: CSS.Transform.toString(sortable.transform),
        transition: sortable.transition,
      }}
      onClick={handleLeftClick}
      onContextMenu={handleRightClick}
      onDragLeave={handleDragLeave}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
      title="Left-click to run/select. Right-click to select only."
    >
      {selectionMode && (
        <div
          className={`selectionMarker ${selected ? "selected" : ""}`}
          aria-hidden="true"
        >
          {selected && <Check size={14} />}
        </div>
      )}

      <div
        className="dragHandle"
        {...sortable.attributes}
        {...sortable.listeners}
        onClick={(e) => e.stopPropagation()}
        onContextMenu={(e) => e.preventDefault()}
        title="Drag to reorder"
      >
        <GripVertical size={16} />
      </div>

      <ButtonIcon icon={button.icon} />
      <span className="deckButtonLabel">{button.label}</span>
    </button>
  );
}
