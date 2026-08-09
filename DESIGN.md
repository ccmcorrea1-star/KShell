---
name: Klauncher Void
description: A black, keyboard-first application launcher surface with one clear selection.
colors:
  black: "#050505"
  soft-black: "#111312"
  foreground: "#F0F0EA"
  secondary-foreground: "#949993"
  muted-foreground: "#777A75"
  paper: "#0C0F0E"
  stage: "#151918"
  focus: "#5660FF"
  selection: "rgba(255, 255, 255, 0.10)"
typography:
  display:
    fontFamily: '"JetBrainsMono Nerd Font Mono", monospace'
    fontSize: "clamp(64px, 10vw, 96px)"
    fontWeight: 800
    lineHeight: 0.84
    letterSpacing: "-0.04em"
  body:
    fontFamily: '"JetBrainsMono Nerd Font Mono", monospace'
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.4
  label:
    fontFamily: '"JetBrainsMono Nerd Font Mono", monospace'
    fontSize: "10px"
    fontWeight: 400
    lineHeight: 1.4
    letterSpacing: "0.08em"
    fontVariation: "normal"
rounded:
  none: "0px"
  hairline: "1px"
spacing:
  xs: "8px"
  sm: "12px"
  md: "20px"
  lg: "40px"
components:
  launcher-panel:
    backgroundColor: "{colors.black}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.hairline}"
    width: "600px"
    height: "380px"
  launcher-input:
    backgroundColor: "{colors.black}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.none}"
    padding: "0 20px"
    height: "64px"
  selection-row:
    backgroundColor: "{colors.selection}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.none}"
    padding: "7px 14px"
    height: "52px"
---

# Design System: Klauncher Void

## Overview

**Creative North Star: "The Black Terminal Window"**

Void is a single black surface held inside a dark outer stage. The page background stays near-black and the stage is only slightly lighter, with a low-contrast grid framing the fixed panel. The launcher does not decorate the task; it creates a temporary field of attention over the desktop and removes itself when the decision is made. Its visual language comes from terminal output, system overlays, and the quiet precision of a monochrome monitor.

The hierarchy is deliberately narrow: the query is the entry point, the application selection is the only high-contrast state, and system metadata stays subordinate. The interface should feel present but not needy. The selected Void direction is the binding visual reference for this mockup surface; future variants should not add color or competing layers without an explicit request.

**Key Characteristics:**

- Flat near-black panel with a clear light border.
- Monospace typography for query, data, status, and application rows.
- Restrained tonal selection row as the single strong state change.
- Keyboard-first hints kept visible at the bottom edge.
- Exact `600 x 380px` launcher panel at the current source size.

## Colors

The palette is near-black and mineral gray, using contrast rather than hue to communicate state. The outer background and stage stay dark so the Void launcher remains the visual focus.

### Primary

- **Void Black** (`#050505`): launcher panel surface and primary dark field.
- **Focus Blue** (`#5660FF`): browser-level keyboard focus ring only; it does not enter the launcher panel palette.

### Neutral

- **Terminal White** (`#F0F0EA`): border, primary text, active metadata, and selected-row text or marker.
- **Soft Black** (`#111312`): secondary dark surface when a supporting layer needs separation.
- **Signal Gray** (`#949993`): application text in the idle state and secondary panel detail.
- **Quiet Gray** (`#777A75`): subtitles, key hints, and low-priority status text.
- **Outer Background** (`#0C0F0E`): dark page surface surrounding the mockup stage.
- **Dark Stage** (`#151918`): slightly lighter outer stage behind the real-size panel and its low-contrast grid.
- **Selection Tone** (`rgba(255, 255, 255, 0.10)`): restrained tonal lift for the selected application row.

**The One Contrast Rule.** The selected application row is the only list state allowed to receive a deliberate tonal lift; keep that lift inside the near-black Void palette.

## Typography

**Display Font:** "JetBrainsMono Nerd Font Mono", monospace
**Body Font:** "JetBrainsMono Nerd Font Mono", monospace
**Label/Mono Font:** "JetBrainsMono Nerd Font Mono", monospace

**Character:** The native GTK launcher uses "JetBrainsMono Nerd Font Mono", monospace as its global stack, keeping query, status, metadata, and application data inside the same technical alphabet. The static mockup may retain its existing Inter fallback for surrounding framing copy, but launcher-facing text follows the native stack.

