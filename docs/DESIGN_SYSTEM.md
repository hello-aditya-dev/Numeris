# Numeris — Design System (implemented reference)

**Status:** Reference for the design system as implemented in
`apps/desktop/src`
**Primary rule:** CONSISTENCY > novelty

Numeris is professional statistical software, not a marketing dashboard. The
interface is calm, precise, dense but readable, structured, fast,
keyboard-first, content-first and visually restrained. The feeling target is
**Apple software + VS Code + Bloomberg + spreadsheet + statistical
laboratory**; the visual restraint is closer to X.com than to contemporary
SaaS dashboards.

Visual formula:

```text
MONOCHROME FOUNDATION + STRONG TYPOGRAPHY + THIN STRUCTURAL DIVIDERS
+ COMPACT INFORMATION DENSITY + RESTRAINED BLUE ACCENT + SMALL RADII
+ MINIMAL SHADOW + SYSTEM-NATIVE INTERACTION + KEYBOARD-FIRST POWER
+ DATA-FIRST LAYOUT
```

The result should feel: **quiet, expensive, technical, trustworthy, fast.**

---

## 1. Color tokens

The system is monochrome-first with one restrained accent. Dark mode is a
true token remapping (same hierarchy: canvas → primary → secondary →
elevated), not a second visual system. All values are defined once as CSS
custom properties and consumed semantically — never hard-coded per screen.

### 1.1 Light mode

| Token | Value | Use |
|---|---|---|
| `background.canvas` | `#f7f7f8` | App background |
| `background.primary` | `#ffffff` | Primary content surface |
| `background.secondary` | `#f2f2f3` | Sidebar / secondary surface |
| `background.tertiary` | `#ebebee` | Tertiary strips, table headers |
| `background.elevated` | `#ffffff` | Transient surfaces (popovers, menus, modals) |
| `border.subtle` | `#e8e8ea` | 1px subtle dividers between quiet regions |
| `border.default` | `#dcdce0` | 1px default control/table borders |
| `border.strong` | `#b9b9c0` | 1px strong borders (selected rows, resize handles) |
| `text.primary` | `#1a1a1e` | Primary text |
| `text.secondary` | `#5c5c66` | Secondary text, labels |
| `text.tertiary` | `#8e8e99` | Tertiary text, placeholders, footnotes |
| `text.disabled` | `#a2a2ac` | Disabled text |
| `icon.primary` | `#1a1a1e` | Primary icons |
| `icon.secondary` | `#5c5c66` | Secondary icons |
| `accent.primary` | `#2361d9` | Accent: actions, selection, focus, links |
| `accent.hover` | `#1c51bd` | Accent hover |
| `accent.active` | `#17428f` | Accent pressed |
| `accent.subtle` | `#e8effc` | Accent-tinted surface (selection background) |
| `accent.text` | `#1c51bd` | Accent-colored text |

### 1.2 Dark mode

Same hierarchy, never pure black; borders are subtle luminance steps rather
than bright outlines.

| Token | Value | Use |
|---|---|---|
| `background.canvas` | `#131316` | App background |
| `background.primary` | `#1a1a1f` | Primary content surface |
| `background.secondary` | `#202027` | Sidebar / secondary surface |
| `background.tertiary` | `#26262e` | Tertiary strips, table headers |
| `background.elevated` | `#222229` | Transient surfaces (popovers, menus, modals) |
| `border.subtle` | `#232329` | 1px subtle dividers |
| `border.default` | `#2d2d36` | 1px default borders |
| `border.strong` | `#43434f` | 1px strong borders |
| `text.primary` | `#ededf2` | Primary text |
| `text.secondary` | `#a5a5b0` | Secondary text, labels |
| `text.tertiary` | `#767681` | Tertiary text, placeholders, footnotes |
| `text.disabled` | `#5c5c66` | Disabled text |
| `icon.primary` | `#ededf2` | Primary icons |
| `icon.secondary` | `#a5a5b0` | Secondary icons |
| `accent.primary` | `#6f9df5` | Accent |
| `accent.hover` | `#85b0f7` | Accent hover |
| `accent.active` | `#5785e2` | Accent pressed |
| `accent.subtle` | `#1d2740` | Accent-tinted surface (selection background) |
| `accent.text` | `#8fb4f9` | Accent-colored text |

