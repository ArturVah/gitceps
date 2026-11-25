//! The git hooks. `gitceps on` points git's global core.hooksPath at a folder of small scripts, one per hook
//! name, that each run `gitceps hook NAME`. While that option is set git stops looking in a repo's own hooks
//! folder, so every hook here first runs the repo's own hook of the same name.
mod checkout;
mod pull;
mod push;
mod rewrite;
mod secrets;

use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use crate::git::{ask, each_line};

/// Every hook git looks for by name, except the ones a stand-in would break:
///   push-to-checkout, proc-receive: git changes how it handles a push just because these exist
///   fsmonitor-watchman: git finds it through core.fsmonitor, not by name
///   post-index-change, reference-transaction: git runs them so often that everyday commands would slow down
pub const NAMES: [&str; 23] = [
    "applypatch-msg",
    "commit-msg",
    "p4-changelist",
    "p4-post-changelist",
    "p4-pre-submit",
    "p4-prepare-changelist",
    "post-applypatch",
    "post-checkout",
    "post-commit",
    "post-merge",
    "post-receive",
    "post-rewrite",
    "post-update",
    "pre-applypatch",
    "pre-auto-gc",
    "pre-commit",
    "pre-merge-commit",
    "pre-push",
    "pre-rebase",
    "pre-receive",
    "prepare-commit-msg",
    "sendemail-validate",
    "update",
];

/// A line in every script `gitceps on` writes. A repo's hook that has it is a copy of Gitceps' own or a link to
/// it, and running it would only start Gitceps again, for ever.
pub const MARK: &str = "# Gitceps' stand-in for a git hook";

/// Runs the hook git asked for, with the arguments git gave it, and returns its exit status.
pub fn run(name: &str, arguments: &[OsString]) -> u8 {
    match name {
        "pre-push" => push::run(arguments),
        "post-merge" => pull::after_merge(arguments),
        "post-rewrite" => rewrite::run(arguments),
        "post-checkout" => checkout::run(arguments),
        _ => own_hook(name, arguments, None),
    }
}

/// Runs the repo's own hook of this name, if it has one, and returns its exit status. `input` is what git
/// wrote for the hook, when Gitceps has read it first.
fn own_hook(name: &str, arguments: &[OsString], input: Option<&[u8]>) -> u8 {
    let Some(own) = own_hook_file(name) else { return 0 };
    let status = start(&own, arguments, input.is_some()).and_then(|mut hook| {
        if let (Some(mut feed), Some(input)) = (hook.stdin.take(), input) {
            let _ = feed.write_all(input); // a hook is free not to read it
        }
        hook.wait()
    });
    match status {
        Ok(status) => status.code().and_then(|code| u8::try_from(code).ok()).unwrap_or(1),
        Err(error) => {
            eprintln!("gitceps: can't run {}: {error}", own.display());
            1
        }
    }
}

/// The repo's own hook of this name, when it has one git would run.
fn own_hook_file(name: &str) -> Option<PathBuf> {
    // Git starts a hook in the repo's top folder, so in a normal checkout the hooks are in ./.git. Worktrees,
    // submodules and bare repos need git to say.
    let common = if std::env::var_os("GIT_DIR").is_none() && Path::new(".git").is_dir() {
        ".git".to_string()
    } else {
        ask(&["rev-parse", "--git-common-dir"])
    };
    if common.is_empty() {
        return None;
    }
    let own = Path::new(&common).join("hooks").join(name);
    let ours =
        || std::fs::read(&own).is_ok_and(|script| script.windows(MARK.len()).any(|part| part == MARK.as_bytes()));
    (runnable(&own) && !ours()).then_some(own)
}

#[cfg(unix)]
fn runnable(hook: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    hook.metadata()
        .is_ok_and(|file| file.is_file() && file.permissions().mode() & 0o111 != 0)
}

/// Starts a hook the way git would. A script without a "#!" line is run by sh, which Linux won't do by itself.
#[cfg(unix)]
fn start(hook: &Path, arguments: &[OsString], piped: bool) -> std::io::Result<Child> {
    const NOT_A_PROGRAM: i32 = 8; // ENOEXEC on Linux, macOS and the BSDs
    match command(Command::new(hook), arguments, piped).spawn() {
        Err(error) if error.raw_os_error() == Some(NOT_A_PROGRAM) => {
            let mut sh = Command::new("sh");
            sh.arg(hook);
            command(sh, arguments, piped).spawn()
        }
        started => started,
    }
}

/// Windows has no executable bit, and git treats every hook file as one.
#[cfg(windows)]
fn runnable(hook: &Path) -> bool {
    hook.is_file()
}

/// Windows can't start a script by its first line, so the shell git itself runs hooks with does it.
#[cfg(windows)]
fn start(hook: &Path, arguments: &[OsString], piped: bool) -> std::io::Result<Child> {
    let mut sh = Command::new("sh");
    sh.args(["-c", r#"exec "$0" "$@""#]).arg(hook);
    command(sh, arguments, piped).spawn()
}

/// A hook's command with git's arguments, and a pipe for its input when there is any.
fn command(mut command: Command, arguments: &[OsString], piped: bool) -> Command {
    command.args(arguments);
    if piped {
        command.stdin(Stdio::piped());
    }
    command
}

/// Everything git wrote for the hook to read.
fn input() -> Vec<u8> {
    let mut input = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut input);
    input
}

/// Whether a commit id is the row of zeros git uses for "no commit".
fn is_zero(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|digit| digit == b'0')
}

/// The repo's name as the cloud shows it, from a remote's URL: git@github.com:you/app.git -> app.
fn repo_name(url: &str) -> String {
    let mut parts = url.split(['/', '\\']).filter(|part| !part.is_empty()).rev();
    let last = parts.next().unwrap_or_default();
    // A repo's own .git folder is named for the folder it sits in: /srv/app/.git -> app.
    let name = if last == ".git" {
        parts.next().unwrap_or_default()
    } else {
        last
    };
    let name = name.strip_suffix(".git").unwrap_or(name);
    if name.is_empty() { "origin" } else { name }.to_string()
}

fn origin_name() -> String {
    repo_name(&ask(&["remote", "get-url", "origin"]))
}

/// The inserted and deleted lines a `git diff` or `git log` adds up to. A binary file counts for nothing.
fn changed_lines(arguments: &[&str]) -> (u64, u64) {
    let (mut ins, mut del) = (0, 0);
    each_line(&[arguments, &["--numstat"]].concat(), |line| {
        // "<inserted> <tab> <deleted> <tab> <path>", with "-" for both in a binary file.
        let mut columns = line.split('\t').map(str::parse::<u64>);
        if let (Some(Ok(more_ins)), Some(Ok(more_del))) = (columns.next(), columns.next()) {
            ins += more_ins;
            del += more_del;
        }
        true
    });
    (ins, del)
}
