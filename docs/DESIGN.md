# Design System: Klauncher Gruvbox

## Overview

Klauncher is a compact, keyboard-first surface for choosing an installed application without leaving the current Wayland workspace. Its visual reference is the native launcher and its companion mockup in `mockups/launcher-designs.html`.

The selected application is marked by a narrow, muted left rail. Everything else stays quiet and legible so the query and selection remain the only hierarchy a person has to read.

Kbar extends the same system into a low `32px` top surface. Its approved
reference is `mockups/bar-design.html`: five workspace controls on the left,
the Portuguese date/time centered by the viewport rather than by neighboring
content, and icon-led system status on the right. The surface uses the same
canvas, structural border, mono type, `2px` radius, and spacing vocabulary as
Klauncher. It has no independent palette, elevation, blur, shadow, pill, or HUD
treatment.

The volume status module is an intentional extension of that quiet system
surface: its icon and percentage remain visible in the bar, while the compact
popover exposes only the PipeWire-backed control and a direct output list with
the active device marked. It uses no separate audio state, HUD, glow, or
decorative control chrome.

## Canonical tokens and consumers

`crates/theme/src/tokens.rs` is the only canonical source for colors, semantic and ANSI palette entries, typography, spacing, radii, borders, and approved launcher geometry. It defines the neutral Gruvbox surfaces used by the launcher and the ANSI palette consumed by configured terminals.

The generator renders these templates:

- `crates/theme/templates/style.css` → `apps/klauncher/src/ui/style.css` for GTK
- `crates/theme/templates/kbar.css` → `apps/kbar/src/ui/style.css` for GTK
- `crates/theme/templates/kbar.kdl` → `contrib/niri/kbar.kdl` for Niri autostart
- `crates/theme/templates/klauncher.kdl` → `contrib/niri/klauncher.kdl` for Niri
- `crates/theme/templates/theme.css` → `mockups/theme.css` for the browser mockup
- `crates/theme/templates/kitty.conf` → the active imported Kitty theme file
- `crates/theme/templates/alacritty.toml` → the active imported Alacritty color file
- `crates/theme/templates/foot.ini` → an active imported Foot color file, when Foot is installed and configured
- `crates/theme/templates/cava.ini` → the active `[color]` section of Cava's configuration
- `crates/theme/templates/fastfetch.jsonc` → color fields in the active Fastfetch configuration

The mockup loads `theme.css`; do not add a second source of visual values to it. The generated files carry a header and must not be edited directly.

Terminal, visualizer, and system-information consumers are detected before writing: the renderer requires the executable, user configuration, and an existing imported theme or active color section. It does not create or modify a consumer that is only present as an executable or an orphaned configuration. The main terminal configuration remains responsible for fonts, shell, shortcuts, and other behavior. Fastfetch preserves its logo source, modules, and layout while replacing only color values. The only non-palette adjustment is forcing an existing Alacritty window opacity to `1.0` so the global no-transparency rule is effective.

After changing tokens or a template, run:

```sh
cargo run -p kshell-theme-gen -- --write
cargo run -p kshell-theme-gen -- --check
```

The colocated theme test also verifies that every template resolves and that the checked-in outputs match the renderer.

## Typography and geometry

Use the canonical mono family at regular UI size and line height. The query, prompt, placeholder, row names, empty state, mockup, and native GTK implementation share these tokens. Terminal font settings remain owned by each terminal configuration.

The launcher retains its approved fixed panel geometry whenever the display has room; on smaller displays the native implementation only constrains it to preserve the canonical screen margin. The header, list inset, row rhythm, icon size, and icon-to-name gap are all named geometry tokens. Application names truncate with an ellipsis rather than changing that rhythm.

Each application row renders only:

```text
[icon]  Application name
```

Real desktop icons retain their source colors. They are recognizers for applications, not interface accents.

## States and depth

Idle rows are transparent over the surface. Hover uses the elevated neutral surface. Selection uses the selected neutral surface plus the structural left rail and remains visible independently of hover. Focus uses the structural border without glow. Disabled content uses the disabled text token over a neutral surface.

The panel is flat: its structural outline and contrast against the canvas establish its edge. Use no shadows, gradients, glass effects, HUD treatments, bright focus colors, or decoration that does not aid search or selection. When the launcher opens, only the desktop-facing field receives a restrained black dim and compositor blur; the launcher panel stays solid and sharp.

Terminal backgrounds, foregrounds, cursors, and selections use the global surface tokens. The complete ANSI Gruvbox palette is reserved for command output, syntax highlighting, links, and semantic terminal states; terminal chrome remains neutral.

Cava uses the same global background and foreground with a restrained neutral Gruvbox ramp for bars. Its audio visualization does not introduce ANSI or decorative accent colors.

## Native interaction contract

The launcher query filters by application name and generic name using fuzzy ranking. The visible row remains icon plus name only. Up and Down move through filtered results, Enter launches the selection, Esc closes the panel, a result click launches it, and clicking outside the panel closes it. The Niri fragment preserves the existing `Mod+Space` binding, compositor rules, and quiet workspace defaults.
