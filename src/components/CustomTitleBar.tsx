import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X } from "lucide-react";

const appWindow = getCurrentWindow();

export function CustomTitleBar() {
  return (
    <header className="customTitleBar" data-tauri-drag-region>
      <div className="titleBarDragArea" data-tauri-drag-region />

      <div className="windowControls">
        <button
          type="button"
          className="windowControlButton"
          aria-label="Minimize"
          onClick={() => appWindow.minimize()}
        >
          <Minus size={14} /> 
        </button>

        <button
          type="button"
          className="windowControlButton"
          aria-label="Maximize or restore"
          onClick={() => appWindow.toggleMaximize()}
        >
          <Square size={12} />
        </button>

        <button
          type="button"
          className="windowControlButton close"
          aria-label="Close"
          onClick={() => appWindow.hide()}
        >
          <X size={14} />
        </button>
      </div>
    </header>
  );
}
