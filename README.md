# KShell

Modular desktop shell workspace for Wayland and Niri, built with Rust, GTK4,
and `gtk4-layer-shell`. The current user-facing component is Klauncher; Kbar
is scaffolded for the future desktop bar.

## Workspace layout

- `apps/klauncher` — application launcher.
- `apps/kbar` — future GTK4/layer-shell bar.
- `crates/theme` — shared design tokens, templates, and rendering logic.
- `crates/niri` — reusable Niri/layer-shell identifiers.
- `tools/theme-gen` — centralized theme generator.
- `contrib`, `mockups`, and `docs` — integration files, visual references, and documentation.

## Appearance

Klauncher uses a compact `520 × 300px` Gruvbox panel with a `48px` search
header and a scrolling application list. Each result contains only its desktop
icon and Application name; long names truncate with an ellipsis. The selected
row uses a muted left indicator, keeping the query and selection easy to read
without auxiliary metadata or decorative controls. Opening it adds a `28%`
black dim over the desktop; the optional Niri integration adds a subtle
compositor blur while the panel remains opaque.

## Features

- Overlay interface centered on the screen.
- Fuzzy search by application name and generic name.
- Reads `.desktop` files from standard XDG directories.
- Supports icons by name or absolute path.
- Shell-free execution that preserves the arguments defined in `Exec`.
- Optional keyboard shortcut for Niri.

## Requirements

- Linux with a Wayland session.
- A Wayland compositor with layer-shell support.
- GTK4 with version 4.12 APIs or later.
- The `gtk4-layer-shell` development library.
- Stable Rust and Cargo.

Development package names vary between distributions. Also install `pkg-config`
and the GTK4 and `gtk4-layer-shell` development packages available for your
distribution.

## Build

From the project root:

```sh
cargo build --release -p klauncher
```

The binary will be generated at `target/release/klauncher`. To install it in a
local directory:

```sh
install -Dm755 target/release/klauncher ~/.local/bin/klauncher
```

Make sure `~/.local/bin` is in your `PATH`.

## Usage

Run the launcher inside a Wayland session:

```sh
klauncher
```

You can also run it directly through Cargo during development:

```sh
cargo run -p klauncher
```

Available controls:

- Type to filter applications.
- `Up` and `Down` navigate through the results.
- `Enter` launches the selected application.
- `Esc` closes the launcher.
- Click a result to launch it.
- Click outside the panel to close it.

The launcher searches for `.desktop` files in:

- `$XDG_DATA_HOME/applications`, or `~/.local/share/applications` when the
  variable is not set.
- Each directory listed in `$XDG_DATA_DIRS`, using its `applications`
  subdirectory.
- `/usr/local/share/applications` and `/usr/share/applications` when
  `$XDG_DATA_DIRS` is not set.

Hidden entries, entries marked as `NoDisplay`, entries incompatible with the
current desktop, and entries with an unavailable `TryExec` are not displayed.

For entries that require a terminal, the launcher uses the terminal defined by
`$TERMINAL`. If the variable is not set, it falls back to `kitty`.

## Niri Integration

Include the provided configuration file in your main Niri configuration:

```kdl
include "/path/to/kshell/contrib/niri/klauncher.kdl"
```

When using the fragment's Gruvbox layout values as an override, put this
include after any existing visual `layout` or `window-rule` configuration that
it is intended to replace. Keep it alongside, rather than inside, your own
`binds` block.

The file configures `Mod+Space` to open the launcher. If the binary is in
`~/.local/bin`, Niri must be able to find it through `PATH`. The compositor
blur rule requires Niri `26.04` or newer; the GTK dim layer and its animation
remain part of the launcher itself.

## Development

Useful commands:

```sh
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo run -p kshell-theme-gen -- --write
cargo run -p kshell-theme-gen -- --check
cargo install --path apps/klauncher
```

Unit tests are colocated with the modules in `apps/klauncher/src/`. The GTK interface requires
a graphical Wayland session for manual testing; application discovery and
parsing are covered by automated tests.

The theme renderer uses `crates/theme/src/tokens.rs` as its only color source. Before
writing terminal, visualizer, and system-information themes it checks that the
executable, user configuration, and an existing imported theme or active color
section are present. Kitty and Alacritty are supported directly, Foot is
supported when it is installed and already uses an imported color file, Cava is
updated when it has an active `[color]` section, and Fastfetch is updated when
`fastfetch/config.jsonc` is active. It writes only those theme sections/fields,
preserving each terminal's font, shell, shortcuts, and other behavior and
preserving Fastfetch's logo, modules, and layout; an existing Alacritty window
opacity is set to `1.0` to keep the theme opaque.

## License

MIT
