// Pure global-shortcut parsing, validation and formatting. No React, no bridge
// calls: everything here is unit-testable.
//
// The stored string must be accepted by the Rust shell. The Tauri shell hands
// it to tauri-plugin-global-shortcut, which parses it with global-hotkey 0.8:
// modifiers first, tokens joined by "+", then exactly one key. Windows' "Win"
// modifier is spelled "Super" in that syntax, so Win/Windows/Meta aliases are
// accepted and normalised to "Super" on disk while the UI keeps showing "Win".

export interface HotkeyParts {
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
  win: boolean;
  /** Canonical key token accepted by the Rust parser, e.g. "Space", "K", "F5". */
  key: string;
}

export type HotkeyValidation =
  | { ok: true; parts: HotkeyParts; accelerator: string; display: string }
  | { ok: false; error: string };

export const DEFAULT_HOTKEY = "Alt+Space";

/** Shown next to hotkey capture controls: reserved combos and live behaviour. */
export const HOTKEY_CONFLICT_NOTE =
  "Windows reserves combinations such as Alt+Tab, Alt+F4 and Win+L. If another app already owns a shortcut, Windows rejects it and the previous shortcut stays active.";

const MODIFIER_ALIASES: Record<string, "ctrl" | "alt" | "shift" | "win"> = {
  ctrl: "ctrl",
  control: "ctrl",
  alt: "alt",
  option: "alt",
  shift: "shift",
  win: "win",
  windows: "win",
  super: "win",
  meta: "win",
  cmd: "win",
  command: "win",
};

const MODIFIER_DISPLAY: Record<"ctrl" | "alt" | "shift" | "win", string> = {
  ctrl: "Ctrl",
  alt: "Alt",
  shift: "Shift",
  win: "Win",
};

const MODIFIER_ACCELERATOR: Record<"ctrl" | "alt" | "shift" | "win", string> = {
  ctrl: "Ctrl",
  alt: "Alt",
  shift: "Shift",
  win: "Super",
};

const KEY_ALIASES: Record<string, string> = {
  space: "Space",
  tab: "Tab",
  enter: "Enter",
  return: "Enter",
  escape: "Escape",
  esc: "Escape",
  backspace: "Backspace",
  delete: "Delete",
  del: "Delete",
  insert: "Insert",
  ins: "Insert",
  home: "Home",
  end: "End",
  pageup: "PageUp",
  pagedown: "PageDown",
  up: "ArrowUp",
  down: "ArrowDown",
  left: "ArrowLeft",
  right: "ArrowRight",
  arrowup: "ArrowUp",
  arrowdown: "ArrowDown",
  arrowleft: "ArrowLeft",
  arrowright: "ArrowRight",
  printscreen: "PrintScreen",
  prtsc: "PrintScreen",
  capslock: "CapsLock",
  numlock: "NumLock",
  scrolllock: "ScrollLock",
  pause: "Pause",
  backquote: "\`",
  "\`": "\`",
  minus: "-",
  "-": "-",
  equal: "=",
  "=": "=",
  comma: ",",
  ",": ",",
  period: ".",
  ".": ".",
  slash: "/",
  "/": "/",
  backslash: "\\\\",
  "\\\\": "\\\\",
  semicolon: ";",
  ";": ";",
  quote: "'",
  "'": "'",
  bracketleft: "[",
  "[": "[",
  bracketright: "]",
  "]": "]",
  numpadadd: "NumpadAdd",
  numpadplus: "NumpadAdd",
  numpadsubtract: "NumpadSubtract",
  numpadmultiply: "NumpadMultiply",
  numpaddivide: "NumpadDivide",
  numpaddecimal: "NumpadDecimal",
  numpadenter: "NumpadEnter",
  numpadequal: "NumpadEqual",
};

const DISPLAY_KEY: Record<string, string> = {
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  PageUp: "PgUp",
  PageDown: "PgDn",
  Escape: "Esc",
  Backspace: "Bksp",
  Delete: "Del",
  Insert: "Ins",
  PrintScreen: "PrtSc",
  CapsLock: "Caps",
  ScrollLock: "ScrLk",
  NumLock: "NumLk",
};

interface ReservedCombo {
  ctrl?: boolean;
  alt?: boolean;
  shift?: boolean;
  win?: boolean;
  key: string;
  label: string;
}

