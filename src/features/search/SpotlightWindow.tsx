import { useEffect, useMemo, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { applyTheme, savedTheme } from "../../design/theme";
type Result = {
  id: string;
  title: string;
  detail: string;
  type: string;
  actions: string[];
};
type SearchResponse = {
  requestId: number;
  results: Result[];
  elapsedMs: number;
};
type Scope = "all" | "apps" | "files" | "folders" | "ocr";
type IconKind =
  | "terminal"
  | "vscode"
  | "spotify"
  | "app"
  | "window"
  | "folder"
  | "ocr"
  | "commit"
  | "code"
  | "clipboard"
  | "gear"
  | "tsx"
  | "typescript"
  | "javascript"
  | "cpp"
  | "rust"
  | "python"
  | "csharp"
  | "java"
  | "calculator"
  | "document"
  | "search";
const SCOPES: Array<{ id: Scope; label: string }> = [
  { id: "all", label: "All" },
  { id: "apps", label: "Apps" },
  { id: "files", label: "Files" },
  { id: "folders", label: "Folders" },
  { id: "ocr", label: "OCR" },
];
const KNOWN_PREFIXES = [
  "app:",
  "apps:",
  "file:",
  "files:",
  "folder:",
  "folders:",
  "ocr:",
  "img:",
  "image:",
  "screenshots:",
  "screenshot:",
  "commits:",
  "notes:",
  "clip:",
  "clipboard:",
  "memory:",
  "bookmarks:",
  "history:",
  "code:",
];
const PREFIX_SCOPE: Array<[string, Scope]> = [
  ["app:", "apps"],
  ["apps:", "apps"],
  ["file:", "files"],
  ["files:", "files"],
  ["folder:", "folders"],
  ["folders:", "folders"],
  ["ocr:", "ocr"],
  ["img:", "ocr"],
  ["image:", "ocr"],
  ["screenshots:", "ocr"],
  ["screenshot:", "ocr"],
];
function hasKnownPrefix(value: string) {
  const lower = value.trim().toLowerCase();
  return KNOWN_PREFIXES.some((prefix) => lower.startsWith(prefix));
}
function scopeForPrefix(value: string): Scope | null {
  const lower = value.trim().toLowerCase();
  return PREFIX_SCOPE.find(([prefix]) => lower.startsWith(prefix))?.[1] ?? null;
}
function scopeQuery(scope: Scope, value: string) {
  const query = value.trim();
  if (!query || hasKnownPrefix(query)) return query;
  if (scope === "apps") return `app:${query}`;
  if (scope === "files") return `file:${query}`;
  if (scope === "folders") return `folder:${query}`;
  if (scope === "ocr") return `ocr:${query}`;
  return query;
}
function parseCalcTitle(title: string) {
  const separator = title.lastIndexOf(" = ");
  if (separator < 0) return { expression: "", value: title.trim() };
  return {
    expression: title.slice(0, separator).trim(),
    value: title.slice(separator + 3).trim(),
  };
}
function resultIconKind(result: Result): IconKind {
  const title = result.title.trim();
  const lower = title.toLowerCase();
  const type = result.type.toUpperCase();
  if (/(terminal|powershell|cmd)/i.test(title)) return "terminal";
  if (lower.includes("visual studio code") || lower === "code" || lower.includes("vs code")) return "vscode";
  if (lower.includes("spotify")) return "spotify";
  if (result.type.toLowerCase() === "app") return "app";
  if (type === "WINDOW") return "window";
  if (type === "FOLDER") return "folder";
  if (type === "OCR") return "ocr";
  if (type === "COMMIT") return "commit";
  if (/\.tsx$/i.test(lower)) return "tsx";
  if (/\.ts$/i.test(lower)) return "typescript";
  if (/\.jsx?$/i.test(lower)) return "javascript";
  if (/\.(cpp|cc|cxx|hpp)$/i.test(lower)) return "cpp";
  if (/\.rs$/i.test(lower)) return "rust";
  if (/\.py$/i.test(lower)) return "python";
  if (/\.cs$/i.test(lower)) return "csharp";
  if (/\.java$/i.test(lower)) return "java";
  if (type === "CODE" || type === "CODE_CONTENT") return "code";
  if (type === "CLIPBOARD") return "clipboard";
  if (type === "ACTION" || type === "SYSTEM") return "gear";
  if (type === "CALC") return "calculator";
  if (type === "FILE" || type === "FILE_CONTENT" || type === "RECENT") return "document";
  return "document";
}
function Tile({
  fill,
  label,
  color = "#fff",
  size = 9,
}: {
  fill: string;
  label: string;
  color?: string;
  size?: number;
}) {
  return (
    <>
      <rect x="2" y="2" width="20" height="20" rx="4" fill={fill} />
      <text
        x="12"
        y="12"
        textAnchor="middle"
        dominantBaseline="central"
        fill={color}
        fontSize={size}
        fontWeight="700"
        fontFamily="Segoe UI, sans-serif"
      >
        {label}
      </text>
    </>
  );
}
function ResultIcon({ kind }: { kind: IconKind }) {
  switch (kind) {
    case "terminal":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <rect x="2" y="3" width="20" height="18" rx="4" fill="#0c1017" stroke="#26313d" />
          <path d="m7 9 3 3-3 3" fill="none" stroke="#4ade80" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
          <path d="M12.5 15H17" fill="none" stroke="#4ade80" strokeWidth="1.8" strokeLinecap="round" />
        </svg>
      );
    case "vscode":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <Tile fill="#007acc" label="V" size={11} />
        </svg>
      );
    case "spotify":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <circle cx="12" cy="12" r="9.5" fill="#1db954" />
          <path d="M7.5 9.4c3.2-.8 6.3-.4 9 1" fill="none" stroke="#0b1f12" strokeWidth="1.5" strokeLinecap="round" />
          <path d="M8.2 12.6c2.5-.6 5-.3 7.2.8" fill="none" stroke="#0b1f12" strokeWidth="1.4" strokeLinecap="round" />
          <path d="M8.9 15.6c1.8-.4 3.5-.2 5.1.6" fill="none" stroke="#0b1f12" strokeWidth="1.3" strokeLinecap="round" />
        </svg>
      );
    case "app":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <rect x="3" y="3" width="8" height="8" rx="2" fill="#cbd5e1" />
          <rect x="13" y="3" width="8" height="8" rx="2" fill="#94a3b8" />
          <rect x="3" y="13" width="8" height="8" rx="2" fill="#94a3b8" />
          <rect x="13" y="13" width="8" height="8" rx="2" fill="#cbd5e1" />
        </svg>
      );
    case "window":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <rect x="3.5" y="4.5" width="17" height="15" rx="2.5" fill="none" stroke="#cbd5e1" strokeWidth="1.6" />
          <path d="M3.5 9h17" stroke="#cbd5e1" strokeWidth="1.4" />
          <circle cx="6.8" cy="6.8" r=".8" fill="#cbd5e1" />
          <circle cx="9.4" cy="6.8" r=".8" fill="#cbd5e1" />
        </svg>
      );
    case "folder":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <path d="M3 7.5A1.5 1.5 0 0 1 4.5 6h5l2 2h8A1.5 1.5 0 0 1 21 9.5v8A1.5 1.5 0 0 1 19.5 19h-15A1.5 1.5 0 0 1 3 17.5v-10Z" fill="#f59e0b" />
          <path d="M3 10h18" stroke="#d97706" strokeWidth="1.1" />
        </svg>
      );
    case "ocr":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <path d="M4 8V5.5A1.5 1.5 0 0 1 5.5 4H8M16 4h2.5A1.5 1.5 0 0 1 20 5.5V8M20 16v2.5a1.5 1.5 0 0 1-1.5 1.5H16M8 20H5.5A1.5 1.5 0 0 1 4 18.5V16" fill="none" stroke="#38bdf8" strokeWidth="1.6" strokeLinecap="round" />
          <path d="M7 9h10M7 12h10M7 15h6" fill="none" stroke="#7dd3fc" strokeWidth="1.5" strokeLinecap="round" />
        </svg>
      );
    case "commit":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <circle cx="12" cy="12" r="3.2" fill="#a855f7" />
          <path d="M12 3v6M12 15v6" stroke="#a855f7" strokeWidth="2" strokeLinecap="round" />
        </svg>
      );
    case "code":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <rect x="3" y="3" width="18" height="18" rx="4" fill="#312e81" />
          <path d="m9 8-3 4 3 4M15 8l3 4-3 4" fill="none" stroke="#a5b4fc" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      );
    case "clipboard":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <rect x="5" y="4" width="14" height="17" rx="2" fill="#cbd5e1" />
          <rect x="8" y="2.5" width="8" height="4" rx="1.2" fill="#94a3b8" />
          <path d="M8 11h8M8 14h8M8 17h5" stroke="#475569" strokeWidth="1.3" strokeLinecap="round" />
        </svg>
      );
    case "gear":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <circle cx="12" cy="12" r="3.1" fill="none" stroke="#94a3b8" strokeWidth="1.7" />
          <path d="M12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M18.7 5.3l-2.1 2.1M7.4 16.6l-2.1 2.1" stroke="#94a3b8" strokeWidth="1.7" strokeLinecap="round" />
        </svg>
      );
    case "tsx":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <Tile fill="#3178c6" label="TSX" size={7.5} />
        </svg>
      );
    case "typescript":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <Tile fill="#3178c6" label="TS" />
        </svg>
      );
    case "javascript":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <Tile fill="#f7df1e" label="JS" color="#111827" />
        </svg>
      );
    case "cpp":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <Tile fill="#00599c" label="C++" size={7.5} />
        </svg>
      );
    case "rust":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <Tile fill="#b7410e" label="R" size={10} />
        </svg>
      );
    case "python":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <Tile fill="#3776ab" label="PY" color="#ffd43b" size={8} />
        </svg>
      );
    case "csharp":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <Tile fill="#68217a" label="C#" size={8.5} />
        </svg>
      );
    case "java":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <Tile fill="#e76f00" label="J" size={11} />
        </svg>
      );
    case "calculator":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <rect x="4" y="2.5" width="16" height="19" rx="3" fill="#10b981" />
          <path d="M7 8h10M7 12h3M14 12h3M7 16h3M14 16h3" stroke="#052e1f" strokeWidth="1.6" strokeLinecap="round" />
        </svg>
      );
    case "search":
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <circle cx="11" cy="11" r="6.5" fill="none" stroke="#cbd5e1" strokeWidth="1.8" />
          <path d="m16 16 4 4" stroke="#cbd5e1" strokeWidth="1.8" strokeLinecap="round" />
        </svg>
      );
    case "document":
    default:
      return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <path d="M7 3h6l4 4v14H7V3Z" fill="#e2e8f0" />
          <path d="M13 3v4h4" fill="#94a3b8" />
          <path d="M9 12h6M9 15h6M9 18h4" stroke="#64748b" strokeWidth="1.3" strokeLinecap="round" />
        </svg>
      );
  }
}
function SearchGlyph() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <circle cx="11" cy="11" r="6.5" fill="none" stroke="currentColor" strokeWidth="1.9" />
      <path d="m16 16 4 4" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" />
    </svg>
  );
}
export function SpotlightWindow() {
  const [query, setQuery] = useState("");
  const [scope, setScope] = useState<Scope>("all");
  const [results, setResults] = useState<Result[]>([]);
  const [selected, setSelected] = useState(0);
  const [searching, setSearching] = useState(false);
  const [error, setError] = useState("");
  const [themeError, setThemeError] = useState("");
  const [opened, setOpened] = useState(0);
  const sequence = useRef(0);
  const executing = useRef(false);
  const input = useRef<HTMLInputElement>(null);
  const itemsRef = useRef<Result[]>([]);
  const selectedRef = useRef(0);
  const calc = useMemo(() => results.find((result) => result.type === "CALC") ?? null, [results]);
  const rows = useMemo(() => results.filter((result) => result.type !== "CALC"), [results]);
  const items = useMemo(() => (calc ? [calc, ...rows] : rows), [calc, rows]);
  const hasCalc = calc !== null;
  const visibleRows = rows.length;
  const activeRow = calc ? selected - 1 : selected;
  const activeId = calc && selected === 0 ? "calc-hero" : rows[activeRow] ? `result-${activeRow}` : undefined;
  useEffect(() => {
    itemsRef.current = items;
  }, [items]);
  useEffect(() => {
    selectedRef.current = selected;
  }, [selected]);
  useEffect(() => {
    applyTheme("dark");
    let current = true;
    let request = 0;
    const loadTheme = () => {
      const id = ++request;
      void Promise.resolve().then(() => isTauri()
        ? invoke<unknown>("load_dock_settings")
        : JSON.parse(localStorage.getItem("protondock-layout") || "null"))
        .then((value) => {
          if (current && id === request) {
            applyTheme(savedTheme(value));
            setThemeError("");
          }
        })
        .catch((e) => { if (current && id === request) setThemeError(`Cannot load theme: ${String(e)}`); });
    };
    loadTheme();
    const pending = isTauri() ? listen("settings-updated", loadTheme) : null;
    window.addEventListener("storage", loadTheme);
    return () => {
      current = false;
      window.removeEventListener("storage", loadTheme);
      void pending?.then((unlisten) => unlisten());
    };
  }, []);
  useEffect(() => {
    const height = Math.min(500, Math.max(174, 112 + (hasCalc ? 78 : 0) + visibleRows * 46));
    void invoke("fit_search", { height }).catch(() => {});
  }, [hasCalc, visibleRows]);
  useEffect(() => {
    if (activeId) document.getElementById(activeId)?.scrollIntoView?.({ block: "nearest" });
  }, [activeId]);
  useEffect(() => {
    const max = Math.max(0, items.length - 1);
    if (selected > max) setSelected(max);
  }, [items.length, selected]);
  useEffect(() => {
    input.current?.focus();
    const unlisten = listen<string>("search-query", (e) => {
      updateQuery(e.payload);
      setOpened((value) => value + 1);
      input.current?.focus();
    });
    return () => {
      void unlisten.then((off) => off());
    };
  }, []);
  useEffect(() => {
    const id = ++sequence.current;
    let current = true;
    const priorId = itemsRef.current[selectedRef.current]?.id ?? null;
    setSearching(true);
    setError("");
    setResults([]);
    setSelected(0);
    const timer = setTimeout(() => {
      void invoke<SearchResponse>("search", {
        query: scopeQuery(scope, query),
        limit: 30,
        requestId: id,
      })
        .then((response) => {
          if (!current || id !== sequence.current) return;
          const nextCalc = response.results.find((result) => result.type === "CALC") ?? null;
          const nextRows = response.results.filter((result) => result.type !== "CALC");
          const nextItems = nextCalc ? [nextCalc, ...nextRows] : nextRows;
          const preserved = priorId ? nextItems.findIndex((result) => result.id === priorId) : -1;
          setResults(response.results);
          setSelected(preserved >= 0 ? preserved : 0);
          setSearching(false);
        })
        .catch((e) => {
          if (!current || id !== sequence.current) return;
          setError(String(e));
          setResults([]);
          setSelected(0);
          setSearching(false);
        });
    }, 100);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [query, scope, opened]);
  function updateQuery(next: string) {
    setQuery(next);
    if (hasKnownPrefix(next)) {
      setScope(scopeForPrefix(next) ?? "all");
    } else if (!next.trim()) {
      setScope("all");
    }
  }
  function dismiss() {
    setError("");
    void invoke("hide_search").catch((e) => setError(String(e)));
  }
  async function execute(result: Result) {
    if (executing.current) return;
    executing.current = true;
    setError("");
    try {
      await invoke("execute_search_result", { resultId: result.id });
    } catch (e) {
      setError(String(e));
    } finally {
      executing.current = false;
    }
  }
  const calcParts = calc ? parseCalcTitle(calc.title) : null;
  return (
    <section
      className="ps-spotlight native-spotlight"
      role="dialog"
      aria-label="ProtonSearch Spotlight"
      onKeyDown={(e) => {
        if (e.nativeEvent.isComposing) return;
        if (e.key === "Escape") {
          e.preventDefault();
          dismiss();
          return;
        }
        if (e.target !== input.current) return;
        if (e.key === "ArrowDown") {
          e.preventDefault();
          setSelected((index) => Math.max(0, Math.min(items.length - 1, index + 1)));
        }
        if (e.key === "ArrowUp") {
          e.preventDefault();
          setSelected((index) => Math.max(0, index - 1));
        }
        if (e.key === "Tab") {
          e.preventDefault();
          const index = SCOPES.findIndex((item) => item.id === scope);
          const next = SCOPES[(index + (e.shiftKey ? -1 : 1) + SCOPES.length) % SCOPES.length];
          setScope(next.id);
        }
        if (e.key === "Enter" && items[selected]) {
          e.preventDefault();
          void execute(items[selected]);
        }
      }}
    >
      <div className="ps-search-top">
        <span className="ps-search-brand">
          <SearchGlyph />
        </span>
        <input
          ref={input}
          className="ps-search-input"
          type="search"
          value={query}
          onChange={(e) => updateQuery(e.target.value)}
          placeholder="Search apps, files, folders, or OCR text..."
          aria-label="Search apps, files, folders, or OCR text"
          role="combobox"
          aria-expanded="true"
          aria-autocomplete="list"
          aria-controls="search-results"
          aria-activedescendant={activeId}
          autoComplete="off"
          spellCheck={false}
        />
      </div>
      <div className="ps-scope-bar" role="group" aria-label="Search scope">
        {SCOPES.map((item) => (
          <button
            key={item.id}
            type="button"
            className="ps-scope"
            aria-pressed={scope === item.id}
            onClick={() => {
              setScope(item.id);
              input.current?.focus();
            }}
          >
            {item.label}
          </button>
        ))}
      </div>
      <div className="ps-search-body">
        <div className="ps-results-column">
          {error && (
            <div className="ps-no-results ps-error" role="alert">
              {error}
            </div>
          )}
          <div
            className="ps-results"
            id="search-results"
            role="listbox"
            aria-label="Search results"
            aria-busy={searching}
          >
            {calc && calcParts && (
              <button
                id="calc-hero"
                type="button"
                className={`ps-calc-hero ${selected === 0 ? "is-selected" : ""}`}
                role="option"
                aria-selected={selected === 0}
                onMouseEnter={() => setSelected(0)}
                onClick={() => void execute(calc)}
              >
                {calcParts.expression && <span className="ps-calc-expression">{calcParts.expression}</span>}
                <span className="ps-calc-value">{calcParts.value}</span>
                <span className="ps-calc-action">Enter to copy</span>
              </button>
            )}
            {rows.map((result, index) => {
              const itemIndex = index + (calc ? 1 : 0);
              const selectedRow = selected === itemIndex;
              const icon = resultIconKind(result);
              return (
                <button
                  key={result.id}
                  id={`result-${index}`}
                  type="button"
                  className={`ps-result ${selectedRow ? "is-selected" : ""}`}
                  role="option"
                  aria-selected={selectedRow}
                  onMouseEnter={() => setSelected(itemIndex)}
                  onClick={() => void execute(result)}
                >
                  <span className="ps-result-ic" data-icon={icon}>
                    <ResultIcon kind={icon} />
                  </span>
                  <span className="ps-result-copy">
                    <b>{result.title}</b>
                    <small>{result.detail}</small>
                  </span>
                </button>
              );
            })}
          </div>
          {!error && !searching && !items.length && (
            <div className="ps-no-results" role="status">
              {query.trim() ? "No matching results" : "Type to search"}
            </div>
          )}
        </div>
      </div>
      {themeError && (
        <span className="ps-sr-only" aria-live="polite" title={themeError}>
          Theme unavailable
        </span>
      )}
    </section>
  );
}
