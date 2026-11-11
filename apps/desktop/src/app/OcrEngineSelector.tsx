import { OCR_ENGINE_OPTIONS, type OcrEngine } from "../bridge/types";

const DETAILS: Record<OcrEngine, { name: string; description: string }> = {
  windows: {
    name: "Windows OCR",
    description: "Built into Windows and available now. Keeps OCR local and preserves the current working engine.",
  },
  pp_ocr_v6: {
    name: "PP-OCRv6",
    description: "Compact and designed for speed; approximately 6.1 MB of models, plus a runtime. Not installed yet.",
  },
  indra: {
    name: "Indra",
    description: "Designed for higher-quality text recognition, including skewed and multilingual text; approximately 27 MB of models. Its license requires acceptance before download.",
  },
  tesseract: {
    name: "Tesseract",
    description: "A traditional OCR engine with separately installed language data; download size depends on the languages selected. Not configured in ProtonSearch.",
  },
};

export function OcrEngineSelector({
  value,
  onChange,
}: {
  value: OcrEngine;
  onChange: (engine: OcrEngine) => void;
}) {
  const availability = new Map(OCR_ENGINE_OPTIONS.map((option) => [option.engine, option.availability]));

  return (
    <div className="ocr-engine-options" data-testid="ocr-engine-options">
      <label className="field-label" htmlFor="ocr-engine">OCR engine</label>
      <select
        id="ocr-engine"
        value={value}
        onChange={(event) => onChange(event.currentTarget.value as OcrEngine)}
      >
        {OCR_ENGINE_OPTIONS.map(({ engine }) => (
          <option key={engine} value={engine} disabled={availability.get(engine) !== "available"}>
            {DETAILS[engine].name}{availability.get(engine) === "available" ? " · Available" : " · Not installed"}
          </option>
        ))}
      </select>
      <ul className="field-help">
        {OCR_ENGINE_OPTIONS.map(({ engine }) => (
          <li key={engine}>
            <strong>{DETAILS[engine].name}:</strong> {DETAILS[engine].description}
          </li>
        ))}
      </ul>
      <p className="field-help">Only installed engines can be selected. No OCR engine or model downloads are available here.</p>
    </div>
  );
}
