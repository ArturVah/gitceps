//! The pull reaction, and the post-merge hook that plays it. Git runs that hook after a merge or pull that
//! brought commits in. Only a pull gets a reaction: `git pull` itself, or merging your upstream the way an
//! IDE's fetch-then-merge does. Merging a local branch stays quiet.
use std::collections::HashMap;
use std::ffi::OsString;

use super::{changed_lines, origin_name, own_hook};
use crate::git::ask;
use crate::switches::Switch;
use crate::window::play;

pub fn after_merge(arguments: &[OsString]) -> u8 {
    own_hook("post-merge", arguments, None); // git ignores this hook's exit status
    if Switch::All.on() {
        // Git labels its work: "pull ..." during `git pull`, "merge <what>" for a plain merge.
        let action = reflog_action();
        if action.starts_with("pull") || action.strip_prefix("merge ").is_some_and(is_upstream) {
            reaction("ORIG_HEAD", "HEAD");
        }
    }
    0
}

/// What git says it is doing, such as "pull origin main" or "rebase (continue)".
pub fn reflog_action() -> String {
    std::env::var("GIT_REFLOG_ACTION").unwrap_or_default()
}

/// Does a ref point at the same commit as the current branch's upstream (origin/main, say)?
fn is_upstream(reference: &str) -> bool {
    let commits = ask(&["rev-parse", reference, "@{upstream}"]);
    matches!(commits.lines().collect::<Vec<_>>()[..], [commit, upstream] if commit == upstream)
}

/// Plays the pull reaction for the commits between `base`, where the branch was, and `tip`, where they end.
/// That it was a pull is for the caller to know. Pulls under 50 lines, or with nothing new, stay quiet.
pub fn reaction(base: &str, tip: &str) {
    if !Switch::Pull.on() {
        return;
    }
    // Author <tab> subject, newest first. A commit with no message has no tab once the end is trimmed.
    let log = ask(&["log", "--no-merges", "--format=%an%x09%s", &format!("{base}..{tip}")]);
    if log.is_empty() {
        return;
    }
    let (ins, del) = changed_lines(&["diff", &format!("{base}...{tip}")]);
    if ins + del < 50 {
        return;
    }
    let commits: Vec<(&str, &str)> = log
        .lines()
        .map(|line| line.split_once('\t').unwrap_or((line, "")))
        .collect();
    let subjects: Vec<&str> = commits.iter().map(|&(_, subject)| subject).take(3).collect();
    let mut by_author: HashMap<&str, usize> = HashMap::new();
    for &(author, _) in &commits {
        *by_author.entry(author).or_default() += 1;
    }
    let busiest = by_author.into_iter().max_by_key(|&(author, count)| (count, author));
    play(
        "pull",
        &[
            ("--ins", &ins.to_string()),
            ("--del", &del.to_string()),
            ("--commits", &commits.len().to_string()),
            ("--msg", subjects.first().copied().unwrap_or_default()),
            ("--msgs", &subjects.join("\n")),
            ("--from", busiest.map_or("", |(author, _)| author)),
            ("--remote", &origin_name()),
        ],
    );
}
