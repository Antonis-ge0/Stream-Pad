import { ChevronLeft, X } from "lucide-react";
import { EMOJI_ICONS } from "@/constants/icons";
import type { DeckButton } from "@/domain/deck";

type IconPickerModalProps = {
  button: DeckButton;
  onClose: () => void;
  onChange: (button: DeckButton) => void;
};

export function IconPickerModal({ button, onClose, onChange }: IconPickerModalProps) {
  function setImageIcon(file: File | null) {
    if (!file) return;

    const reader = new FileReader();

    reader.onload = () => {
      onChange({
        ...button,
        icon: reader.result as string,
      });

      onClose();
    };

    reader.readAsDataURL(file);
  }

  return (
    <div className="modalBackdrop" onMouseDown={onClose}>
      <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modalHeader">
          <button className="drawerBack" title="Back" onMouseDown={onClose}>
            <ChevronLeft size={18} />
          </button>

          <h2>Choose Icon</h2>

          <button title="Close" onClick={onClose}>
            <X size={18} />
          </button>
        </div>

        <h3>Emoji Icons</h3>
        <div className="iconPresetGrid" title="Emoji Icon">
          {EMOJI_ICONS.map((icon) => (
            <button
              key={icon}
              className="iconPreset"
              onClick={() => {
                onChange({ ...button, icon });
                onClose();
              }}
            >
              {icon}
            </button>
          ))}
        </div>

        <h3>Upload Image</h3>
        <label className="fileButton">
          Upload PNG / JPG / SVG
          <input
            type="file"
            accept="image/*"
            onChange={(e) => setImageIcon(e.target.files?.[0] ?? null)}
          />
        </label>

        <h3>Paste Image URL or Emoji</h3>
        <input
          value={button.icon ?? ""}
          onChange={(e) => onChange({ ...button, icon: e.target.value })}
          placeholder="https://... or 🎮"
          title="Image URL or Emoji"
        />

        <h3 />

        <button
          title="Clear the Icon"
          onClick={() => onChange({ ...button, icon: null })}
        >
          Clear Icon
        </button>
      </div>
    </div>
  );
}
