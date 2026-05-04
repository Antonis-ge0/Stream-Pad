import {
  DndContext,
  DragEndEvent,
  PointerSensor,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import { SortableContext, rectSortingStrategy } from "@dnd-kit/sortable";
import { ListChevronsDownUp } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { Profile } from "../../types";
import { SortableProfileOption } from "./SortableProfileOption";

type ProfileDropdownProps = {
  profiles: Profile[];
  activeProfileId: string;
  onSelect: (profileId: string) => void;
  onReorder: (oldIndex: number, newIndex: number) => void;
};

export function ProfileDropdown({
  profiles,
  activeProfileId,
  onSelect,
  onReorder,
}: ProfileDropdownProps) {
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement | null>(null);

  const activeProfile =
    profiles.find((profile) => profile.id === activeProfileId) ?? profiles[0];

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 6,
      },
    }),
  );

  function onDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const oldIndex = profiles.findIndex((profile) => profile.id === active.id);
    const newIndex = profiles.findIndex((profile) => profile.id === over.id);

    onReorder(oldIndex, newIndex);
  }

  useEffect(() => {
    if (!open) return;

    function handleClickOutside(e: MouseEvent) {
      if (!containerRef.current) return;

      if (!containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }

    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        setOpen(false);
      }
    }

    document.addEventListener("mousedown", handleClickOutside);
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [open]);

  return (
    <div className="profileDropdown" ref={containerRef}>
      <button
        type="button"
        className="profileDropdownButton"
        onClick={() => setOpen((value) => !value)}
        title="Profile List"
      >
        <span>{activeProfile?.name ?? "Select profile"}</span>
        <span className="profileDropdownArrow">
          <ListChevronsDownUp size={18} />
        </span>
      </button>

      {open && (
        <div className="profileDropdownMenu">
          <DndContext sensors={sensors} onDragEnd={onDragEnd}>
            <SortableContext
              items={profiles.map((profile) => profile.id)}
              strategy={rectSortingStrategy}
            >
              {profiles.map((profile) => (
                <SortableProfileOption
                  key={profile.id}
                  profile={profile}
                  active={profile.id === activeProfileId}
                  onSelect={() => {
                    onSelect(profile.id);
                    setOpen(false);
                  }}
                />
              ))}
            </SortableContext>
          </DndContext>
        </div>
      )}
    </div>
  );
}
