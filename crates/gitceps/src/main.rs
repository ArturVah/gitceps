use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use gitceps::tour::TourCheck;

#[derive(Parser)]
#[command(version, about)]
enum Command {
    /// Shows a tour of a PR or commit that a coding agent wrote
    Tour { file: PathBuf },
}

fn main() -> ExitCode {
    match Command::parse() {
        Command::Tour { file } => tour(file),
    }
}

/// Prints what the tour is missing and fails, or says it's fine. GITCEPS_DRY=1 prints what the window would get.
fn tour(file: PathBuf) -> ExitCode {
    let page = match TourCheck::new(&file).and_then(TourCheck::page) {
        Ok(page) => page,
        Err(error) => {
            eprintln!("{error:#}");
            return ExitCode::FAILURE;
        }
    };
    if std::env::var_os("GITCEPS_DRY").is_some() {
        println!("{}", serde_json::to_string(&page).expect("a page is plain data"));
        return ExitCode::SUCCESS;
    }
    let steps = page.steps.len();
    let total = page.shown_lines + page.skipped_lines;
    println!(
        "Tour checked: {steps} step{}, {} of {total} changed lines shown, {} skipped.",
        if steps == 1 { "" } else { "s" },
        page.shown_lines,
        page.skipped_lines
    );
    ExitCode::SUCCESS
}
