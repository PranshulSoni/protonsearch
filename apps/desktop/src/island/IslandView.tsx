import { useEffect, useRef } from "react";
import { api } from "../bridge/api";
import "./island.css";

/**
 * Dynamic Island host. The V9 surface itself is owned by the island design
 * system (see docs/design/ISLAND_V9_SPEC.md); this host only owns the native
 * window contract: measure the rendered surface, resize the transparent window
 * around it, and dismiss on Escape until the real cards land.
 */
export default function IslandView() {
  const surfaceRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const el = surfaceRef.current;
    if (!el) return;
    const apply = () => {
      const rect = el.getBoundingClientRect();
      const width = Math.ceil(Math.max(240, rect.width));
      const height = Math.ceil(Math.max(40, rect.height));
      void api.setIslandGeometry(width, height).catch(() => undefined);
    };
    apply();
    const observer = new ResizeObserver(apply);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void api.hideIsland().catch(() => undefined);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div className="island-host">
      <div className="island-surface island-surface--placeholder" ref={surfaceRef}>
        <span className="island-brand" aria-hidden="true">
          P
        </span>
        <span className="island-placeholder-text">Search…</span>
      </div>
    </div>
  );
}
