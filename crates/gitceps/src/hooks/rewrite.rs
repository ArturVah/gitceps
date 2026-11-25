//! The post-rewrite hook. Git runs it after `commit --amend` and after a rebase, and writes one
//! "<old commit> <new commit>" line per rewritten commit for it to read.
//!   amend:  plays only when the commit was already pushed (he sneaks in to swap a plate)
//!   rebase: `git pull --rebase` gets the pull reaction; a rebase you run yourself, of 3 commits or more,
//!           gets juggled
use std::ffi::OsString;

use super::pull::{self, reflog_action};
use super::{input, origin_name, own_hook};
use crate::git::ask;
use crate::switches::Switch;
use crate::window::play;

pub fn run(arguments: &[OsString]) -> u8 {
    let rewrites = input();
    own_hook("post-rewrite", arguments, Some(&rewrites)); // git ignores this hook's exit status
    let rewrites = String::from_utf8_lossy(&rewrites);
    let mut first = rewrites.split_whitespace();
    let (Some(old), Some(new)) = (first.next(), first.next()) else {
        return 0;
    };
    if !Switch::All.on() {
        return 0;
    }

    match arguments.first().and_then(|kind| kind.to_str()) {
        // A commit nobody has seen yet can be amended in peace.
        Some("amend") => {
            if Switch::Amend.on() && old != new && !ask(&["branch", "-r", "--contains", old]).is_empty() {
                let subject = ask(&["log", "-1", "--format=%s", new]);
                play(
                    "amend",
                    &[("--commits", "1"), ("--msg", &subject), ("--remote", &origin_name())],
                );
            }
        }
        Some("rebase") => {
            // `git pull --rebase` labels its work "pull ...". After it stopped on a conflict and was continued,
            // that label is left only in the rebase's "(start)" entry in HEAD's reflog.
            let started = ask(&["reflog", "-1", "--grep-reflog= (start): ", "--format=%gs"]);
            if reflog_action().starts_with("pull") || started.starts_with("pull") {
                // The first rewritten commit sits right on top of the new base, so old..onto is what came in.
                let onto = ask(&["rev-parse", "-q", "--verify", &format!("{new}^")]);
                if !onto.is_empty() {
                    pull::reaction(old, &onto);
                }
                return 0;
            }
            let count = rewrites.lines().filter(|line| !line.is_empty()).count();
            if Switch::Rebase.on() && count >= 3 {
                let subject = ask(&["log", "-1", "--format=%s", "HEAD"]);
                let count = count.to_string();
                play(
                    "rebase",
                    &[("--commits", &count), ("--msg", &subject), ("--remote", &origin_name())],
                );
            }
        }
        _ => {}
    }
    0
}