### Hierarchy

- **Display** (800, `clamp(64px, 10vw, 96px)`, `0.84`): the Void direction title on the review surface.
- **Title** (400, `15px`, `1.4`): application names inside the launcher panel.
- **Body** (400, `14px`, `1.4`): explanatory copy around the mockup.
- **Label** (400, `10px`, `0.08em`, uppercase): system metadata, key hints, and status values.

**The Data Voice Rule.** Do not use a decorative display face inside the launcher. A value that describes system state should look like data.

## Layout

The review surface centers one fixed-size panel inside a measured stage. The launcher itself is exactly `600 x 380px`, matching `PANEL_WIDTH` and `PANEL_HEIGHT` in `src/ui/gtk.rs`. On narrow viewports, the stage scrolls horizontally instead of shrinking or distorting the panel.

Inside the reference mockup, the structure is a compact vertical stack: a `64px` query row, a `266px` application list, and a `46px` keyboard/status footer. Application rows use a consistent `52px` rhythm and preserve enough width for long names to truncate cleanly. The outer `2px` panel border accounts for the remaining height of the `380px` panel.

## Elevation & Depth

The launcher is flat by default. Depth comes from the border, the dark surrounding stage, its low-contrast grid, and the tonal change of the selected row rather than from a floating or inset shadow. The panel should not use blur, glass treatment, hard offset shadows, or decorative glow.

**The Flat Surface Rule.** If the panel needs more emphasis, increase contrast or clarify the border before adding elevation.

## Shapes

The panel is rectangular with a `1px` to `2px` light border and no decorative rounding. The reference mockup uses small square letter marks with a `1px` border and a `1px` radius as icon stand-ins. The native launcher preserves each desktop entry's real icon, whether it is supplied by theme name or absolute path. Buttons and rows use square silhouettes. Pills, soft cards, and rounded glass surfaces do not belong to this world.

## Components

### Buttons

- **Shape:** square, `0px` radius for launcher controls.
- **Primary:** application rows are transparent at rest and receive `{colors.selection}` when selected.
- **Hover / Focus:** hover raises the row contrast slightly; keyboard focus remains visible with the browser or GTK focus treatment.

### Cards / Containers

- **Corner Style:** panel and stage are rectangular; no card radius.
- **Background:** panel uses `{colors.black}`; the page stage uses `{colors.stage}`.
- **Shadow Strategy:** no panel shadow; rely on border and tonal separation.
- **Border:** `2px` light outer border on the launcher panel, `1px` dividers between data groups.
- **Internal Padding:** panel edges use `20px`; list rows use `7px 14px`.

### Inputs / Fields

- **Style:** transparent black input with a `1px` lower divider and a leading `>` prompt.
- **Focus:** native caret with no visible search outline; the panel border and lower divider provide the field boundary. The placeholder is quiet gray.
- **Error / Empty:** the application list announces `no applications found` when the launcher query has no match.

### Signature Component

The Void application row is the signature component in the reference mockup: a square mark, a two-line application label, a right-aligned keyboard hint, and one full-row tonal selected state. In the native launcher, the square mark is replaced by the desktop entry's real icon and clicking a result launches that application.

## Native Interaction Contract

The native launcher keeps the mockup's keyboard-first interaction while using live desktop entries. `Up` and `Down` wrap through filtered results, `Enter` launches the selected result, `Esc` closes the surface, and clicking a result launches the clicked row rather than only changing selection. A click outside the panel closes it.

## Do's and Don'ts

### Do:

- **Do** preserve the exact `600 x 380px` panel proportion for this surface.
- **Do** use monochrome contrast to communicate selection and hierarchy.
- **Do** keep the query focused on open and expose Up, Down, Enter, and Esc behavior.
- **Do** keep long application names truncated rather than allowing the panel rhythm to break.

### Don't:

- **Don't** add accent colors to application rows or categories.
- **Don't** replace the restrained selection tone with a glow, gradient, or colored pill.
- **Don't** hide the close affordance or make keyboard state depend on hover.
- **Don't** introduce rounded cards, glass blur, or hard offset shadows into the Void panel.
