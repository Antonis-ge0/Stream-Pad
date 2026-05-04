import {
  AppWindow,
  Copy,
  FolderOpen,
  Globe,
  ImageIcon,
  Music4,
  Trash,
} from "lucide-react";
import type { DeckAction, DeckButton } from "@/domain/deck";
import { APP_CONFIG } from "@/config/appConfig";
import { ButtonIcon } from "@/components/deck/ButtonIcon";

const ACTION_OPTIONS = [
  { value: "openUrl", label: "URL", icon: Globe },
  { value: "launchApp", label: "App", icon: AppWindow },
  { value: "playSound", label: "Sound", icon: Music4 },
  { value: "openFolder", label: "Folder", icon: FolderOpen },
] as const;

type ButtonInspectorProps = {
  button: DeckButton;
  onChange: (button: DeckButton) => void;
  onDelete: () => void;
  onDuplicate: () => void;
  onOpenIconPicker: () => void;
};

export function ButtonInspector({
  button,
  onChange,
  onDelete,
  onDuplicate,
  onOpenIconPicker,
}: ButtonInspectorProps) {
  const firstAction = button.actions[0] ?? { type: "openUrl", url: "" };
  const actionType = firstAction.type;

  function updateFirstAction(patch: Partial<DeckAction>) {
    onChange({
      ...button,
      actions: [{ ...firstAction, ...patch } as DeckAction],
    });
  }

  function handleActionTypeChange(type: DeckAction["type"]) {
    if (type === "openUrl") {
      updateFirstAction({ type, url: "" });
    }

    if (type === "launchApp") {
      updateFirstAction({ type, path: "", args: [] });
    }

    if (type === "playSound") {
      updateFirstAction({ type, sound: "" });
    }

    if (type === "openFolder") {
      updateFirstAction({ type, path: "" });
    }
  }

  function setSoundFile(file: File | null) {
    if (!file) return;

    const audio = document.createElement("audio");
    const objectUrl = URL.createObjectURL(file);

    audio.src = objectUrl;

    audio.onloadedmetadata = () => {
      URL.revokeObjectURL(objectUrl);

      if (audio.duration > APP_CONFIG.maxSoundDurationSeconds) {
        alert("Sound must be 30 seconds or shorter.");
        return;
      }

      const reader = new FileReader();

      reader.onload = () => {
        updateFirstAction({
          type: "playSound",
          sound: reader.result as string,
        });
      };

      reader.readAsDataURL(file);
    };
  }

  return (
    <>
      <div className="inspectorHeader">
        <h2>Edit Page</h2>
      </div>

      <div className="previewCard" title="Preview Card">
        <ButtonIcon icon={button.icon} />
        <strong>{button.label || "Untitled"}</strong>
      </div>

      <div className="form">
        <label>Button Name</label>
        <input
          value={button.label}
          onChange={(e) => onChange({ ...button, label: e.target.value })}
          placeholder="Example: YouTube"
          title="Button Name"
        />

        <label>Icon</label>
        <button type="button" onClick={onOpenIconPicker} title="Choose an Icon">
          <ImageIcon size={16} /> Choose Icon
        </button>

        <label>Action</label>
        <div className="actionTypePicker" role="tablist" aria-label="Button Action">
          {ACTION_OPTIONS.map((option) => (
            <button
              key={option.value}
              type="button"
              className={`actionTypeButton ${
                actionType === option.value ? "active" : ""
              }`}
              onClick={() => handleActionTypeChange(option.value)}
              title={option.label}
              aria-pressed={actionType === option.value}
            >
              <option.icon size={16} strokeWidth={2} />
              <span>{option.label}</span>
            </button>
          ))}
        </div>

        {actionType === "openUrl" && (
          <>
            <label>URL</label>
            <input
              value={firstAction.url}
              onChange={(e) => updateFirstAction({ url: e.target.value })}
              placeholder="https://example.com"
              title="URL"
            />
          </>
        )}

        {actionType === "launchApp" && (
          <>
            <label>Application path</label>
            <input
              value={firstAction.path}
              onChange={(e) => updateFirstAction({ path: e.target.value })}
              placeholder="C:\\Program Files\\App\\app.exe"
              title="Application"
            />
          </>
        )}

        {actionType === "playSound" && (
          <>
            <label>Sound file</label>

            <label className="fileButton">
              Upload sound, max {APP_CONFIG.maxSoundDurationSeconds} seconds
              <input
                type="file"
                accept="audio/wav,audio/mpeg,audio/mp3,audio/ogg"
                onChange={(e) => setSoundFile(e.target.files?.[0] ?? null)}
              />
            </label>

            <input
              value={firstAction.sound}
              onChange={(e) => updateFirstAction({ sound: e.target.value })}
              placeholder="Or paste local file path"
              title="Sound File"
            />
          </>
        )}

        {actionType === "openFolder" && (
          <>
            <label>Folder path</label>
            <input
              value={firstAction.path}
              onChange={(e) => updateFirstAction({ path: e.target.value })}
              placeholder="C:\\Users\\PC\\Documents"
              title="Folder Path"
            />
          </>
        )}

        <div className="inspectorActions">
          <button onClick={onDuplicate} title="Duplicate Button">
            <Copy size={16} /> Duplicate
          </button>

          <button className="danger" onClick={onDelete} title="Delete Button">
            <Trash size={16} /> Delete
          </button>
        </div>
      </div>
    </>
  );
}
