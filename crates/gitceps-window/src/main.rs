//! Gitceps' windows. `gitceps` starts this program and never waits for it:
//!
//!   gitceps-window play MODE [--ins N] [--del N] [--msg TEXT] [--secret FILE] [--remote REPO] …
//!     Plays one reaction in the corner of the screen (overlay.rs).
//!
//!   gitceps-window show --title TEXT --subtitle TEXT
//!     Shows a checked tour, read from standard input, in a floating panel (tour.rs).
#![cfg_attr(windows, windows_subsystem = "windows")]

mod overlay;
mod pages;
mod platform;
mod tour;
mod webview;

use std::io::Read;
use std::process::ExitCode;

use serde_json::{Map, Value};

fn main() -> ExitCode {
    #[cfg(target_os = "linux")]
    platform::prefer_x11();
    let mut arguments = std::env::args().skip(1);
    let kind = arguments.next().unwrap_or_default();
    let mode = if kind == "play" { arguments.next() } else { None };
    // The rest comes in "--name value" pairs, so a value is free to begin with a dash, as a commit message may.
    let mut options = Map::new();
    while let (Some(name), Some(value)) = (arguments.next(), arguments.next()) {
        options.insert(name.trim_start_matches('-').to_string(), Value::String(value));
    }

    match (kind.as_str(), mode) {
        ("play", Some(mode)) => {
            options.insert("mode".to_string(), Value::String(mode));
            overlay::play(Value::Object(options))
        }
        ("show", _) => {
            let mut tour = String::new();
            if std::io::stdin().read_to_string(&mut tour).is_err() {
                eprintln!("gitceps: the tour didn't arrive");
                return ExitCode::FAILURE;
            }
            let text = |name: &str| {
                options
                    .get(name)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string()
            };
            tour::show(text("title"), text("subtitle"), tour)
        }
        _ => {
            eprintln!("gitceps-window shows Gitceps' windows, and gitceps starts it. Try: gitceps --help");
            ExitCode::FAILURE
        }
    }
}
