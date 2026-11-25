use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use gitceps::switches::{self, Switch};
use gitceps::tour::TourCheck;
use gitceps::{hooks, window};

/// A gym bro who pushes your code to the cloud every time you git push
#[derive(Parser)]
#[command(version)]
enum Command {
    /// Turns Gitceps on for every git repo on this computer
    On,
    /// Turns Gitceps off
    Off,
    /// Prints "on" or "off"
    Status,
    /// Lists each switch, and whether it's on where you are
    List,
    /// Turns a reaction, or the secret check, back on
    Enable {
        switch: Switch,
        /// Only in the repo you're in
        #[arg(long)]
        here: bool,
    },
    /// Turns a reaction, or the secret check, off
    Disable {
        switch: Switch,
        /// Only in the repo you're in
        #[arg(long)]
        here: bool,
    },
    /// Shows a tour of a PR or commit that a coding agent wrote
    Tour { file: PathBuf },
    /// What git runs for each of its hooks while Gitceps is on
    #[command(hide = true)]
    Hook {
        name: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        arguments: Vec<OsString>,
    },
}

fn main() -> ExitCode {
    let done = match Command::parse() {
        Command::On => switches::on(),
        Command::Off => switches::off(),
        Command::Status => switches::status(),
        Command::List => {
            switches::list();
            Ok(())
        }
        Command::Enable { switch, here } => switches::set(switch, true, here),
        Command::Disable { switch, here } => switches::set(switch, false, here),
        Command::Tour { file } => tour(&file),
        Command::Hook { name, arguments } => return ExitCode::from(hooks::run(&name, &arguments)),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

/// Checks a tour and opens it, or fails with everything the tour is missing.
fn tour(file: &Path) -> Result<()> {
    let check = TourCheck::new(file)?;
    let repo = check.repo_name();
    let page = check.page()?;
    let json = serde_json::to_string(&page)?;
    if window::dry() {
        println!("{json}");
        return Ok(());
    }
    let plural = |count: usize| if count == 1 { "" } else { "s" };
    let (steps, files) = (page.steps.len(), page.overview.len());
    println!(
        "Tour checked: {steps} step{}, {} of {} changed lines shown, {} skipped.",
        plural(steps),
        page.shown_lines,
        page.shown_lines + page.skipped_lines,
        page.skipped_lines
    );

    let number = page.number.map(|number| format!("#{number} · ")).unwrap_or_default();
    let ins: usize = page.overview.iter().map(|file| file.ins).sum();
    let del: usize = page.overview.iter().map(|file| file.del).sum();
    window::show(
        &format!("{number}{}", page.title),
        &format!("{repo} · {files} file{} · +{ins} −{del}", plural(files)),
        &json,
    )?;
    println!("Tour opened.");
    Ok(())
}
