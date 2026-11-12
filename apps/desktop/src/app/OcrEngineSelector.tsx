import { useEffect, useId, useState } from "react";
import { api } from "../bridge/api";
import type { OcrEngine, OcrEngineDetails } from "../bridge/types";
import "./OcrEngineSelector.css";

// Fallback catalog, used only when the engine cannot be queried. Live data from
// get_ocr_engines replaces it; keep the copy in step with crates/ps-core/src/engine.rs.
const DEFAULT_ENGINES: OcrEngineDetails[] = [
  {
    engine: "windows",
    name: "Windows OCR",
    availability: "available",
    sizeLabel: "Built-in (0 MB)",
    speedLabel: "Fast (~45ms/img)",
    badge: "Built-in",
    description: "Built into Windows 10 and 11. Nothing to download.",
    highlights: [],
    license: null,
    downloadSizeBytes: null,
  },
  {
    engine: "pp_ocr_v6",
    name: "PP-OCRv6",
    availability: "not_installed",
    sizeLabel: "~6.1 MB",
    speedLabel: "Ultra-Fast (~30ms/img)",
    badge: "Recommended",
    description: "Compact model pack for screenshots, PDFs and printed text.",
    highlights: [],
    license: "Apache 2.0",
    downloadSizeBytes: 6100000,
  },
  {
    engine: "indra",
    name: "Indra OCR",
    availability: "not_installed",
    sizeLabel: "~27 MB",
    speedLabel: "Deep (~120ms/img)",
    badge: "Multilingual & Skewed",
    description: "Stronger on skewed, rotated and multilingual text.",
    highlights: [],
    license: "Research & non-commercial",
    downloadSizeBytes: 27000000,
  },
  {
    engine: "tesseract",
    name: "Tesseract OCR",
    availability: "not_installed",
    sizeLabel: "External CLI",
    speedLabel: "Traditional (~150ms/img)",
    badge: "Extensible",
    description: "Open-source engine that uses your installed Tesseract.",
    highlights: [],
    license: "Apache 2.0",
    downloadSizeBytes: null,
  },
];

const ORDER: OcrEngine[] = ["windows", "pp_ocr_v6", "indra", "tesseract"];

function unavailableMessage(engine: OcrEngine, name: string): string {
  if (engine === "tesseract") {
    return "Tesseract OCR isn't installed yet. Install it first.";
  }
  if (engine === "windows") {
    return "Windows OCR needs an OCR language. Install opens Windows language settings.";
  }
  return `${name} isn't installed yet. Install it first.`;
}

