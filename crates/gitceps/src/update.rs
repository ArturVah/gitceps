//! `gitceps update`: installs the latest version over this one, the way this one was installed.
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::paths;

const INSTALLER: &str = "https://raw.githubusercontent.com/ArturVah/gitceps/main/install";

/// Runs the installer again when it put this program in the Gitceps folder. Homebrew keeps track of what it
/// installed, so an update there is its to make.
pub fn update() -> Result<()> {
    let this = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .context("can't find where gitceps is")?;
    let installed = paths::folder()?
        .canonicalize()
        .is_ok_and(|folder| this.starts_with(folder));
    if !installed {
        bail!(
            "This Gitceps in {} didn't come from the installer, so update it the way you installed it \
             (Homebrew: brew upgrade gitceps).",
            this.parent().unwrap_or(&this).display()
        );
    }
    let done = if cfg!(windows) {
        let script = format!("irm {INSTALLER}.ps1 | iex");
        Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .status()
    } else {
        let script = format!("set -o pipefail; curl -fsSL {INSTALLER}.sh | bash");
        Command::new("bash").args(["-c", &script]).status()
    };
    if !done.context("can't start the installer")?.success() {
        bail!("the update didn't finish");
    }
    Ok(())
}