// Combos Windows itself owns; RegisterHotKey will not succeed for these.
const RESERVED_COMBOS: ReservedCombo[] = [
  { alt: true, key: "Tab", label: "Alt+Tab" },
  { alt: true, key: "F4", label: "Alt+F4" },
  { ctrl: true, alt: true, key: "Delete", label: "Ctrl+Alt+Delete" },
  { ctrl: true, shift: true, key: "Escape", label: "Ctrl+Shift+Escape" },
  { win: true, key: "L", label: "Win+L" },
  { win: true, key: "D", label: "Win+D" },
  { win: true, key: "E", label: "Win+E" },
  { win: true, key: "R", label: "Win+R" },
  { win: true, key: "Tab", label: "Win+Tab" },
  { win: true, key: "X", label: "Win+X" },
];

function parseKeyToken(raw: string): string | null {
  const token = raw.trim();
  if (!token) return null;
  const lower = token.toLowerCase();
  if (/^[a-z]$/.test(lower)) return lower.toUpperCase();
  if (/^[0-9]$/.test(lower)) return lower;
  if (/^f([1-9]|1[0-2])$/.test(lower)) return lower.toUpperCase();
  if (/^digit([0-9])$/.test(lower)) return lower.slice(-1);
  if (/^key[a-z]$/.test(lower)) return lower.slice(-1).toUpperCase();
  if (/^numpad([0-9])$/.test(lower)) return "Numpad" + lower.slice(-1);
  if (/^num([0-9])$/.test(lower)) return "Numpad" + lower.slice(-1);
  return KEY_ALIASES[lower] ?? null;
}

function isFunctionKey(key: string): boolean {
  return /^F([1-9]|1[0-2])$/.test(key);
}

function matchesReserved(parts: HotkeyParts): ReservedCombo | null {
  const { ctrl, alt, shift, win } = parts;
  return (
    RESERVED_COMBOS.find(
      (combo) =>
        Boolean(combo.ctrl) === ctrl &&
        Boolean(combo.alt) === alt &&
        Boolean(combo.shift) === shift &&
        Boolean(combo.win) === win &&
        combo.key === parts.key,
    ) ?? null
  );
}

/** Parse and validate a user-entered shortcut. */
export function parseHotkey(raw: string): HotkeyValidation {
  const trimmed = raw.trim();
  if (!trimmed) return { ok: false, error: "Enter a shortcut, for example Alt+Space." };

  const tokens = trimmed.split("+");
  const flags = { ctrl: false, alt: false, shift: false, win: false };
  let key: string | null = null;

  for (const rawToken of tokens) {
    const token = rawToken.trim();
    if (!token) {
      return { ok: false, error: "Remove the extra “+” — a shortcut is modifiers plus one key." };
    }
    const modifier = MODIFIER_ALIASES[token.toLowerCase()];
    if (modifier) {
      if (key !== null) {
        return { ok: false, error: "Put modifiers first, for example Ctrl+Shift+K." };
      }
      flags[modifier] = true;
      continue;
    }
    if (key !== null) {
      return {
        ok: false,
        error: "A shortcut can have only one main key. Remove “" + token + "”.",
      };
    }
    const parsed = parseKeyToken(token);
    if (!parsed) {
      return {
        ok: false,
        error: "“" + token + "” is not a supported key. Use a letter, number, F1–F12, Space, Enter, Tab or an arrow key.",
      };
    }
    key = parsed;
  }

  if (key === null) {
    return { ok: false, error: "Add a main key after the modifiers, for example Alt+Space." };
  }
  const hasModifier = flags.ctrl || flags.alt || flags.shift || flags.win;
  if (!hasModifier && !isFunctionKey(key)) {
    return {
      ok: false,
      error: "Hold at least one modifier (Ctrl, Alt, Shift or Win). A single key would be captured system-wide.",
    };
  }

  const parts: HotkeyParts = { ...flags, key };
  const reserved = matchesReserved(parts);
  if (reserved) {
    return {
      ok: false,
      error: reserved.label + " is reserved by Windows. Choose a different combination.",
    };
  }

  const accelerator = buildAccelerator(parts);
  return { ok: true, parts, accelerator, display: buildDisplay(parts) };
}

