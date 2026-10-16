//! Spawning external programs (git) from a GUI app.

use std::ffi::OsStr;
use std::process::Command;

/// A [`Command`] for `program`. Every process the app spawns goes through this
/// (clippy's `disallowed-methods` forbids a bare `Command::new`).
#[allow(clippy::disallowed_methods)]
pub fn command(program: impl AsRef<OsStr>) -> Command {
    Command::new(program)
}

/// Apps launched from Finder get a minimal PATH; borrow the login shell's so
/// `git` resolves like in the terminal. Call first thing in `main`.
pub fn inherit_shell_path() {
    #[cfg(target_os = "macos")]
    {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
        if let Ok(out) = command(shell)
            .args(["-l", "-c", "printf %s \"$PATH\""])
            .output()
            && out.status.success()
            && let Ok(path) = String::from_utf8(out.stdout)
            && !path.is_empty()
        {
            // SAFETY: called first thing in main, before any thread is spawned.
            unsafe { std::env::set_var("PATH", path) };
        }
    }
}
