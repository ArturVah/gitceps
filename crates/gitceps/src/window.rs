//! Starts Gitceps' windows. They are a program of their own, gitceps-window, beside this one, so a git hook
//! never loads a window library and never waits for an animation.
use std::fs::OpenOptions;
use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use crate::paths;

/// With GITCEPS_DRY set nothing is shown: what a window would get is printed instead.
pub fn dry() -> bool {
    std::env::var_os("GITCEPS_DRY").is_some()
}

/// Plays a reaction in the corner of the screen, without waiting for it.
pub fn play(mode: &str, details: &[(&str, &str)]) {
    if dry() {
        let details: Vec<String> = details.iter().map(|(flag, value)| format!("{flag} {value}")).collect();
        eprintln!("gitceps would play: {mode} ({})", details.join(" "));
        return;
    }
    let mut command = background();
    command.args(["play", mode]).stdin(Stdio::null());
    for (flag, value) in details {
        command.args([flag, value]);
    }
    let _ = command.spawn(); // a push goes on without its animation
}

/// Opens a checked tour, which the window reads on its standard input.
pub fn show(title: &str, subtitle: &str, page: &str) -> Result<()> {
    let mut window = background()
        .args(["show", "--title", title, "--subtitle", subtitle])
        .stdin(Stdio::piped())
        .spawn()
        .context("can't open the tour")?;
    let mut feed = window.stdin.take().expect("its input is piped");
    feed.write_all(page.as_bytes()).context("can't open the tour")
}

/// The window program, set up to outlive the command that starts it and to write to the log rather than to
/// that command's output.
fn background() -> Command {
    let name = format!("gitceps-window{}", std::env::consts::EXE_SUFFIX);
    // Beside the file itself: a link to gitceps, as Homebrew makes, may have no gitceps-window beside it.
    let this = std::env::current_exe().and_then(std::fs::canonicalize);
    let program = this.map_or_else(|_| name.clone().into(), |this| this.with_file_name(&name));
    let mut command = Command::new(program);

    let log = paths::log().ok().and_then(|path| {
        std::fs::create_dir_all(path.parent()?).ok()?;
        let log = OpenOptions::new().create(true).append(true).open(path).ok()?;
        Some((log.try_clone().ok()?, log))
    });
    match log {
        Some((output, errors)) => command.stdout(output).stderr(errors),
        None => command.stdout(Stdio::null()).stderr(Stdio::null()), // a sandbox may not allow the log
    };

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x8;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x200;
        command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    command
}
