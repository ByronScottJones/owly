//! Opening a link in the user's browser (OWLY-7). Done by the backend with
//! the desktop's own opener; the page has no shell or opener permission.

use std::io;
use std::process::{Command, Stdio};

/// Starts `xdg-open` on `url`, which the caller has checked is `http` or
/// `https` (so it can't be read as an option), and leaves it to run.
pub fn open(url: &str) -> io::Result<()> {
    let mut child = Command::new("xdg-open")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    // Reaped on a thread of its own, so a slow opener never holds up the window.
    std::thread::spawn(move || match child.wait() {
        Ok(status) if !status.success() => tracing::warn!("xdg-open ended with {status}"),
        Ok(_) => {}
        Err(e) => tracing::warn!("waiting for xdg-open: {e}"),
    });
    Ok(())
}