### 1.3 Semantic state colors

Color is reserved for meaning — never decoration, never color-only signals
(status always pairs text/icon with color).

| Token | Light | Dark | Meaning |
|---|---|---|---|
| `state.success` | `#1a7f37` | `#3fb950` | Valid / successful operation |
| `state.success.subtle` | `#e6f4ea` | `#12261e` | Success surface |
| `state.warning` | `#9a6700` | `#d29922` | Attention required |
| `state.warning.subtle` | `#fff8c5` | `#2d2408` | Warning surface |
| `state.error` | `#cf222e` | `#f85149` | Error / invalid / destructive |
| `state.error.subtle` | `#ffebe9` | `#361a1c` | Error surface |
| `state.info` | `#2361d9` | `#6f9df5` | Information (accent) |
| `state.info.subtle` | `#e8effc` | `#1d2740` | Info surface |
| `state.selected` | `accent.subtle` | `accent.subtle` | Selection background |
| `state.focus` | `accent.primary` | `accent.primary` | Focus ring |

Selection semantics are constant: **accent = selected**, neutral hover =
potential target, focus outline = keyboard focus.

## 2. Typography

Platform system font stacks (never redistribute proprietary font files):

```text
macOS:   SF Pro / -apple-system
Windows: Segoe UI / system UI stack
Linux:   platform-appropriate system sans fallback
Code:    system monospace stack (ui-monospace, SF Mono, Cascadia, Consolas)
```

| Style | Size / line-height | Weight | Use |
|---|---|---|---|
| Display | 28 / 34 | 600 | First-run / empty-project hero (rare) |
| Title | 22 / 28 | 600 | Screen titles |
| Section | 17 / 22 | 600 | Section headings |
| Body | 14 / 20 | 400 | Default body text |
| Body Small | 13 / 18 | 400 | Dense secondary text |
| Caption | 12 / 16 | 400 | Captions, table footnotes |
| Micro | 11 / 14 | 400 | Status bar, badges, axis labels |
| Code | 13 / 18 | 400 | Console, scripts, command preview (monospace) |
| Numeric Large | 20 / 24 | 500 | Key result figures |

Weights: Regular 400, Medium 500, Semibold 600, Bold 700 (strong emphasis
only). No ultralight/heavy display weights.

**Tabular numerals are mandatory** (`font-variant-numeric: tabular-nums`) for
coefficients, standard errors, p-values, N, R², percentages and dates in
tables — numbers align predictably. Statistical formatting (p-value
thresholds `p < 0.001`, `R² = 0.384`, `N = 12,482`) is centralized; no screen
invents its own number formatting.

## 3. Spacing — 4-point grid

| Token | px |
|---|---|
| `space-xs` | 4 |
| `space-sm` | 8 |
| `space-md` | 12 |
| `space-lg` | 16 |
| `space-xl` | 20 |
| `space-2xl` | 24 |
| `space-3xl` | 32 |
| `space-4xl` | 40 |
| `space-5xl` | 48 |
| `space-6xl` | 64 |

Common rules: control internal padding 8–12; field gap 8; field-group gap 16;
section gap 24; major section gap 32; panel padding 16 or 20; window content
padding 20–24. No arbitrary 13/17/22/27px values. Avoid excessive whitespace —
this is research software.

## 4. Corner radius

| Token | px |
|---|---|
| `radius.none` | 0 |
| `radius.sm` | 4 |
| `radius.md` | 6 (default for controls) |
| `radius.lg` | 8 |
| `radius.xl` | 10 |
| `radius.pill` | 999 (tags, status chips, compact filters only) |

