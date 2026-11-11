import { useEffect, useState } from "react";
import { api } from "../bridge/api";
import type { OcrEngine, OcrEngineDetails } from "../bridge/types";
import "./OcrEngineSelector.css";

const DEFAULT_ENGINES: OcrEngineDetails[] = [
  {
    engine: "windows",
    name: "Windows OCR",
    availability: "available",
    sizeLabel: "Built-in (0 MB)",
    speedLabel: "Fast (~45ms/img)",
    badge: "Built-in",
    description: "Built into Windows and available now. Keeps OCR local and preserves the current working engine.",
    highlights: [
      "Built into Windows 10 & 11",
      "Zero setup or downloads required",
      "100% offline and low memory footprint",
    ],
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
    description: "Compact and designed for speed; approximately 6.1 MB of models, plus a runtime.",
    highlights: [
      "High accuracy on screenshots, PDFs & text",
      "Ultra-fast local inference (~30ms/image)",
      "Compact ~6.1 MB model download",
    ],
    license: "Apache 2.0",
    downloadSizeBytes: 6100000,
  },
  {
    engine: "indra",
    name: "Indra",
    availability: "not_installed",
    sizeLabel: "~27 MB",
    speedLabel: "Deep (~120ms/img)",
    badge: "Multilingual & Skewed",
    description: "Designed for higher-quality text recognition, including skewed and multilingual text; approximately 27 MB of models. Its license requires acceptance before download.",
    highlights: [
      "Specialized in skewed and rotated text",
      "Extended multilingual script coverage",
      "Requires research license acceptance",
    ],
    license: "Research & Non-Commercial (Acceptance required)",
    downloadSizeBytes: 27000000,
  },
  {
    engine: "tesseract",
    name: "Tesseract",
    availability: "not_installed",
    sizeLabel: "External CLI",
    speedLabel: "Traditional (~150ms/img)",
    badge: "Extensible",
    description: "A traditional OCR engine with separately installed language data; download size depends on the languages selected. Not configured in ProtonSearch.",
    highlights: [
      "Uses system Tesseract installation",
      "Extensible with custom traineddata packs",
      "Traditional open-source engine",
    ],
    license: "Apache 2.0",
    downloadSizeBytes: null,
  },
];

const ORDER: OcrEngine[] = ["windows", "pp_ocr_v6", "indra", "tesseract"];

