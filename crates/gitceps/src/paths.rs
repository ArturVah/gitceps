//! Where Gitceps keeps its files: a folder in the user's home, and the log where the system keeps logs.
use std::path::PathBuf;

use anyhow::{Context, Result};

pub fn home() -> Result<PathBuf> {
    std::env::home_dir().context("can't find your home folder")
}

/// Gitceps' own folder, where the installer puts the programs too.
pub fn folder() -> Result<PathBuf> {
    Ok(home()?.join(".gitceps"))
}

/// The small scripts git runs while Gitceps is on, one per hook name.
pub fn hooks() -> Result<PathBuf> {
    Ok(folder()?.join("hooks"))
}

/// What the windows print, since nobody is there to read it when they run. A Mac keeps logs in one place.
pub fn log() -> Result<PathBuf> {
    let home = home()?;
    Ok(if cfg!(target_os = "macos") {
        home.join("Library").join("Logs").join("gitceps.log")
    } else {
        folder()?.join("gitceps.log")
    })
}
