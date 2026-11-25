//! The pre-push hook, which git runs just before a push. It measures what is being pushed, plays the
//! reaction for it, and blocks the push if it holds something that looks like a secret.
use std::ffi::OsString;
use std::path::Path;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use super::{changed_lines, input, is_zero, own_hook, repo_name, secrets};
use crate::git::{ask, each_line, succeeds};
use crate::switches::Switch;
use crate::window::play;

/// Git calls the hook with the remote's name and URL, and writes one line per ref for it to read:
/// <local ref> <local commit> <remote ref> <remote commit>.
pub fn run(arguments: &[OsString]) -> u8 {
    let refs = input();
    // The repo's own pre-push hook goes first. If it refuses, so does this one.
    let status = own_hook("pre-push", arguments, Some(&refs));
    if status != 0 {
        return status;
    }

    let refs = String::from_utf8_lossy(&refs);
    let mut tips = Vec::new();
    let mut rewritten = false;
    for line in refs.lines() {
        let [_, local, _, remote] = line.split_whitespace().collect::<Vec<_>>()[..] else {
            continue;
        };
        if is_zero(local) {
            continue; // a remote branch being deleted
        }
        tips.push(local);
        // The remote has commits this push would drop: someone pushed first, or history was rewritten.
        rewritten |= !is_zero(remote) && !succeeds(&["merge-base", "--is-ancestor", remote, local]);
    }
    if tips.is_empty() {
        return 0;
    }

    let secret = if Switch::Secrets.on() { secret(&tips) } else { None };
    if Switch::All.on() && Switch::Push.on() {
        let url = arguments.get(1).map(|url| url.to_string_lossy()).unwrap_or_default();
        let (ins, del) = changed_lines(&new_commits(&[], &tips));
        let mode = match ins + del {
            _ if secret.is_some() => "secret",
            _ if rewritten && forced() => "force",
            _ if rewritten => "rejected", // git will refuse it
            ..=10 => "tiny",
            10_000.. => "forklift",
            400.. => "huge",
            _ => "normal",
        };
        play(
            mode,
            &[
                ("--ins", &ins.to_string()),
                ("--del", &del.to_string()),
                ("--msg", &ask(&["log", "-1", "--format=%s", tips[0]])),
                ("--secret", secret.as_deref().unwrap_or_default()),
                ("--remote", &repo_name(&url)),
            ],
        );
    }

    if let Some(secret) = secret {
        eprintln!("gitceps: NOPE. That looks like a secret ({secret}). Push blocked.");
        eprintln!("    If it's a fake test value: git push --no-verify");
        return 1;
    }
    0
}

/// `git log` arguments for the commits no remote has yet, each listed once even when several refs share them.
/// Commits another remote has don't count: on a fork they are other people's work.
fn new_commits<'a>(options: &[&'a str], tips: &[&'a str]) -> Vec<&'a str> {
    [&["log", "--format="], options, tips, &["--not", "--remotes"]].concat()
}

/// The file that gives a secret away, or "key in code".
fn secret(tips: &[&str]) -> Option<String> {
    let mut found = None;
    each_line(&new_commits(&["--name-only", "--diff-filter=d"], tips), |path| {
        found = secrets::is_secret_file(path).then(|| path.to_string());
        found.is_none()
    });
    // Pinned, so the user's colours, external diff tool or text conversion can't hide a line. A merge gets a
    // pass of its own for what resolving it added, which a plain patch leaves out. Git before 2.36 can't show
    // that, and prints nothing.
    let pinned = ["-p", "-U0", "--no-color", "--no-ext-diff", "--no-textconv"];
    for merges in [&[][..], &["--merges", "--remerge-diff"]] {
        if found.is_none() {
            each_line(&new_commits(&[&pinned, merges].concat(), tips), |line| {
                if secrets::adds_key(line) {
                    found = Some("key in code".to_string());
                }
                found.is_none()
            });
        }
    }
    found
}

/// Git doesn't tell a hook whether the push was forced, so this reads the command line of the git that
/// started it: the nearest git above this process. On Windows a shell sits between the two.
fn forced() -> bool {
    let mut system = System::new();
    let mut above = sysinfo::get_current_pid().ok();
    // This process, perhaps a shell, then git: a few steps up at most.
    for _ in 0..4 {
        let Some(pid) = above else { break };
        let command_line = ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always);
        system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), false, command_line);
        let Some(process) = system.process(pid) else { break };
        if Path::new(process.name()).file_stem().is_some_and(|name| name == "git") {
            // An alias such as pf, for "push --force-with-lease", reaches here already expanded.
            return process.cmd().iter().skip(1).any(|word| forces(&word.to_string_lossy()));
        }
        above = process.parent();
    }
    false
}

/// --force and its longer forms, -f alone or among other short options, and a "+" before a refspec.
fn forces(argument: &str) -> bool {
    let short_options = argument.strip_prefix('-').filter(|rest| !rest.starts_with('-'));
    argument.starts_with("--force")
        || (argument.starts_with('+') && argument.len() > 1)
        || short_options.is_some_and(|options| options.contains('f') && options.chars().all(char::is_alphabetic))
}