function buildAccelerator(parts: HotkeyParts): string {
  const tokens: string[] = [];
  if (parts.ctrl) tokens.push(MODIFIER_ACCELERATOR.ctrl);
  if (parts.alt) tokens.push(MODIFIER_ACCELERATOR.alt);
  if (parts.shift) tokens.push(MODIFIER_ACCELERATOR.shift);
  if (parts.win) tokens.push(MODIFIER_ACCELERATOR.win);
  tokens.push(parts.key);
  return tokens.join("+");
}

function buildDisplay(parts: HotkeyParts): string {
  const tokens: string[] = [];
  if (parts.ctrl) tokens.push(MODIFIER_DISPLAY.ctrl);
  if (parts.alt) tokens.push(MODIFIER_DISPLAY.alt);
  if (parts.shift) tokens.push(MODIFIER_DISPLAY.shift);
  if (parts.win) tokens.push(MODIFIER_DISPLAY.win);
  tokens.push(DISPLAY_KEY[parts.key] ?? parts.key);
  return tokens.join("+");
}


/* ------------------------------------------------------------------------- */
/* Capture-mode keyboard chords                                              */
/* ------------------------------------------------------------------------- */

export interface KeyboardChordEvent {
  key: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

const MODIFIER_ONLY_KEYS = new Set([
  "Control",
  "Shift",
  "Alt",
  "Meta",
  "AltGraph",
  "CapsLock",
  "NumLock",
  "ScrollLock",
  "Fn",
  "OS",
]);

const NAMED_EVENT_KEYS: Record<string, string> = {
  Escape: "Escape",
  Enter: "Enter",
  Backspace: "Backspace",
  Delete: "Delete",
  Insert: "Insert",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  ArrowUp: "ArrowUp",
  ArrowDown: "ArrowDown",
  ArrowLeft: "ArrowLeft",
  ArrowRight: "ArrowRight",
  PrintScreen: "PrintScreen",
  Pause: "Pause",
};

const PUNCTUATION_EVENT_KEYS = new Set(["-", "=", ",", ".", "/", ";", "'", "[", "]", "\\", "\`"]);

function eventKeyToken(event: KeyboardChordEvent): string | null {
  const key = event.key;
  if (!key) return null;
  if (key === " ") return "Space";
  if (key === "Tab") return "Tab";
  if (key.length === 1) {
    if (/^[a-zA-Z]$/.test(key)) return key.toUpperCase();
    if (/^[0-9]$/.test(key)) return key;
    if (PUNCTUATION_EVENT_KEYS.has(key)) return key;
    return null;
  }
  if (/^F([1-9]|1[0-2])$/.test(key)) return key;
  return NAMED_EVENT_KEYS[key] ?? null;
}

/**
 * Turns a browser keydown into a validated accelerator while a capture control
 * is armed. Returns null for modifier-only presses (keep listening) and an
 * error for keys that cannot be part of a global shortcut.
 */
export function chordFromKeyboardEvent(event: KeyboardChordEvent): HotkeyValidation | null {
  if (MODIFIER_ONLY_KEYS.has(event.key)) return null;
  const token = eventKeyToken(event);
  if (!token) {
    return {
      ok: false,
      error:
        "That key can't be used. Use a letter, number, F1–F12, Space, Enter, Tab, an arrow key or punctuation.",
    };
  }
  const modifiers: string[] = [];
  if (event.ctrlKey) modifiers.push("Ctrl");
  if (event.altKey) modifiers.push("Alt");
  if (event.shiftKey) modifiers.push("Shift");
  if (event.metaKey) modifiers.push("Win");
  return parseHotkey([...modifiers, token].join("+"));
}

/** Convenience for build/save paths that only need the canonical string. */
export function normalizeHotkey(raw: string): string | null {
  const parsed = parseHotkey(raw);
  return parsed.ok ? parsed.accelerator : null;
}

/** Human-readable value for a stored accelerator, tolerant of old strings. */
export function displayHotkey(stored: string | null | undefined): string {
  if (!stored) return "";
  const parsed = parseHotkey(stored);
  return parsed.ok ? parsed.display : stored;
}

/** Individual keycaps for the visual preview in onboarding. */
export function hotkeyCapTokens(stored: string): string[] {
  const parsed = parseHotkey(stored);
  return parsed.ok ? parsed.display.split("+") : [];
}