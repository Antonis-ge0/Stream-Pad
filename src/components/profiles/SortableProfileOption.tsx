import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { Profile } from "../../types";

type SortableProfileOptionProps = {
  profile: Profile;
  active: boolean;
  onSelect: () => void;
};

export function SortableProfileOption({
  profile,
  active,
  onSelect,
}: SortableProfileOptionProps) {
  const sortable = useSortable({ id: profile.id });

  return (
    <button
      ref={sortable.setNodeRef}
      type="button"
      className={`profileDropdownItem ${active ? "active" : ""}`}
      style={{
        transform: CSS.Transform.toString(sortable.transform),
        transition: sortable.transition,
      }}
      {...sortable.attributes}
      {...sortable.listeners}
      onClick={onSelect}
    >
      {profile.name}
    </button>
  );
}
