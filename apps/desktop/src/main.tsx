import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource-variable/inter";
import App from "./App";
import { isTauriRuntime } from "./bridge/api";
import "./styles/base.css";
import "./styles/shell.css";

export type ViewKind = "launcher" | "main";

function resolveView(): ViewKind {
  const param = new URLSearchParams(window.location.search).get("view");
  if (param === "launcher" || param === "main") return param;
  // In the real app the window label is authoritative.
  if (isTauriRuntime()) {
    try {
      const label = (window as unknown as { __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label?: string } } } })
        .__TAURI_INTERNALS__?.metadata?.currentWindow?.label;
      if (label === "launcher") return "launcher";
    } catch {
      /* fall through */
    }
  }
  return "main";
}

const view = resolveView();
document.documentElement.dataset.window = view;
document.documentElement.dataset.theme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
document.documentElement.dataset.density = "comfortable";
document.documentElement.dataset.motion = "full";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App view={view} />
  </StrictMode>,
);
