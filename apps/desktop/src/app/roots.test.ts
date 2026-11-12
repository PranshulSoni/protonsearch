import { describe, expect, it } from "vitest";
import type { RootConfig } from "../bridge/types";
import {
  ROOT_MODE_ORDER,
  addRoot,
  cycleRootMode,
  isAbsoluteWindowsPath,
  normalizeWindowsPath,
  removeRoot,
  rootDisplayName,
  rootsFromSuggestions,
  samePath,
  setRootMode,
  toggleRootEnabled,
  unusedSuggestions,
  validateExcludedPath,
  validateRootPath,
} from "./roots";

const ROOTS: RootConfig[] = [
  { path: "C:\\Users\\Me\\Documents", mode: "content", enabled: true },
  { path: "C:\\Users\\Me\\Desktop", mode: "names_only", enabled: true },
];

describe("cycleRootMode", () => {
  it("cycles names only -> content -> content and OCR -> names only", () => {
    expect(cycleRootMode("names_only")).toBe("content");
    expect(cycleRootMode("content")).toBe("content_and_ocr");
    expect(cycleRootMode("content_and_ocr")).toBe("names_only");
  });

  it("covers the documented mode order", () => {
    expect(ROOT_MODE_ORDER).toEqual(["names_only", "content", "content_and_ocr"]);
  });
});

describe("root selection", () => {
  it("toggles a root off and back on", () => {
    const off = toggleRootEnabled(ROOTS, "C:\\Users\\Me\\Documents");
    expect(off[0].enabled).toBe(false);
    const on = toggleRootEnabled(off, "C:\\Users\\Me\\Documents");
    expect(on[0].enabled).toBe(true);
  });

  it("treats a missing enabled flag as enabled", () => {
    const withoutFlag: RootConfig[] = [{ path: "C:\\Work", mode: "names_only" }];
    const toggled = toggleRootEnabled(withoutFlag, "C:\\Work");
    expect(toggled[0].enabled).toBe(false);
  });

  it("matches paths case-insensitively and ignores trailing separators", () => {
    const toggled = toggleRootEnabled(ROOTS, "c:/users/me/documents/");
    expect(toggled[0].enabled).toBe(false);
  });

  it("sets the mode for one root only", () => {
    const next = setRootMode(ROOTS, "C:\\Users\\Me\\Desktop", "content_and_ocr");
    expect(next[0].mode).toBe("content");
    expect(next[1].mode).toBe("content_and_ocr");
  });

  it("uses the leaf folder name as a display label", () => {
    expect(rootDisplayName("C:\\Users\\Me\\Documents")).toBe("Documents");
    expect(rootDisplayName("C:\\")).toBe("C:");
  });

  it("removes a root case-insensitively", () => {
    const next = removeRoot(ROOTS, "c:\\users\\me\\desktop");
    expect(next).toHaveLength(1);
    expect(next[0].path).toBe("C:\\Users\\Me\\Documents");
  });
});

describe("normalizeWindowsPath and absolute path validation", () => {
  it("trims quotes and trailing separators", () => {
    expect(normalizeWindowsPath('  "C:\\Users\\Me\\Documents\\"  ')).toBe(
      "C:\\Users\\Me\\Documents",
    );
  });

  it("converts forward slashes and collapses duplicate separators", () => {
    expect(normalizeWindowsPath("C:/Users//Me/Projects/")).toBe("C:\\Users\\Me\\Projects");
  });

  it("keeps a drive root intact", () => {
    expect(normalizeWindowsPath("C:")).toBe("C:\\");
    expect(normalizeWindowsPath("C:\\")).toBe("C:\\");
  });

  it("normalises UNC shares", () => {
    expect(normalizeWindowsPath("\\\\server\\share\\folder\\")).toBe(
      "\\\\server\\share\\folder",
    );
  });

  it("recognises absolute drive and UNC paths only", () => {
    expect(isAbsoluteWindowsPath("C:\\Users")).toBe(true);
    expect(isAbsoluteWindowsPath("C:\\")).toBe(true);
    expect(isAbsoluteWindowsPath("\\\\server\\share")).toBe(true);
    expect(isAbsoluteWindowsPath("C:Users")).toBe(false);
    expect(isAbsoluteWindowsPath("Documents")).toBe(false);
    expect(isAbsoluteWindowsPath("\\\\server")).toBe(false);
  });
});

describe("adding roots and exclusions", () => {
  it("adds a valid absolute path with the requested mode", () => {
    const result = addRoot(ROOTS, "D:\\Projects", "content");
    expect(result.ok).toBe(true);
    expect(result.roots).toHaveLength(3);
    expect(result.roots[2]).toEqual({ path: "D:\\Projects", mode: "content", enabled: true });
  });

  it("rejects a relative path with an actionable message", () => {
    const result = addRoot(ROOTS, "Documents");
    expect(result.ok).toBe(false);
    expect(result.error).toMatch(/full folder path/);
  });

  it("rejects duplicates case-insensitively", () => {
    const result = addRoot(ROOTS, "c:/users/me/documents/");
    expect(result.ok).toBe(false);
    expect(result.error).toMatch(/already in the list/);
  });

  it("rejects empty input", () => {
    expect(addRoot(ROOTS, "   ").ok).toBe(false);
  });

  it("validates excluded paths with a distinct duplicate message", () => {
    const duplicate = validateExcludedPath(["C:\\Temp"], "c:\\temp\\");
    expect(duplicate.ok).toBe(false);
    expect(duplicate.error).toMatch(/already excluded/);
    expect(validateExcludedPath([], "D:\\Cache").ok).toBe(true);
  });

  it("validates root paths without mutating the list", () => {
    const validation = validateRootPath(ROOTS, "D:\\Archive");
    expect(validation.ok).toBe(true);
    expect(validation.path).toBe("D:\\Archive");
    expect(ROOTS).toHaveLength(2);
  });

  it("compares paths by normalised key", () => {
    expect(samePath("C:\\Users\\Me\\", "c:/users/me")).toBe(true);
    expect(samePath("C:\\Users\\Me", "C:\\Users\\You")).toBe(false);
  });
});

describe("suggested roots", () => {
  const suggestions = [
    { path: "C:\\Users\\Me\\Documents", label: "Documents", exists: true, defaultMode: "content" as const },
    { path: "C:\\Users\\Me\\Pictures", label: "Pictures", exists: false, defaultMode: "content_and_ocr" as const },
  ];

  it("preselects existing suggestions with their default mode", () => {
    const roots = rootsFromSuggestions(suggestions);
    expect(roots).toEqual([
      { path: "C:\\Users\\Me\\Documents", mode: "content", enabled: true },
      { path: "C:\\Users\\Me\\Pictures", mode: "content_and_ocr", enabled: false },
    ]);
  });

  it("offers only existing suggestions that are not configured yet", () => {
    expect(unusedSuggestions(suggestions, ROOTS)).toEqual([]);
    const unused = unusedSuggestions(suggestions, [{ path: "C:\\Work", mode: "names_only" }]);
    expect(unused.map((entry) => entry.label)).toEqual(["Documents"]);
  });
});