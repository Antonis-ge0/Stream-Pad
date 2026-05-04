type ConfirmDeleteModalProps = {
  type: "button" | "buttons" | "profile";
  name: string;
  count?: number;
  onCancel: () => void;
  onConfirm: () => void;
};

export function ConfirmDeleteModal({
  type,
  name,
  count,
  onCancel,
  onConfirm,
}: ConfirmDeleteModalProps) {
  return (
    <div className="modalBackdrop" onMouseDown={onCancel}>
      <div className="confirmModal" onMouseDown={(e) => e.stopPropagation()}>
        <h2>Delete {type === "buttons" ? "buttons" : type}?</h2>

        <p>
          {type === "buttons" ? (
            <>
              Are you sure you want to delete <strong>{count ?? 0} buttons</strong>?
            </>
          ) : (
            <>
              Are you sure you want to delete <strong>{name || `this ${type}`}</strong>?
            </>
          )}
        </p>

        {type === "profile" && (
          <p className="warningText">
            This will also delete every button inside this profile. 
          </p>
        )}

        <div className="confirmActions">
          <button onClick={onCancel} title="Cancel">
            Cancel
          </button>

          <button className="danger" onClick={onConfirm} title="Delete">
            Delete
          </button>
        </div>
      </div>
    </div>
  );
}
