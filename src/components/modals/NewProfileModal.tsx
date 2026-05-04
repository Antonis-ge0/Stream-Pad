import { useState } from "react";

type NewProfileModalProps = {
  onCancel: () => void;
  onCreate: (name: string) => void;
};

export function NewProfileModal({ onCancel, onCreate }: NewProfileModalProps) {
  const [name, setName] = useState("");

  function submit() {
    onCreate(name);
  }

  return (
    <div className="modalBackdrop" onMouseDown={onCancel}>
      <div className="confirmModal" onMouseDown={(e) => e.stopPropagation()}>
        <h2>New Profile</h2>

        <p>Choose a name for your new profile.</p>

        <input
          autoFocus
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Example: Streaming"
          title="Profile Name"
          onKeyDown={(e) => {
            if (e.key === "Enter") submit();
          }}
        />

        <div className="confirmActions" title="Cancel">
          <button onClick={onCancel}>Cancel</button>

          <button className="primary" title="Create" onClick={submit}>
            Create
          </button>
        </div>
      </div>
    </div>
  );
}
