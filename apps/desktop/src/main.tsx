import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource-variable/inter";
import App from "./App";
import { api, isTauriRuntime } from "./bridge/api";
import "./styles/base.css";
import "./styles/shell.css";

export type ViewKind = "launcher" | "main" | "island";

function resolveView(): ViewKind {
  const param = new URLSearchParams(window.location.search).get("view");
  if (param === "launcher" || param === "main" || param === "island") return param;
  // In the real app the window label is authoritative.
  if (isTauriRuntime()) {
    try {
      const label = (window as unknown as { __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label?: string } } } })
        .__TAURI_INTERNALS__?.metadata?.currentWindow?.label;
      if (label === "launcher") return "launcher";
      if (label === "island") return "island";
    } catch {
      /* fall through */
    }
  }
  return "main";
}

const view = resolveView();

// Opt-in performance instrumentation (PROTONSEARCH_PERF_HOOKS=1 in the shell):
// mark first-paint after a launcher/island summon and after keyboard selection
// paints, so the benchmark harness can split Win32-visible vs first-pixel time.
if (isTauriRuntime()) {
  void import("@tauri-apps/api/event")
    .then(({ listen }) =>
      listen<{ mode: string }>("perf:show", () => {
        requestAnimationFrame(() => {
          requestAnimationFrame(() => {
            const label = view === "island" ? "island-first-paint" : "launcher-first-paint";
            void api.perfMark(label, Date.now()).catch(() => undefined);
          });
        });
      }),
    )
    .catch(() => undefined);
  document.addEventListener(
    "keydown",
    (event) => {
      if (view !== "launcher") return;
      if (event.key === "ArrowDown" || event.key === "ArrowUp" || event.key === "Enter") {
        requestAnimationFrame(() => {
          void api.perfMark("launcher-selection-painted", Date.now()).catch(() => undefined);
        });
      }
    },
    true,
  );
}
document.documentElement.dataset.window = view;
document.documentElement.dataset.theme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
document.documentElement.dataset.density = "comfortable";
document.documentElement.dataset.motion = "full";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App view={view} />
  </StrictMode>,
);