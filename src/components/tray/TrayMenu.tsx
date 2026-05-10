import { invoke } from "@tauri-apps/api/core";

export function TrayMenu() {
  const launch = () => {
    void invoke("launch_stream_pad_from_tray");
  };

  const quit = () => {
    void invoke("quit_stream_pad_from_tray");
  };

  return (
    <main className="tray-menu-shell" onMouseLeave={() => void invoke("hide_stream_pad_tray_menu")}>
      <button className="tray-menu-item" type="button" onClick={launch}>
        <span>Launch Stream Pad</span>
      </button>
      <button className="tray-menu-item tray-menu-item-danger" type="button" onClick={quit}>
        <span>Quit</span>
      </button>
    </main>
  );
}
