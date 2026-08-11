# KShell

KShell is a modular desktop shell for Wayland and Niri, built with Rust,
GTK4, and `gtk4-layer-shell`. The current workspace contains the Klauncher
application launcher, the Kbar top bar, reusable Niri and theme crates, and a
theme-generation tool.

## Current features

The repository documents the implemented baseline retrospectively. The
feature specifications distinguish behavior confirmed by the code, behavior
inferred from the current module boundaries, and decisions that are still
`TBD`.

| Feature | Current implementation | Specification |
| --- | --- | --- |
| Klauncher | XDG `.desktop` discovery, fuzzy search, keyboard/mouse selection, and shell-free launch | [001-klauncher](specs/001-klauncher/spec.md) |
| Kbar | Niri workspaces, Portuguese clock/calendar, volume, network, and optional battery status | [002-kbar](specs/002-kbar/spec.md) |
| Niri integration | JSON event stream, workspace state, reconnecting IPC, layer-shell identifiers, and generated KDL fragments | [003-niri-integration](specs/003-niri-integration/spec.md) |
| Shared theme system | Canonical tokens, generated GTK/KDL/mockup files, and opt-in configured consumer updates | [004-theme-system](specs/004-theme-system/spec.md) |

## Workspace layout

- `apps/klauncher` — launcher core and GTK4/layer-shell interface.
- `apps/kbar` — GTK4/layer-shell top bar and system-status services.
- `crates/niri` — reusable Niri protocol, state, connection, and compatibility identifiers.
- `crates/theme` — shared visual tokens, templates, and rendering helpers.
- `tools/theme-gen` — command-line theme generator.
- `contrib/niri` — generated optional Niri configuration fragments.
- `mockups` — browser mockups and generated visual reference assets.
- `specs` — feature specifications, with optional plans and task lists for
  active changes that need them.
- `docs/architecture` — global architecture and design-system documentation.
- `docs/decisions` — accepted architectural decision records.

See the [architecture overview](docs/architecture/overview.md) for current
boundaries and runtime flows, the [constitution](.specify/memory/constitution.md)
for global feature rules, and the [test structure](tests/README.md) for the
existing validation layout.

## Requirements

- Linux with a Wayland session.
- A Wayland compositor with layer-shell support; Niri is the supported integration.
- GTK4 with version 4.12 APIs or later.
- The `gtk4-layer-shell` development library.
- Stable Rust and Cargo.
- `pkg-config` and the GTK4/layer-shell development packages for the distribution.

The launcher and bar cannot be meaningfully exercised without a graphical
Wayland session, but their deterministic parsing, ranking, state, and service
boundary logic is covered by the workspace tests.

## Build and run

From the project root:

```sh
cargo build --release -p klauncher
cargo build --release -p kbar
```

Run either application directly inside a suitable session:

```sh
cargo run -p klauncher
cargo run -p kbar
```

To install binaries for Niri autostart and keybindings:

```sh
cargo install --path apps/klauncher
cargo install --path apps/kbar
```

Niri resolves these commands through `PATH`.

## Klauncher behavior

Klauncher presents a centered, keyboard-first overlay. It reads applications
from `$XDG_DATA_HOME/applications` (or the standard home fallback), then from
the application directories in `$XDG_DATA_DIRS`, with the standard system
directories used when that variable is absent. Discovery is recursive and
deduplicates desktop-file IDs.

Entries that are not applications, are hidden, are marked `NoDisplay`, are
not visible for the current desktop, or have an unavailable `TryExec` are
excluded. Names use the preferred locale when a localized value is available.
Search matches the application name and generic name with fuzzy ranking.

The selected row is launched with `Enter` or a click. `Up` and `Down` navigate
with wrapping, `Esc` closes the launcher, and clicking outside the panel closes
it. The panel uses the current Gruvbox design tokens; see the
[design-system architecture document](docs/architecture/design-system.md).

Desktop `Exec` entries are parsed into an executable and argument vector. They
are never passed to a shell. Terminal entries use `$TERMINAL` and fall back to
`kitty`; non-terminal applications run with inherited argument boundaries and
without a terminal attached.

## Kbar and Niri integration

Include the generated fragments in the main Niri configuration:

```kdl
include "/path/to/kshell/contrib/niri/kbar.kdl"
include "/path/to/kshell/contrib/niri/klauncher.kdl"
```

`kbar.kdl` starts Kbar with `spawn-at-startup`. `klauncher.kdl` keeps the
existing `Mod+Space` binding, launcher blur rule, and quiet visual defaults.
The current application IDs and layer-shell namespaces are compatibility
identifiers; a rename requires a coordinated update of generated templates
and compositor rules.

Kbar reserves the top exclusive zone, displays five visual workspace slots,
and receives workspace state from `$NIRI_SOCKET`. Its clock is updated on the
minute and opens a Portuguese calendar popover. Volume is read and controlled
through `wpctl`; the popover supports a slider, mute, and output selection.
Network status uses `nmcli` with a default-route fallback, and battery status
comes from `/sys/class/power_supply` when a battery is present. All service
commands use bounded, shell-free subprocesses.

Set `KSHELL_OUTPUT` to a Wayland connector name such as `DP-1` to target an
explicit output. Without it, the compositor selects the surface output; full
multi-bar orchestration is not currently defined.

## Theme generation

`crates/theme/src/tokens.rs` is the canonical source for shared colors,
geometry, typography, and Niri identifiers. The generator renders the
checked-in GTK, KDL, and mockup artifacts:

```sh
cargo run -p kshell-theme-gen -- --write
cargo run -p kshell-theme-gen -- --check
```

When an installed consumer has an existing compatible configuration, `--write`
also updates its theme values while preserving unrelated settings. See the
[theme specification](specs/004-theme-system/spec.md) for the current scope.

## Validation

The required local checks are the same gates used by CI:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo run -p kshell-theme-gen -- --check
```

The [constitution](.specify/memory/constitution.md) and [agent guidelines](AGENTS.md)
define the SDD and validation rules for future changes.

## License

MIT
