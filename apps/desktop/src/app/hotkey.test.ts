import { describe, expect, it } from "vitest";
import {
  HOTKEY_CONFLICT_NOTE,
  displayHotkey,
  normalizeHotkey,
  parseHotkey,
} from "./hotkey";

describe("parseHotkey", () => {
  it("accepts the default shortcut", () => {
    const result = parseHotkey("Alt+Space");
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.accelerator).toBe("Alt+Space");
      expect(result.display).toBe("Alt+Space");
    }
  });

  it("normalises case, whitespace and modifier aliases", () => {
    const result = parseHotkey("  ctrl + shift + k ");
    expect(result).toEqual(
      expect.objectContaining({ ok: true, accelerator: "Ctrl+Shift+K", display: "Ctrl+Shift+K" }),
    );
  });

  it("stores the Windows key as Super for the Rust parser", () => {
    const result = parseHotkey("Win+Space");
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.accelerator).toBe("Super+Space");
      expect(result.display).toBe("Win+Space");
    }
  });

  it("canonicalises modifier order", () => {
    const result = parseHotkey("Super+Shift+K");
    expect(result.ok).toBe(true);
    if (result.ok) expect(result.accelerator).toBe("Shift+Super+K");
  });

  it("allows a bare function key", () => {
    const result = parseHotkey("f9");
    expect(result.ok).toBe(true);
    if (result.ok) expect(result.accelerator).toBe("F9");
  });

  it("rejects a bare letter because it would be captured system-wide", () => {
    const result = parseHotkey("K");
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toMatch(/modifier/);
  });

  it("rejects modifiers without a main key", () => {
    const result = parseHotkey("Ctrl+Alt");
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toMatch(/Add a main key/);
  });

  it("rejects empty tokens created by a double plus", () => {
    const result = parseHotkey("Ctrl++");
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toMatch(/extra/);
  });

  it("rejects more than one main key", () => {
    const result = parseHotkey("Ctrl+K+J");
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toMatch(/only one main key/);
  });

  it("rejects modifiers typed after the key", () => {
    const result = parseHotkey("Ctrl+K+Shift");
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toMatch(/modifiers first/);
  });

  it("rejects unsupported keys and out-of-range function keys", () => {
    const hyper = parseHotkey("Hyper+K");
    expect(hyper.ok).toBe(false);
    if (!hyper.ok) expect(hyper.error).toMatch(/not a supported key/);
    expect(parseHotkey("Ctrl+F13").ok).toBe(false);
  });

  it("rejects combinations Windows reserves", () => {
    const altTab = parseHotkey("Alt+Tab");
    expect(altTab.ok).toBe(false);
    if (!altTab.ok) expect(altTab.error).toMatch(/reserved by Windows/);

    expect(parseHotkey("Ctrl+Alt+Delete").ok).toBe(false);
    expect(parseHotkey("Super+L").ok).toBe(false);
  });

  it("accepts punctuation and named keys", () => {
    const semicolon = parseHotkey("Ctrl+;");
    expect(semicolon.ok).toBe(true);
    if (semicolon.ok) expect(semicolon.accelerator).toBe("Ctrl+;");

    const arrows = parseHotkey("Alt+ArrowUp");
    expect(arrows.ok).toBe(true);
    if (arrows.ok) {
      expect(arrows.accelerator).toBe("Alt+ArrowUp");
      expect(arrows.display).toBe("Alt+↑");
    }
  });

  it("collapses duplicate modifiers", () => {
    const result = parseHotkey("Ctrl+Ctrl+K");
    expect(result.ok).toBe(true);
    if (result.ok) expect(result.accelerator).toBe("Ctrl+K");
  });
});

describe("normalizeHotkey and displayHotkey", () => {
  it("returns null for invalid shortcuts", () => {
    expect(normalizeHotkey("not a shortcut")).toBeNull();
  });

  it("returns the canonical accelerator for valid shortcuts", () => {
    expect(normalizeHotkey("win+shift+s")).toBe("Shift+Super+S");
  });

  it("renders stored accelerators for humans", () => {
    expect(displayHotkey("Super+Space")).toBe("Win+Space");
    expect(displayHotkey("Alt+Space")).toBe("Alt+Space");
  });

  it("passes unknown stored strings through instead of hiding them", () => {
    expect(displayHotkey("legacy-value")).toBe("legacy-value");
    expect(displayHotkey("")).toBe("");
  });

  it("documents the registration conflict behaviour", () => {
    expect(HOTKEY_CONFLICT_NOTE).toMatch(/Win\+L/);
  });
});
