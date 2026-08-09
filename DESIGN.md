---
name: Klauncher Gruvbox
description: A compact Gruvbox application launcher with a focused query and one quiet selection.
colors:
  background: "#1d2021"
  surface: "#282828"
  hover: "#32302f"
  selection: "#3c3836"
  border: "#665c54"
  foreground: "#ebdbb2"
  secondary-foreground: "#a89984"
typography:
  mono:
    fontFamily: '"JetBrainsMono Nerd Font Mono", monospace'
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.4
rounded:
  panel: "2px"
  row: "0px"
spacing:
  panel-edge: "14px"
  list-edge: "7px"
  row-icon-gap: "10px"
components:
  launcher-panel:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.foreground}"
    width: "520px"
    height: "300px"
  launcher-input:
    backgroundColor: "{colors.background}"
    textColor: "{colors.foreground}"
    height: "48px"
  selection-row:
    backgroundColor: "{colors.selection}"
    textColor: "{colors.foreground}"
    height: "38px"
---

# Design System: Klauncher Gruvbox

## Overview

Klauncher is a small, keyboard-first surface for choosing one installed Application without leaving the current Wayland workspace. Its reference is the native launcher and the matching mockup in `mockups/launcher-designs.html`: a compact Gruvbox panel with a single query line and a calm application list.

The interface has one deliberate signature: the selected Application is marked by a narrow, muted left rail. Everything else stays quiet and legible so the query and selection remain the only hierarchy a person has to read.

## Tokens

| Token | Value | Use |
| --- | --- | --- |
| Background | `#1d2021` | desktop-facing field and query header |
| Surface | `#282828` | launcher panel and idle list |
| Hover | `#32302f` | pointer or focus hover state |
| Selection | `#3c3836` | selected Application row |
| Border | `#665c54` | 2px panel border, header divider, selection rail |
| Foreground | `#ebdbb2` | query and Application names |
| Secondary | `#a89984` | prompt, placeholder, and empty state |

Real desktop icons retain their source colors. They are recognizers for Applications, not interface accents.

## Typography and hierarchy

Use `"JetBrainsMono Nerd Font Mono", monospace` throughout the launcher. Text is regular weight at `13px`; the query prompt and placeholder use the secondary token. The Application name has no competing display type, label system, auxiliary metadata, or decorative control.

Each Application row renders only:

```text
[18px icon]  Application name
```

Names truncate with an ellipsis rather than changing the panel rhythm.

## Geometry

The panel is exactly `520 × 300px`, including its `2px` outer border. It remains this size whenever the monitor has room; the native implementation may constrain it only to keep the required `16px` screen margin on smaller displays.

- Query header: `48px` high, `14px` horizontal inset, leading `>` prompt.
- List: fills all remaining panel height, has `7px` inset, and scrolls when needed.
- Application row: `38px` high, `18px` icon, `10px` icon-to-name gap.
- Selection: a `2px` left rail in Border, with the Selection fill.

The mockup uses the same dimensions and a fixed panel rather than growing or shrinking with its result count.

## States and depth

Idle rows are transparent over Surface. Hover uses Hover. Keyboard or pointer selection uses Selection plus the left rail; it must remain visible independently of hover. The query uses the native caret without an extra glow.

The panel is flat: a `2px` Border outline and the contrast against Background establish its edge. Corners may be no rounder than `2px`. Do not use shadows, blur, gradients, glass effects, HUD treatments, bright focus colors, or decoration that does not aid search or selection.

## Native interaction contract

The Launcher Query filters by Application name and generic name using fuzzy ranking. The visible row remains icon plus name only. Up and Down move through filtered results, Enter launches the selection, Esc closes the panel, a result click launches it, and clicking outside the panel closes it.

## Do and don't

Do preserve the fixed compact geometry, the muted Gruvbox tokens, the focused query, and ellipsis for long names.

Do not introduce vibrant accents, gradients, glows, rounded cards, or decorative HUD styling.
