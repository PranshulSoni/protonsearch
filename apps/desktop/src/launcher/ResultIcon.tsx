import { useState, type ReactElement } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { isTauriRuntime } from "../bridge/api";
import type { SearchResult, SourceKind } from "../bridge/types";

/**
 * Result icon tile: real shell icon when the engine supplies an IconRef
 * (Tauri app_icon), otherwise a deterministic letter/glyph badge.
 * Binding: docs/design/DESIGN_SYSTEM.md §9.2 (no unicode glyph icons).
 */

type Tone = "word" | "code" | "folder" | "action" | "image" | "neutral";

const WORD_EXTS = new Set([
  "doc", "docx", "rtf", "odt", "pdf", "ppt", "pptx", "xls", "xlsx", "csv", "txt", "epub", "one", "msg",
]);
const CODE_EXTS = new Set([
  "md", "mdx", "js", "jsx", "ts", "tsx", "json", "jsonc", "py", "rs", "go", "java", "c", "h", "cpp", "hpp",
  "cs", "rb", "php", "sh", "ps1", "bat", "cmd", "yml", "yaml", "toml", "ini", "cfg", "conf", "sql", "html",
  "css", "scss", "less", "xml", "tsv", "log",
]);
const IMAGE_EXTS = new Set(["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "ico", "tif", "tiff", "heic"]);

const TONE_BY_KIND: Partial<Record<SourceKind, Tone>> = {
  application: "action",
  command: "action",
  action: "action",
  calculator: "action",
  folder: "folder",
  document: "word",
  content_match: "word",
  ocr_text: "image",
  clipboard: "action",
  browser_history: "action",
  bookmark: "action",
  quicklink: "action",
  snippet: "action",
  git_commit: "code",
  git_repository: "code",
  git_todo: "code",
  window: "action",
  setting: "action",
  agent: "action",
  system: "neutral",
  file: "neutral",
  recent: "neutral",
};

function extensionOf(result: SearchResult): string | null {
  if (result.icon.type === "file_type") {
    const ext = result.icon.ext.toLowerCase().replace(/^\./, "");
    return ext || null;
  }
  if (result.icon.type === "kind") {
    return null;
  }
  if (result.path) {
    const match = /\.([a-zA-Z0-9]{1,8})$/.exec(result.path);
    return match ? match[1].toLowerCase() : null;
  }
  return null;
}

function toneFor(result: SearchResult): Tone {
  const ext = extensionOf(result);
  if (ext) {
    if (ext === "folder") return "folder";
    if (IMAGE_EXTS.has(ext)) return "image";
    if (WORD_EXTS.has(ext)) return "word";
    if (CODE_EXTS.has(ext)) return "code";
  }
  if (result.icon.type === "file_type" && result.icon.ext.toLowerCase() === "folder") return "folder";
  return TONE_BY_KIND[result.kind] ?? "neutral";
}

function initials(title: string): string {
  const words = title.trim().split(/\s+/).filter(Boolean);
  if (words.length === 0) return "?";
  if (words.length === 1) return words[0].slice(0, 2).toUpperCase();
  return (words[0][0] + words[1][0]).toUpperCase();
}

function badgeText(result: SearchResult): string | null {
  switch (result.kind) {
    case "application":
      return initials(result.title);
    case "folder":
      return null;
    case "calculator":
    case "action":
    case "command":
    case "clipboard":
    case "browser_history":
    case "bookmark":
    case "quicklink":
    case "snippet":
    case "git_commit":
    case "git_repository":
    case "git_todo":
    case "window":
    case "setting":
    case "agent":
    case "system":
    case "ocr_text":
      return null;
    default:
      break;
  }
  const ext = extensionOf(result);
  if (ext) return ext.slice(0, 3).toUpperCase();
  return initials(result.title);
}

type GlyphName = "folder" | "command" | "arrow" | "clipboard" | "globe" | "code" | "calculator" | "image" | "settings" | "file";

function glyphFor(result: SearchResult): GlyphName | null {
  // File-ish results use the deterministic extension badge (DESIGN_SYSTEM §9.2);
  // glyphs are reserved for semantic kinds (folders, apps, actions, web, git, OCR).
  if (
    result.kind === "file" ||
    result.kind === "document" ||
    result.kind === "content_match" ||
    result.kind === "recent"
  ) {
    return null;
  }
  switch (result.kind) {
    case "folder":
      return "folder";
    case "application":
    case "window":
      // Real app icons render as images; the fallback is the app's initials.
      return null;
    case "command":
      return "command";
    case "action":
    case "quicklink":
    case "snippet":
      return "arrow";
    case "calculator":
      return "calculator";
    case "clipboard":
      return "clipboard";
    case "browser_history":
    case "bookmark":
      return "globe";
    case "git_commit":
    case "git_repository":
    case "git_todo":
      return "code";
    case "setting":
      return "settings";
    case "ocr_text":
      return "image";
    default:
      break;
  }
  return null;
}

const GLYPH_PATHS: Record<GlyphName, ReactElement> = {
  folder: <path d="M3 7.5A2.5 2.5 0 0 1 5.5 5h3.2l1.8 2H18.5A2.5 2.5 0 0 1 21 9.5v7A2.5 2.5 0 0 1 18.5 19h-13A2.5 2.5 0 0 1 3 16.5Z" />,
  command: (
    <>
      <rect x="3.5" y="4.5" width="17" height="15" rx="2" />
      <path d="m7.5 9 3 3-3 3" />
      <path d="M13 15.5h4" />
    </>
  ),
  arrow: (
    <>
      <path d="M7 17 17 7" />
      <path d="M9 7h8v8" />
    </>
  ),
  clipboard: (
    <>
      <rect x="5.5" y="5" width="13" height="16" rx="2" />
      <path d="M9 5V3.5h6V5" />
      <path d="M9 10h6M9 14h4" />
    </>
  ),
  globe: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M3.5 12h17" />
      <path d="M12 3.5c2.6 2.4 4 5.3 4 8.5s-1.4 6.1-4 8.5c-2.6-2.4-4-5.3-4-8.5s1.4-6.1 4-8.5Z" />
    </>
  ),
  code: <path d="m9 7-5 5 5 5M15 7l5 5-5 5" />,
  calculator: (
    <>
      <rect x="5" y="3.5" width="14" height="17" rx="2" />
      <path d="M8.5 7.5h7M8.5 12h2M13.5 12h2M8.5 16h2M13.5 16h2" />
    </>
  ),
  image: (
    <>
      <rect x="3.5" y="4.5" width="17" height="15" rx="2" />
      <circle cx="9" cy="10" r="1.6" />
      <path d="m4.5 17 4.6-4.4 3.1 3 3.3-3.2 3.9 3.8" />
    </>
  ),
  settings: (
    <>
      <circle cx="12" cy="12" r="3.2" />
      <path d="M12 3.5v2.2M12 18.3v2.2M3.5 12h2.2M18.3 12h2.2M6 6l1.6 1.6M16.4 16.4 18 18M18 6l-1.6 1.6M7.6 16.4 6 18" />
    </>
  ),
  file: (
    <>
      <path d="M6.5 3.5h7.6L19 8.4v12.1H6.5Z" />
      <path d="M14 3.5V8.5h5" />
      <path d="M9 12h7M9 15.5h5" />
    </>
  ),
};

/** Only safe, renderable image data URLs are passed to <img>. */
function isRenderableIconData(value: string | null | undefined): value is string {
  return (
    typeof value === "string" &&
    value.length > 64 &&
    /^data:image\/(?:png|jpe?g|webp|gif|svg\+xml);base64,/i.test(value.slice(0, 64))
  );
}

export function ResultIcon({ result, large = false }: { result: SearchResult; large?: boolean }) {
  const [iconFailed, setIconFailed] = useState(false);
  const tone = toneFor(result);
  const className = "fileicon fileicon--" + tone + (large ? " fileicon--large" : "");

  if (result.meta?.thumbnailDataUrl && !iconFailed) {
    return (
      <span className={className + " fileicon--real fileicon--thumb"} data-testid="result-icon" aria-hidden="true">
        <img
          src={result.meta.thumbnailDataUrl}
          alt=""
          width={large ? 32 : 28}
          height={large ? 32 : 28}
          style={{ objectFit: "cover", borderRadius: "4px" }}
          draggable={false}
          decoding="async"
          loading="lazy"
          onError={() => setIconFailed(true)}
        />
      </span>
    );
  }

  // Real app icon sources: IPC app_icon_data (data URL, preferred — no asset
  // scope issues) or the legacy app_icon path via convertFileSrc. Any decode
  // failure falls back to the deterministic monogram/letter badge.
  const dataIcon =
    result.icon.type === "app_icon_data" && isRenderableIconData(result.icon.data)
      ? result.icon.data
      : null;
  const fileIconSrc =
    result.icon.type === "app_icon" && isTauriRuntime()
      ? convertFileSrc(result.icon.path)
      : null;
  const appIconSrc = dataIcon ?? fileIconSrc;
  if (appIconSrc && !iconFailed) {
    return (
      <span className={className + " fileicon--real"} data-testid="result-icon" aria-hidden="true">
        <img
          src={appIconSrc}
          alt=""
          width={large ? 32 : 28}
          height={large ? 32 : 28}
          draggable={false}
          decoding="async"
          loading="lazy"
          data-icon-source={dataIcon ? "data" : "path"}
          onError={() => setIconFailed(true)}
        />
      </span>
    );
  }

  const glyph = glyphFor(result);
  const text = glyph ? null : badgeText(result);
  return (
    <span className={className} data-testid="result-icon" aria-hidden="true">
      {glyph ? (
        <svg className="fileicon-glyph" width={18} height={18} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" strokeLinejoin="round">
          {GLYPH_PATHS[glyph]}
        </svg>
      ) : (
        <span className="fileicon-text">{text ?? ""}</span>
      )}
    </span>
  );
}
