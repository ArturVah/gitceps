//! Reading from git: running it, and reading many files of a commit at once.
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

/// What git itself clears before it works in another repository (`git rev-parse --local-env-vars`). A git hook
/// inherits these, and they would win over `-C repo`.
pub const REPO_ENV: [&str; 15] = [
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_OBJECT_DIRECTORY",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_GRAFT_FILE",
    "GIT_INDEX_FILE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_REPLACE_REF_BASE",
    "GIT_PREFIX",
    "GIT_SHALLOW_FILE",
    "GIT_COMMON_DIR",
];

/// Runs the git on the PATH in a repo, with `input` on its standard input, and returns what it printed.
pub fn git_with_input(arguments: &[&str], repo: &Path, input: Option<&[u8]>) -> Result<Vec<u8>> {
    let mut command = Command::new("git");
    for name in REPO_ENV {
        command.env_remove(name);
    }
    let mut child = command
        .arg("-C")
        .arg(repo)
        .args(["-c", "core.quotePath=false"])
        .args(arguments)
        .env_remove("GIT_DIFF_OPTS") // it would override the context lines a caller asks for
        .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("can't run git")?;
    let feed = child.stdin.take();
    // Written while the output is read, so neither pipe can fill up and stall git.
    let output = std::thread::scope(|scope| {
        if let (Some(mut feed), Some(input)) = (feed, input) {
            scope.spawn(move || feed.write_all(input));
        }
        child.wait_with_output()
    })?;
    if !output.status.success() {
        bail!(
            "git {}: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}

pub fn git(arguments: &[&str], repo: &Path) -> Result<String> {
    Ok(String::from_utf8_lossy(&git_with_input(arguments, repo, None)?).into_owned())
}

/// A git command for where we are: the repository git started a hook for, or the folder a command was typed
/// in. A hook's own input stays unread.
fn here(arguments: &[&str]) -> Command {
    let mut command = Command::new("git");
    command
        .args(["-c", "core.quotePath=false"])
        .args(arguments)
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// What git prints where we are, without the line break at its end. Empty when git fails: a branch without an
/// upstream or an option nobody set is an answer here, not an error.
pub fn ask(arguments: &[&str]) -> String {
    match here(arguments).output() {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout).trim_end().to_string(),
        _ => String::new(),
    }
}

/// Whether a git command succeeds where we are.
pub fn succeeds(arguments: &[&str]) -> bool {
    here(arguments)
        .stdout(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Hands each line git prints to `more`, as it comes, and stops git once `more` returns false. Output of any
/// size (every line of a repo's history, on its first push) costs no memory.
pub fn each_line(arguments: &[&str], mut more: impl FnMut(&str) -> bool) {
    let Ok(mut child) = here(arguments).stdout(Stdio::piped()).spawn() else {
        return;
    };
    let mut output = BufReader::new(child.stdout.take().expect("stdout is piped"));
    let mut line = Vec::new();
    while output.read_until(b'\n', &mut line).is_ok_and(|read| read > 0)
        && more(String::from_utf8_lossy(&line).trim_end_matches(['\n', '\r']))
    {
        line.clear();
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Files as they are in a commit, as lines, read by one git process rather than one per file. A path that isn't
/// a file there (missing, a folder or a submodule) is left out, and so is one with a line break in it, which
/// can't be asked for.
pub fn files(paths: &[String], commit: &str, repo: &Path) -> Result<HashMap<String, Vec<String>>> {
    let paths: Vec<&String> = paths.iter().filter(|path| !path.contains('\n')).collect();
    let asked: String = paths.iter().map(|path| format!("{commit}:{path}\n")).collect();
    let data = git_with_input(&["cat-file", "--batch"], repo, Some(asked.as_bytes()))?;
    let mut found = HashMap::new();
    let mut at = 0;
    for path in paths {
        // Each answer is "<id> <type> <size>\n<content>\n", or "<name> missing\n".
        let Some(end) = data[at..]
            .iter()
            .position(|&byte| byte == b'\n')
            .map(|offset| at + offset)
        else {
            break;
        };
        let header = String::from_utf8_lossy(&data[at..end]);
        let header: Vec<&str> = header.split(' ').collect();
        at = end + 1;
        let [_, kind, size] = header[..] else { continue };
        let Ok(size) = size.parse::<usize>() else { continue };
        let Some(content) = data.get(at..at + size) else { break };
        if kind == "blob" {
            found.insert(path.clone(), text_lines(&String::from_utf8_lossy(content)));
        }
        at += size + 1;
    }
    Ok(found)
}

/// Splits on "\n" and drops the "\r" of a Windows line ending.
pub fn text_lines(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line).to_string())
        .collect();
    if lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}
