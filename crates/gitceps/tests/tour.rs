//! `gitceps tour`, run on a small repo built here. What fixtures/ expects was recorded from the Swift app
//! this code was ported from, on the same repo.
use std::path::Path;
use std::process::{Command, Output, Stdio};

use serde_json::Value;
use tempfile::TempDir;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

/// Runs git with fixed names and dates, so the commits get the same hashes on every machine.
fn git(repo: &Path, arguments: &[&str], input: &str) -> String {
    use std::io::Write;
    let mut command = Command::new("git");
    for name in gitceps::git::REPO_ENV {
        command.env_remove(name);
    }
    let mut child = command
        .arg("-C")
        .arg(repo)
        .args(arguments)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", repo.join("no-such-config"))
        .env("GIT_AUTHOR_NAME", "Push Bro")
        .env("GIT_AUTHOR_EMAIL", "bro@example.com")
        .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z")
        .env("GIT_COMMITTER_NAME", "Push Bro")
        .env("GIT_COMMITTER_EMAIL", "bro@example.com")
        .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("git runs");
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "git {arguments:?} failed");
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn write(repo: &Path, path: &str, content: impl AsRef<[u8]>) {
    let file = repo.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, content).unwrap();
}

fn numbered(count: usize) -> String {
    (1..=count).map(|n| format!("row {n}\n")).collect()
}

/// A repo whose `before..after` holds every kind of change a tour can meet, with the tour files beside it.
/// Modes, the symlink and the submodule are set in the index, so this also works where the file system
/// has none of them.
fn fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"], "");
    git(&repo, &["config", "core.autocrlf", "false"], "");

    write(
        &repo,
        "src/app.js",
        "function total(items) {\n  let sum = 0;\n  for (const item of items) {\n    sum += item.price;\n  }\n  return sum;\n}\n\n\
         function label(item) {\n  return item.name;\n}\n\nmodule.exports = { total, label };\n// end\n",
    );
    write(&repo, "old name.txt", "one\ntwo\nthree\nfour\nfive\n");
    write(&repo, "gone.txt", "first\n\nlast\n");
    write(&repo, "script.sh", "#!/bin/sh\necho hi\n");
    write(&repo, "link", "plain file\n");
    write(&repo, "win.txt", "alpha\r\nbeta\r\ngamma\r\n");
    write(&repo, "logo.bin", [0u8, 1, 2, 3]);
    write(
        &repo,
        "context.md",
        "# Notes\n\nPrices are in cents.\nCounts are whole numbers.\n",
    );
    write(&repo, "tail.txt", "a\nb\nc\n");
    git(&repo, &["add", "-A"], "");
    git(
        &repo,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            "160000,1111111111111111111111111111111111111111,sub",
        ],
        "",
    );
    git(&repo, &["commit", "-q", "-m", "base"], "");
    git(&repo, &["tag", "before"], "");

    write(
        &repo,
        "src/app.js",
        "function total(items) {\n  let sum = 0;\n  for (const item of items) {\n    if (!item.price) continue;\n    \
         sum += item.price * item.count;\n  }\n  return sum;\n}\n\nfunction label(item) {\n  return item.name.trim();\n}\n\n\
         function count(items) {\n  return items.length;\n}\n\nmodule.exports = { total, label, count };\n",
    );
    std::fs::rename(repo.join("old name.txt"), repo.join("new name.txt")).unwrap();
    write(&repo, "new name.txt", "one\ntwo\n3\nfour\nfive\n");
    std::fs::remove_file(repo.join("gone.txt")).unwrap();
    std::fs::remove_file(repo.join("link")).unwrap();
    write(&repo, "win.txt", "alpha\r\nBETA\r\ngamma\r\ndelta\r\n");
    write(&repo, "logo.bin", [0u8, 9, 9, 9, 9]);
    write(&repo, "added.rs", "fn main() {\n\n    println!(\"hi\");\n}\n");
    write(&repo, "long.txt", numbered(320));
    write(&repo, "tail.txt", "a\nb\n");
    git(&repo, &["add", "-A"], "");
    git(&repo, &["update-index", "--chmod=+x", "script.sh"], "");
    let target = git(&repo, &["hash-object", "-w", "--stdin"], "context.md");
    git(
        &repo,
        &["update-index", "--add", "--cacheinfo", &format!("120000,{target},link")],
        "",
    );
    git(
        &repo,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            "160000,2222222222222222222222222222222222222222,sub",
        ],
        "",
    );
    git(&repo, &["commit", "-q", "-m", "head"], "");
    git(&repo, &["tag", "after"], "");

    for tour in ["full.json", "broken.json"] {
        std::fs::copy(format!("{FIXTURES}/{tour}"), dir.path().join(tour)).unwrap();
    }
    dir
}

/// `gitceps tour FILE` in dry-run mode, run on a tour beside the fixture repo, without the user's git config.
fn tour(dir: &TempDir, file: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gitceps"))
        .args(["tour", file])
        .current_dir(dir.path())
        .env("GITCEPS_DRY", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", dir.path().join("no-such-config"))
        .output()
        .expect("gitceps runs")
}

#[test]
fn a_tour_of_every_kind_of_change_builds_the_recorded_page() {
    let dir = fixture();
    let output = tour(&dir, "full.json");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let page: Value = serde_json::from_slice(&output.stdout).unwrap();
    let expected: Value =
        serde_json::from_str(&std::fs::read_to_string(format!("{FIXTURES}/full.page.json")).unwrap()).unwrap();
    assert_eq!(page, expected);
}

#[test]
fn a_broken_tour_gets_every_problem_at_once() {
    let dir = fixture();
    let output = tour(&dir, "broken.json");
    assert!(!output.status.success());
    let expected = std::fs::read_to_string(format!("{FIXTURES}/broken.txt")).unwrap();
    assert_eq!(String::from_utf8_lossy(&output.stderr), expected);
}
