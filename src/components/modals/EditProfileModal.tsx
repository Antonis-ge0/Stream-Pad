import { useState } from "react";

type EditProfileModalProps = {
  currentName: string;
  onCancel: () => void;
  onSave: (name: string) => void;
};

export function EditProfileModal({
  currentName,
  onCancel,
  onSave,
}: EditProfileModalProps) {
  const [name, setName] = useState(currentName);

  function submit() {
    onSave(name);
  }

  return (
    <div className="modalBackdrop" onMouseDown={onCancel}>
      <div className="confirmModal" onMouseDown={(e) => e.stopPropagation()}>
        <h2>Edit Profile</h2>

        <p>Change this profile's name.</p>

        <input
          autoFocus
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Profile Name"
          title="Profile Name"
          onKeyDown={(e) => {
            if (e.key === "Enter") submit();
          }}
        />

        <div className="confirmActions">
          <button title="Cancel" onClick={onCancel}>
            Cancel
          </button>

          <button className="primary" title="Save" onClick={submit}>
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