Buttons are never giant pills.

## 5. Borders and dividers

Borders are structural, not decorative. Always 1px: `border.subtle`
(quiet dividers), `border.default` (controls, tables), `border.strong`
(selection, resize). No 2px borders except deliberate focus/validation
cases. Separators establish structure heavily enough to organize, lightly
enough to stay calm — the core of the X-like visual discipline.

## 6. Shadows and elevation

**Almost no shadows.** Elevation (a small, consistent shadow) is used only
on transient surfaces: popovers, menus, context menus, modals, floating
inspectors. Main app regions are separated by background change, borders
and spacing — never by large shadows.

## 7. Component state matrix

Every interactive component documents and implements its state model:

```text
Default · Hover · Pressed · Focused · Selected · Disabled · Loading ·
Error · Warning · Success
```

(Only states applicable to the component are implemented, but the state
model must be documented per component — e.g. `NxDataGrid` rows: default,
hover, selected, focused; cells: editing, error.) Focus is always visible,
high-contrast, and never changes component geometry. Hover adds information
(subtle background/border change) — no scaling, glow or motion-heavy effects.

## 8. Keyboard shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl/Cmd + K` | Command palette |
| `Ctrl/Cmd + P` | Search project |
| `Ctrl/Cmd + S` | Save |
| `Ctrl/Cmd + Z` | Undo |
| `Ctrl/Cmd + Shift + Z` | Redo |
| `Ctrl/Cmd + Enter` | Run |
| `Ctrl/Cmd + F` | Find |
| `Ctrl/Cmd + /` | Comment (console / script editor) |
| `Esc` | Close / cancel |

macOS uses Command; Windows/Linux use Ctrl. Every workflow has a keyboard
path — the product is usable without a mouse.

## 9. Error message format

Errors always answer three questions — **What happened → Why → What to do**:

```text
Could not run model

Cluster variable contains missing identifiers.

Why this happened
4 observations have missing values in firm.

What to do
Resolve the missing cluster identifiers or choose a different
specification.

[ Inspect Data ]
```

Never a bare error code. Warnings use a less aggressive treatment than
errors (never error-red for routine warnings):

> Heteroskedasticity detected. Robust standard errors are recommended for
> this specification.

Microcopy is precise and calm: "Run regression", "Dataset imported",
"4 observations contain missing cluster identifiers" — never "Let's crunch
some numbers!" or "Something went wrong."

## 10. Component inventory

All application UI is composed from `Nx*` primitives (the `Ec*` inventory
of the design specification, renamed for Numeris). One component, one
anatomy — only state, emphasis, content and platform behavior may change.

| Component | Role |
|---|---|
| `NxButton` | Four levels only: primary, secondary, tertiary/text, destructive — one primary action per context |
| `NxIconButton` | Icon-only, tooltip required; full state model |
| `NxTextField` | Label + value + help/validation anatomy; one anatomy everywhere |
| `NxSelect` | Shared base for dropdowns, comboboxes, variable pickers; keyboard nav + type-ahead + virtualization |
| `NxCheckbox` | Independent choices |
| `NxRadioGroup` | Mutually exclusive choices |
| `NxSegmentedControl` | Closely related exclusive modes (e.g. Data / Results / Diagnostics) |
| `NxTabView` | Closely related views in the same content region |
| `NxToolbar` | Max three meaningful action groups |
| `NxSidebar` | Compact navigation with section labels (Workspace / Project / Analysis / Output / Research) |
| `NxInspector` | Contextual inspector reflecting the current selection |
| `NxDataGrid` | Virtualized data grid (sticky headers, resize, keyboard nav, copy/paste, inline edit, context menu) |
| `NxResultTable` | Statistical result tables — same geometry as data tables, tabular numerals, aligned headers, separated footnotes |
| `NxCommandPalette` | Global command palette (Ctrl/Cmd + K): searchable, fuzzy, grouped, remembers recents |
| `NxPopover` | Transient options surface |
| `NxSheet` | Long-form configuration panel |
| `NxModal` | Decisions and focused configuration only |
| `NxToast` | Sparing; never the only place important information exists |
| `NxStatus` | Consistent status labels (Ready / Running / Saved / Unsaved / Success / Warning / Error / Offline / Licensed) — text + visual cue, never color alone |
| `NxDiagnostic` | Stable semantic diagnostics (✓/⚠ + label, click to open details) |
| `NxTree` | Hierarchical lists (analysis tree, registry) |
| `NxCodeEditor` | Console/scripts: line numbers, highlighting, autocomplete, run |
| `NxChartToolbar` | One compact toolbar for every figure: Zoom / Reset / Select / Annotate / Export / More |
| `NxChartShell` | Figure container: research-figure aesthetics, restrained gridlines, publication export |

