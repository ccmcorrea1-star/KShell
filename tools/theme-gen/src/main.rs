use std::env;
use std::path::Path;
use std::process::ExitCode;

use kshell_theme::tokens;

fn main() -> ExitCode {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .to_path_buf();

    match env::args().nth(1).as_deref() {
        Some("--write") => match tokens::write_generated_files(&workspace_root) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("could not write generated theme files: {error}");
                ExitCode::FAILURE
            }
        },
        Some("--check") => match tokens::generated_files_are_current(&workspace_root) {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => {
                eprintln!("generated theme files are stale; run: cargo run -p kshell-theme-gen -- --write");
                ExitCode::FAILURE
            }
            Err(error) => {
                eprintln!("could not check generated theme files: {error}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("usage: cargo run -p kshell-theme-gen -- <--write|--check>");
            ExitCode::FAILURE
        }
    }
}