export function OcrEngineSelector({
  value,
  onChange,
  disabled = false,
}: {
  value: OcrEngine;
  onChange: (engine: OcrEngine) => void | Promise<unknown>;
  disabled?: boolean;
}) {
  const headingId = useId();
  const [engines, setEngines] = useState<OcrEngineDetails[]>(DEFAULT_ENGINES);
  const [busyEngine, setBusyEngine] = useState<OcrEngine | null>(null);
  const [failure, setFailure] = useState<{ engine: OcrEngine; message: string } | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  // Returns the live catalog, or null when the engine cannot be reached.
  const loadEngines = async (): Promise<OcrEngineDetails[] | null> => {
    try {
      const live = await api.getOcrEngines();
      if (live && live.length > 0) {
        setEngines(live);
        return live;
      }
    } catch {
      // Keep the catalog already on screen.
    }
    return null;
  };

  useEffect(() => {
    void loadEngines();
  }, []);

  // Availability is re-read on every choice, so an engine installed while the page
  // is open can be selected without reloading.
  const select = async (engine: OcrEngine) => {
    if (disabled || busyEngine) return;
    setFailure(null);
    const catalog = (await loadEngines()) ?? engines;
    const info = catalog.find((entry) => entry.engine === engine);
    const name = info?.name ?? engine;
    if (info?.availability !== "available") {
      setFailure({ engine, message: unavailableMessage(engine, name) });
      return;
    }
    try {
      await onChange(engine);
      if (engine !== value) setNotice(`Switched to ${name}. All images are being reindexed in the background.`);
    } catch {
      setFailure({ engine, message: `Couldn't switch to ${name}. The current engine is unchanged.` });
    }
  };

  const download = async (engine: OcrEngine) => {
    if (disabled || busyEngine) return;
    const name = engines.find((entry) => entry.engine === engine)?.name ?? engine;
    setBusyEngine(engine);
    setFailure(null);
    try {
      await api.downloadOcrEngine(engine);
      await loadEngines();
      await onChange(engine);
      if (engine !== value) setNotice(`Installed ${name}. All images are being reindexed in the background.`);
    } catch {
      setFailure({ engine, message: `Couldn't install ${name}. The current engine is unchanged.` });
    } finally {
      setBusyEngine(null);
    }
  };

  const installTesseract = async () => {
    if (disabled || busyEngine) return;
    setBusyEngine("tesseract");
    setFailure(null);
    try {
      await api.installTesseractOcr();
      await loadEngines();
    } catch (error) {
      setFailure({
        engine: "tesseract",
        message: typeof error === "string" ? error : "Couldn't install Tesseract OCR.",
      });
    } finally {
      setBusyEngine(null);
    }
  };

  const openLanguageSettings = async () => {
    setFailure(null);
    try {
      await api.openWindowsLanguageSettings();
    } catch {
      setFailure({ engine: "windows", message: "Couldn't open Windows language settings." });
    }
  };

  // The one action for an engine that is not installed yet.
  const install = (engine: OcrEngine) => {
    if (engine === "tesseract") return installTesseract();
    if (engine === "windows") return openLanguageSettings();
    return download(engine);
  };

  const sorted = [...engines].sort((a, b) => ORDER.indexOf(a.engine) - ORDER.indexOf(b.engine));

  return (
    <div className="ocr-engines" data-testid="ocr-engine-options">
      {/* Native select kept for tests; the cards below are the interface. */}
      <select
        id="ocr-engine"
        className="sr-only"
        aria-hidden="true"
        tabIndex={-1}
        value={value}
        onChange={(event) => void select(event.currentTarget.value as OcrEngine)}
      >
        {sorted.map((info) => (
          <option
            key={info.engine}
            value={info.engine}
            disabled={info.availability !== "available"}
          >
            {info.name}
          </option>
        ))}
      </select>

      <div className="ocr-engines-head">
        <h3 className="ocr-engines-title" id={headingId}>
          OCR engines
        </h3>
        <p className="ocr-engines-lead">Used to read text in images and screenshots.</p>
      </div>

      {failure ? (
        <p className="field-error" role="alert">
          {failure.message}
        </p>
      ) : notice ? (
        <p className="ocr-engine-notice" role="status">
          {notice}
        </p>
      ) : null}

      <ul className="ocr-engine-list" aria-labelledby={headingId}>
        {sorted.map((info) => {
          const current = value === info.engine;
          const installed = info.availability === "available";
          return (
            <li key={info.engine} className={"ocr-engine" + (current ? " is-current" : "")}>
              <div className="ocr-engine-head">
                <span className="ocr-engine-name">{info.name}</span>
                <span className="badge badge-quiet">{info.badge}</span>
                {current ? <span className="ocr-engine-current">Current</span> : null}
              </div>
              <div className="ocr-engine-meta">
                <span className="badge badge-quiet">{info.speedLabel}</span>
                <span className="badge badge-quiet">{info.sizeLabel}</span>
              </div>
              <p className="ocr-engine-desc">{info.description}</p>
              {info.license ? <p className="ocr-engine-license">License: {info.license}</p> : null}
              <div className="ocr-engine-foot">
                <span className={"ocr-engine-status" + (installed ? " is-installed" : "")}>
                  {installed ? "Installed" : "Not installed"}
                </span>
                {installed ? (
                  current ? null : (
                    <button type="button" className="btn" disabled={disabled} onClick={() => void select(info.engine)}>
                      Select
                    </button>
                  )
                ) : (
                  <button
                    type="button"
                    className="btn btn-primary"
                    disabled={disabled || busyEngine !== null}
                    onClick={() => void install(info.engine)}
                  >
                    {busyEngine === info.engine ? (
                      <>
                        <span className="ocr-spinner" aria-hidden="true" />
                        Installing…
                      </>
                    ) : (
                      "Install"
                    )}
                  </button>
                )}
              </div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