Component API discipline: semantic props only —
`<NxButton variant="primary" size="medium" />`, never
`<Button blue rounded="9" padding="13px" />`.

## 11. The no-cards rule

Cards are permitted only where they provide meaningful grouping: the license
information panel, the project health summary, a clearly grouped settings
section. Metric-per-card grids (`╭ OLS / 12,482 ╰`) are rejected. Tables and
structured lists replace dashboard cards. Results render as editorial,
document-like structures — the result is the interface, decorations are
unnecessary. Model comparison is a dense table, not cards.

## 12. Empty, loading and error states

**Empty states teach the workflow** — no decorative illustrations:

```text
No datasets yet.

Import a CSV, Excel, or statistical dataset to begin.

[ Import Data ]
```

**Loading states stay calm** — no flashy spinners. Quick operations get
subtle progress feedback; longer computation shows meaningful status:

```text
Running fixed-effects regression…
██████████████████░░░ 87%
4,217,392 observations processed
```

Where percentage is unknowable: "Estimating model… / Computing clustered
covariance…" — the user must know the software is alive.

**Error states** follow the §9 format (What / Why / What to do) with a
concrete next action button.

## 13. Motion

Functional only: micro 80–120ms, standard 140–180ms, panel/modal 180–240ms —
for panel/menu opening, selection changes, collapse/expand. No motion for
statistical output, no animated counters, no confetti. Reduced-motion
behavior eliminates nonessential animation while retaining clear state
changes.

## 14. The 15 golden screens

Every significant design change must be reviewed against this permanent
visual benchmark set:

```text
01 Project Home        06 Model Comparison     11 Settings
02 Data Editor         07 Analysis Tree        12 License
03 Variable Inspector  08 Figure View          13 Error State
04 Regression Form     09 Report View          14 Empty State
05 Regression Results  10 Command Palette     15 Dark Mode
```

Visual regression tests cover the core components (buttons, fields, menus,
tables, sidebar, toolbar, command palette, modal, diagnostics, results) in
light + dark.

## 15. Token usage rules

Tokens live in one central location and are consumed semantically:

```css
/* Allowed */
color: var(--color-text-secondary);
padding: var(--space-md);
border-radius: var(--radius-md);

/* Forbidden — raw values in product screens */
color: #2a72ff;
padding: 13px;
border-radius: 11px;
```

No module implements its own theme logic; all modules consume shared tokens
(themes: Light and Dark; a possible future High Contrast follows the same
token system). Changing a global token/component follows the design-system
release process: update spec → implementation → visual tests → review
representative screens → document → changelog. Never silently change a
global value because one screen looked better.

## 16. Anti-pattern checklist

Reject designs containing: excessive rounded cards, giant hero sections,
gradients, glass everywhere, neon colors, oversized icons/headings,
decorative illustrations, floating AI assistants, chat bubbles,
unnecessary onboarding, animated metric counters, confetti, gamification,
badge overload, inconsistent controls, different layouts for similar
workflows, random spacing/colors/radii/typography.
