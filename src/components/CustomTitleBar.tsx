import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X } from "lucide-react";

const appWindow = getCurrentWindow();

export function CustomTitleBar() {
  const startDrag = async (event: React.MouseEvent<HTMLDivElement>) => {
    if (event.button !== 0) {
      return;
    }

    await appWindow.startDragging();
  };

  return (
    <header className="customTitleBar">
      <div
        className="titleBarDragArea"
        data-tauri-drag-region
        onMouseDown={startDrag}
      >
      </div>

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