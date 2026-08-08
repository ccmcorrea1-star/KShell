# Repository Guidelines

## Project Structure & Module Organization

Klauncher is a Rust 2021 binary crate for a minimal Wayland/Niri application launcher.

- `src/main.rs` coordinates loading, the GTK session, and launching.
- `src/core/desktop.rs` discovers and parses XDG `.desktop` application files.
- `src/core/search.rs` implements fuzzy matching and ranking.
- `src/core/launch.rs` starts selected applications in their own Unix session.
- `src/ui/gtk.rs` owns the GTK4/gtk4-layer-shell launcher interface.
- `contrib/niri/klauncher.kdl` contains the optional Niri keybinding/window configuration.
- Tests are colocated in `#[cfg(test)]` modules; there is currently no separate `tests/` directory.

## Build, Test, and Development Commands

Run commands from the repository root:

```sh
cargo fmt --check                 # verify rustfmt formatting
cargo check                       # type-check without producing a binary
cargo test                        # run all unit tests
cargo clippy --all-targets -- -D warnings  # lint and reject warnings
cargo build                       # build the debug binary
cargo run                         # build and run the launcher
```

The application reads user/system application entries through `XDG_DATA_HOME`, `XDG_DATA_DIRS`, and `HOME`, so Linux/XDG behavior should be used for manual testing. To try Niri integration, include `contrib/niri/klauncher.kdl` from the Niri configuration as described in that file.

## Coding Style & Naming Conventions

Use stable Rust 2021 and four-space indentation; let `rustfmt` determine layout. Keep functions focused and prefer explicit `Result`/`Option` handling over panics in runtime code. Use `snake_case` for functions, variables, and modules; `UpperCamelCase` for types; and `SCREAMING_SNAKE_CASE` for constants. Avoid shell invocation when handling desktop `Exec` entries—preserve argument boundaries and validate parsing behavior.

## Testing Guidelines

Add focused unit tests beside the implementation they cover. Name tests by behavior, such as `parses_application_and_expands_exec_fields`. Cover parser edge cases, fuzzy ranking, UI rendering, and lifecycle-sensitive behavior where practical. Run `cargo test` and `cargo fmt --check` before submitting changes.

## Commit & Pull Request Guidelines

Use short, imperative Conventional Commit-style subjects, for example `feat: improve launcher UI` or `fix: handle missing desktop directories`. Pull requests should explain the user-visible or architectural change, link related issues when applicable, list validation commands, and include terminal screenshots or recordings for UI changes. Keep configuration changes and source changes clearly described.

## Security & Configuration Tips

Treat `.desktop` file contents and environment paths as untrusted input. Preserve the existing no-shell launch model, avoid logging command arguments or user paths unnecessarily, and test changes against malformed entries. Do not commit personal Niri configuration, machine-specific paths, or generated `target/` artifacts.
