# Specification: Minimalist ProtonSearch Launcher & In-Process Math Engine

**Document ID**: SPEC-2026-11-07-LAUNCHER-MATH  
**Status**: Approved (Target Architecture)  
**Compliance**: `/antislop` Hard Gate, WCAG AA 4.5:1, Zero Em Dashes  
**Author**: Antigravity Pair-Programming Agent  

---

## 1. Executive Summary & Design Read

This specification defines the minimalist redesign of the ProtonSearch floating spotlight launcher and its in-process recursive-descent mathematical evaluation engine.

### Antislop Design Read
* **Page/Surface Kind**: Native Windows search launcher and quick command palette.
* **Target Audience**: Software engineers, technical builders, and keyboard power users.
* **Visual Language**: Minimalist Tahoe Glass: deep obsidian backing, hairline borders, structural whitespace, zero decorative clutter.
* **Dials**: `ENERGY 1 (Calm)` / `RHYTHM 1 (Disciplined)` / `MOTION 1 (Subtle 120ms transitions)`.

---

## 2. Minimalist UI Philosophy (Antislop Alignment)

1. **Restrained Visual Surfaces (R-01, R-10, R-13)**:
   * No loud rainbow or neon gradients. The window uses deep obsidian glass (`rgba(13, 17, 23, 0.92)`) with `backdrop-filter: blur(32px)`.
   * Inner containers use flat transparent or subtle surfaces (`rgba(255, 255, 255, 0.03)`). Stacked blur-on-blur effects are eliminated.
   * Glow is restricted solely to the active item border or focus outline. Zero floating orbs or background noise grids.

2. **Typography & Tabular Numerals (R-06, R-25)**:
   * Typeface: `Segoe UI Variable`, with fallback to system sans-serif.
   * Mathematical results and status indicators use `font-variant-numeric: tabular-nums` to prevent layout jitter during typing.
   * Contrast: All body and hint text strictly exceeds WCAG AA 4.5:1 ratio against the dark background.

3. **Quiet, Functional Badges (R-09, R-11)**:
   * Replaces decorative glowing capsule pills with understated monospace smallcaps labels (`APP`, `CODE`, `GIT`, `SYS`, `MATH`).
   * Quick-jump number keys display as concise keyboard badges (<kbd>Alt 1</kbd> .. <kbd>Alt 6</kbd>) only on the top visible matches.

4. **Zero Em Dashes (R-02 Hard Gate)**:
   * All copy, subtitles, path separators, and status messages strictly avoid the em dash character. Subtitles use middle dots (`·`), colons (`:`), or arrows (`>`).

---

## 3. Mathematical Operations Specification

The launcher search bar doubles as an instant zero-latency calculator matching the native `protonsearch/src/calculator.rs` engine.

### 3.1 Supported Grammar & Syntax

| Category | Operators / Forms | Example Input | Evaluated Output |
|---|---|---|---|
| **Arithmetic** | `+`, `-`, `*`, `×`, `/`, `÷` | `125 * 8` | `= 1,000` |
| **Precedence** | Parentheses `(`, `)` | `(40 + 10) * 3` | `= 150` |
| **Powers** | `^` (right-associative) | `2 ^ 8` | `= 256` |
| **Percentages** | `N% of M` | `15% of 340` | `= 51` |
| **Percent scaling**| `N%` | `250 * 20%` | `= 50` |
| **Roots & Magnitude** | `sqrt(x)`, `abs(x)` | `sqrt(144) + abs(-8)` | `= 20` |
| **Rounding** | `round(x)`, `floor(x)`, `ceil(x)` | `round(4.65)` | `= 5` |
| **Trigonometry** | `sin(deg)`, `cos(deg)`, `tan(deg)` | `sin(90) + cos(0)` | `= 2` |
| **Logarithms** | `log(x)` (base 10), `ln(x)` (natural) | `log(1000)` | `= 3` |
| **Constants** | `pi`, `π`, `e` | `2 * pi * 10` | `= 62.8318` |

### 3.2 Evaluation Lifecycle
1. **Trigger Condition**: Input contains mathematical operators (`+`, `-`, `*`, `/`, `^`, `%`), parentheses, or recognized function keywords (`sqrt`, `sin`, `pi`), alongside at least one numeric character or constant.
2. **Parser Architecture**: In-process recursive-descent parser (`tokenize` -> `parse_expr` -> `parse_term` -> `parse_power` -> `parse_unary` -> `parse_primary`).
3. **Safety & Bounds**:
   * Guards against stack overflow on pathological nesting (>128 levels).
   * Rejects division by zero (`return null`).
   * Ignores comma thousand separators in input (e.g. `1,000 + 500`).
4. **Output Presentation**:
   * Displays as an elevated clean calculation card at the top of results.
   * Formats numbers with comma thousands separators (`Intl.NumberFormat`).
   * Displays full precision up to 8 decimal places with trailing zero trimming.

### 3.3 Keyboard Interaction
* **Enter (<kbd>↵</kbd>)**: Copies the evaluated value to the Windows clipboard, shows a quiet frosted toast notification, and closes the launcher (or allows chained input).
* **Click Action**: Clicking the calculation hero triggers the copy event.

---

## 4. Delivery & Verification Criteria

* **Functional Completeness (C-2)**: Evaluates complex combined expressions (e.g. `sqrt(144) + 15% of 200 = 42`).
* **Resilience (C-4)**: Empty queries show clear minimalist quick-starts; invalid math expressions fail gracefully without layout shifts or console errors.
* **Accessibility (R-32, R-39)**: Full keyboard navigation (<kbd>↑↓</kbd>, <kbd>Ctrl+N/P</kbd>, <kbd>Alt+1..6</kbd>, <kbd>Ctrl+K</kbd>, <kbd>Tab</kbd>, <kbd>Esc</kbd>).