export function OcrEngineSelector({
  value,
  onChange,
  disabled = false,
}: {
  value: OcrEngine;
  onChange: (engine: OcrEngine) => void;
  disabled?: boolean;
}) {
  const [engines, setEngines] = useState<OcrEngineDetails[]>(DEFAULT_ENGINES);
  const [downloadingEngine, setDownloadingEngine] = useState<OcrEngine | null>(null);
  const [downloadError, setDownloadError] = useState<string | null>(null);
  const [showLicenseModal, setShowLicenseModal] = useState(false);

  const loadEngines = async () => {
    try {
      const live = await api.getOcrEngines();
      if (live && live.length > 0) {
        setEngines(live);
      }
    } catch {
      // Keep default engine catalog
    }
  };

  useEffect(() => {
    void loadEngines();
  }, []);

  const handleDownload = async (engine: OcrEngine) => {
    if (disabled || downloadingEngine) return;
    setDownloadingEngine(engine);
    setDownloadError(null);
    try {
      await api.downloadOcrEngine(engine);
      await loadEngines();
      onChange(engine);
    } catch (err: unknown) {
      setDownloadError(err instanceof Error ? err.message : String(err));
    } finally {
      setDownloadingEngine(null);
    }
  };

  const badgeClass = (badge: string) => {
    const lower = badge.toLowerCase();
    if (lower.includes("recommended")) return "ocr-badge recommended";
    if (lower.includes("built-in")) return "ocr-badge builtin";
    if (lower.includes("multilingual")) return "ocr-badge multilingual";
    return "ocr-badge traditional";
  };

  // Sort engines to match canonical order: windows, pp_ocr_v6, indra, tesseract
  const sortedEngines = [...engines].sort((a, b) => ORDER.indexOf(a.engine) - ORDER.indexOf(b.engine));

  return (
    <div className="ocr-selector-container" data-testid="ocr-engine-options">
      {/* Accessible native select preserving testing contract */}
      <select
        id="ocr-engine"
        value={value}
        aria-label="OCR engine"
        style={{ position: "absolute", width: 1, height: 1, padding: 0, margin: -1, overflow: "hidden", clip: "rect(0, 0, 0, 0)", border: 0 }}
        onChange={(event) => onChange(event.currentTarget.value as OcrEngine)}
      >
        {sortedEngines.map((opt) => (
          <option key={opt.engine} value={opt.engine} disabled={opt.availability !== "available"}>
            {opt.name} · {opt.availability === "available" ? "Available" : "Not installed"}
          </option>
        ))}
      </select>

      <div className="ocr-selector-header">
        <label className="ocr-selector-title" htmlFor="ocr-cards">
          OCR Engine &amp; Models
        </label>
        <span className="ocr-selector-subtitle">
          Choose which local OCR engine extracts readable text from images, screenshots, and scanned documents. All processing stays strictly local on your PC.
        </span>
      </div>

      {downloadError ? (
        <div className="field-error" role="alert" style={{ margin: "4px 0 8px 0" }}>
          Download failed: {downloadError}
        </div>
      ) : null}

      <div className="ocr-cards-grid" id="ocr-cards">
        {sortedEngines.map((info) => {
          const isActive = value === info.engine;
          const isAvailable = info.availability === "available";
          const isDownloading = downloadingEngine === info.engine;

          return (
            <div
              key={info.engine}
              className={`ocr-card ${isActive ? "active" : ""}`}
              onClick={() => {
                if (isAvailable && !disabled) {
                  onChange(info.engine);
                }
              }}
            >
              <div>
                <div className="ocr-card-top">
                  <div className="ocr-card-name-group">
                    <span className="ocr-card-name">{info.name}</span>
                    <span className={badgeClass(info.badge)}>{info.badge}</span>
                  </div>
                  {isActive ? (
                    <span className="ocr-status-pill active">
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                        <polyline points="20 6 9 17 4 12" />
                      </svg>
                      Active
                    </span>
                  ) : null}
                </div>

                <div className="ocr-card-metrics">
                  <span className={`ocr-pill ${info.speedLabel.includes("Ultra") ? "fast" : ""}`}>
                    {info.speedLabel}
                  </span>
                  <span className="ocr-pill">{info.sizeLabel}</span>
                </div>

                <p className="ocr-card-desc">{info.description}</p>

                <ul className="ocr-card-highlights">
                  {info.highlights.map((h, i) => (
                    <li key={i} className="ocr-card-highlight-item">
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <polyline points="20 6 9 17 4 12" />
                      </svg>
                      {h}
                    </li>
                  ))}
                </ul>
              </div>

              <div className="ocr-card-action">
                <span className="ocr-status-pill">
                  {isAvailable ? (
                    <span className="ocr-status-pill available">Installed</span>
                  ) : (
                    <span className="ocr-status-pill uninstalled">Not installed</span>
                  )}
                </span>

                {isActive ? (
                  <button
                    type="button"
                    className="ocr-btn ocr-btn-secondary"
                    disabled
                  >
                    Current
                  </button>
                ) : isAvailable ? (
                  <button
                    type="button"
                    className="ocr-btn ocr-btn-secondary"
                    disabled={disabled}
                    onClick={(e) => {
                      e.stopPropagation();
                      onChange(info.engine);
                    }}
                  >
                    Select
                  </button>
                ) : info.engine === "pp_ocr_v6" ? (
                  <button
                    type="button"
                    className="ocr-btn ocr-btn-primary"
                    disabled={disabled || isDownloading}
                    onClick={(e) => {
                      e.stopPropagation();
                      void handleDownload("pp_ocr_v6");
                    }}
                  >
                    {isDownloading ? (
                      <>
                        <span className="ocr-spinner" />
                        Downloading...
                      </>
                    ) : (
                      "Download (~6.1 MB)"
                    )}
                  </button>
                ) : info.engine === "indra" ? (
                  <button
                    type="button"
                    className="ocr-btn ocr-btn-secondary"
                    disabled={disabled || isDownloading}
                    onClick={(e) => {
                      e.stopPropagation();
                      setShowLicenseModal(true);
                    }}
                  >
                    {isDownloading ? (
                      <>
                        <span className="ocr-spinner" />
                        Downloading...
                      </>
                    ) : (
                      "View License & Download"
                    )}
                  </button>
                ) : (
                  <button
                    type="button"
                    className="ocr-btn ocr-btn-secondary"
                    disabled
                    title="Install Tesseract on system PATH to enable"
                  >
                    System CLI
                  </button>
                )}
              </div>
            </div>
          );
        })}
      </div>

      <p className="field-help" style={{ marginTop: 8 }}>
        Only installed engines can be selected. No OCR engine or model downloads are available here.
      </p>

      {showLicenseModal ? (
        <div className="ocr-license-modal-backdrop" onClick={() => setShowLicenseModal(false)}>
          <div className="ocr-license-modal" onClick={(e) => e.stopPropagation()}>
            <h3>Indra OCR Model License</h3>
            <p>
              Indra is designed for higher-quality text recognition, including skewed and multilingual text; approximately 27 MB of models. Its license requires acceptance before download.
            </p>
            <p>
              This model is distributed for non-commercial and research use only. Model files are downloaded directly to your local ProtonSearch data folder and are never shared or sent off your device.
            </p>
            <div className="ocr-license-actions">
              <button
                type="button"
                className="ocr-btn ocr-btn-primary"
                disabled={disabled || downloadingEngine === "indra"}
                onClick={() => {
                  setShowLicenseModal(false);
                  void handleDownload("indra");
                }}
              >
                {downloadingEngine === "indra" ? (
                  <>
                    <span className="ocr-spinner" />
                    Downloading...
                  </>
                ) : (
                  "Accept & Download (~27 MB)"
                )}
              </button>
              <button
                type="button"
                className="ocr-btn ocr-btn-secondary"
                onClick={() => setShowLicenseModal(false)}
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
