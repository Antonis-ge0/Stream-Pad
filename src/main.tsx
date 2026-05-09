import React from "react";
import ReactDOM from "react-dom/client";
import App from "@/App";
import { TrayMenu } from "@/components/tray/TrayMenu";
import "@/styles/index.css";

const isTrayMenu = new URLSearchParams(window.location.search).has("trayMenu");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {isTrayMenu ? <TrayMenu /> : <App />}
  </React.StrictMode>,
);
