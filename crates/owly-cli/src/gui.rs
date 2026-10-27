//! `owly gui`: hands over to the owly-gui window, installed beside the CLI by
//! the owly-gui package (or built next to it in a checkout).

use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

/// The GUI's binary: next to this one when it is there, else on `PATH`.
fn program() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("owly-gui")))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from("owly-gui"))
}

/// Replaces this process with the window, so its exit status is the GUI's.
/// Only returns, with the reason, when it can't start.
pub fn run() -> Result<ExitCode, String> {
    let program = program();
    let error = Command::new(&program).exec();
    Err(match error.kind() {
        std::io::ErrorKind::NotFound => format!(
            "{} not found: install the owly-gui package, or run `just gui-build`",
            program.display()
        ),
        _ => format!("could not start {}: {error}", program.display()),
    })
}
