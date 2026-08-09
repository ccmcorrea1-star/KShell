# Klauncher

Lightweight application launcher for Wayland and Niri, built with Rust, GTK4,
and `gtk4-layer-shell`.

## Appearance

Klauncher uses a compact `520 × 300px` Gruvbox panel with a `48px` search
header and a scrolling application list. Each result contains only its desktop
icon and Application name; long names truncate with an ellipsis. The selected
row uses a muted left indicator, keeping the query and selection easy to read
without auxiliary metadata or decorative controls.

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
cargo build --release
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
cargo run
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
include "/path/to/klauncher/contrib/niri/klauncher.kdl"
```

When using the fragment's Gruvbox layout values as an override, put this
include after any existing visual `layout` or `window-rule` configuration that
it is intended to replace. Keep it alongside, rather than inside, your own
`binds` block.

The file configures `Mod+Space` to open the launcher. If the binary is in
`~/.local/bin`, Niri must be able to find it through `PATH`.

## Development

Useful commands:

```sh
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

Unit tests are colocated with the modules in `src/`. The GTK interface requires
a graphical Wayland session for manual testing; application discovery and
parsing are covered by automated tests.

## License

MIT
