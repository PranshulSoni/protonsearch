import React from "react";
import { createRoot } from "react-dom/client";
import { SpotlightWindow } from "./features/search/SpotlightWindow";
import { WidgetCardWindow } from "./features/widget-card/WidgetCardWindow";
import { DockShell } from "./components/dock/DockShell";
import "./design/reference.css";
import "./design/native.css";
import "./widgets/final/final-widgets.css";
import "./design/dock-final.css";
import "./design/dock-original.css";
const surface = new URLSearchParams(location.search).get("window") || "dock";
createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {surface === "search" ? (
      <SpotlightWindow />
    ) : surface === "widget" ? (
      <WidgetCardWindow />
    ) : (
      <DockShell settingsOnly={surface === "settings"} />
    )}
  </React.StrictMode>,
);
