//! The git hooks and the switches that turn them on, run by real git commands on throwaway repos. With
//! GITCEPS_DRY=1 a hook prints the reaction it would play, and that line is what these tests read.
use std::cell::Cell;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

/// A folder holding a "server" (the bare repo `server.git`), my clone of it in `mine` with one commit pushed,
/// and a teammate's clone in `team`. The folder stands in for the home folder, with a git config of its own,
/// and Gitceps is on in it.
struct Sandbox {
    dir: TempDir,
    files: Cell<usize>,
}

impl Sandbox {
    fn new() -> Sandbox {
        let sandbox = Sandbox {
            dir: tempfile::tempdir().unwrap(),
            files: Cell::new(0),
        };
        // Colours forced on, as some people have them, so every test also shows they change nothing.
        let config = "[user]\n\tname = Me\n\temail = me@example.com\n[init]\n\tdefaultBranch = main\n\
                      [protocol \"file\"]\n\tallow = always\n[core]\n\tautocrlf = false\n\
                      [color]\n\tui = always\n";
        std::fs::write(sandbox.path().join(".gitconfig"), config).unwrap();
        let (on, said) = sandbox.gitceps(".", &["on"]);
        assert!(on, "{said}");
        sandbox.git(".", &["init", "-q", "--bare", "server.git"]);
        sandbox.git(".", &["clone", "-q", "server.git", "mine"]);
        sandbox.write("mine/README.md", "# App\n");
        sandbox.git("mine", &["add", "-A"]);
        sandbox.git("mine", &["commit", "-qm", "first commit"]);
        sandbox.git("mine", &["push", "-q"]);
        sandbox.git(".", &["clone", "-q", "server.git", "team"]);
        sandbox.git("team", &["config", "user.name", "Sam"]);
        sandbox
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    /// A program set to run in the sandbox, where it sees the sandbox's home folder and git config and not
    /// the real ones.
    fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        for name in gitceps::git::REPO_ENV {
            command.env_remove(name);
        }
        command
            .current_dir(self.path())
            .env_remove("GIT_REFLOG_ACTION")
            .env_remove("CLAUDE_CONFIG_DIR")
            .env_remove("CODEX_HOME")
            .env("HOME", self.path())
            .env("USERPROFILE", self.path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.path().join(".gitconfig"))
            .env("GIT_EDITOR", "true")
            .env("GITCEPS_DRY", "1");
        command
    }

    /// Runs git in one of the sandbox's repos, or with "." in the folder they sit in.
    fn output(&self, repo: &str, arguments: &[&str]) -> Output {
        let mut git = self.command("git");
        git.arg("-C").arg(repo).args(arguments).output().expect("git runs")
    }

    /// Runs gitceps in one of the repos, and returns whether it succeeded and everything it printed.
    fn gitceps(&self, repo: &str, arguments: &[&str]) -> (bool, String) {
        let mut gitceps = self.command(env!("CARGO_BIN_EXE_gitceps"));
        let output = gitceps
            .current_dir(self.path().join(repo))
            .args(arguments)
            .output()
            .unwrap();
        let printed = [output.stdout, output.stderr].concat();
        (output.status.success(), String::from_utf8_lossy(&printed).into_owned())
    }

    /// Runs a git command that sets a test up, and stops the test if it fails.
    fn git(&self, repo: &str, arguments: &[&str]) {
        let output = self.output(repo, arguments);
        let said = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "git {arguments:?} in {repo}: {said}");
    }

    /// Everything a git command printed for the person running it, the hooks' lines included.
    fn said(&self, repo: &str, arguments: &[&str]) -> String {
        String::from_utf8_lossy(&self.output(repo, arguments).stderr).into_owned()
    }

    /// The reactions a git command set off, by name: "pull", or "" when Gitceps stayed quiet.
    fn played(&self, repo: &str, arguments: &[&str]) -> String {
        self.said(repo, arguments)
            .lines()
            .filter_map(|line| line.strip_prefix("gitceps would play: "))
            .map(|play| play.split(' ').next().unwrap_or_default())
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn write(&self, path: &str, content: &str) {
        let file = self.path().join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, content).unwrap();
    }

    /// Commits a new file of `lines` lines in a repo.
    fn commit(&self, repo: &str, lines: usize, message: &str) {
        let file = self.files.get() + 1;
        self.files.set(file);
        let content: String = (1..=lines).map(|line| format!("file {file}, line {line}\n")).collect();
        self.write(&format!("{repo}/file{file}.txt"), &content);
        self.git(repo, &["add", "-A"]);
        self.git(repo, &["commit", "-qm", message]);
    }

    /// Commits `count` files of 30 lines each in my clone, without pushing them.
    fn work(&self, count: usize) {
        for _ in 0..count {
            self.commit("mine", 30, "my work");
        }
    }

    /// The teammate catches up, then pushes `count` commits of 40 lines each.
    fn teammate(&self, count: usize) {
        self.git("team", &["pull", "-q", "--no-rebase"]);
        for _ in 0..count {
            self.commit("team", 40, "team work");
        }
        self.git("team", &["push", "-q"]);
    }

    /// Gives a repo a hook of its own, the kind Gitceps has to keep running.
    fn own_hook(&self, repo: &str, name: &str, script: &str) {
        let path = format!("{repo}/.git/hooks/{name}");
        self.write(&path, script);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(self.path().join(path), std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
}

#[test]
fn a_push_is_sized_by_its_changed_lines() {
    let sandbox = Sandbox::new();
    let sizes = [
        (1, "tiny"),
        (10, "tiny"),
        (11, "normal"),
        (399, "normal"),
        (400, "huge"),
        (9_999, "huge"),
        (10_000, "forklift"),
    ];
    for (lines, reaction) in sizes {
        sandbox.commit("mine", lines, "more code");
        assert_eq!(sandbox.played("mine", &["push"]), reaction, "{lines} lines");
    }
}

#[test]
fn a_push_reports_its_lines_its_last_message_and_the_repo() {
    let sandbox = Sandbox::new();
    sandbox.commit("mine", 60, "feat: add dark mode");
    sandbox.git("mine", &["rm", "-q", "README.md"]);
    sandbox.git("mine", &["commit", "-qm", "chore: drop the readme"]);
    let said = sandbox.said("mine", &["push"]);
    let expected =
        "gitceps would play: normal (--ins 60 --del 1 --msg chore: drop the readme --secret  --remote server)";
    assert!(said.contains(expected), "{said}");
}

#[test]
fn a_push_that_sends_no_commits_stays_quiet() {
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.played("mine", &["push"]), "", "nothing new");
    sandbox.git("mine", &["push", "-q", "--no-verify", "origin", "main:old"]);
    assert_eq!(
        sandbox.played("mine", &["push", "origin", "--delete", "old"]),
        "",
        "a deleted branch"
    );
}

#[test]
fn a_push_over_someone_elses_commits_is_rejected_unless_forced() {
    let sandbox = Sandbox::new();
    sandbox.teammate(1);
    sandbox.commit("mine", 5, "mine");
    assert_eq!(sandbox.played("mine", &["push"]), "rejected");

    sandbox.git("mine", &["config", "alias.pf", "push --force-with-lease"]);
    let forced: [&[&str]; 5] = [
        &["push", "--force"],
        &["push", "-f"],
        &["push", "--force-with-lease"],
        &["push", "origin", "+main"],
        &["pf"],
    ];
    for (take, push) in forced.into_iter().enumerate() {
        assert_eq!(sandbox.played("mine", push), "force", "{push:?}");
        sandbox.git("mine", &["commit", "-q", "--amend", "-m", &format!("take {take}")]);
    }
}

#[test]
fn a_push_with_a_secret_is_blocked() {
    let sandbox = Sandbox::new();
    let push = |path: &str, content: &str| {
        sandbox.write(&format!("mine/{path}"), content);
        sandbox.git("mine", &["add", "-A", "--force"]);
        sandbox.git("mine", &["commit", "-qm", "chore: add config"]);
        let output = sandbox.output("mine", &["push"]);
        sandbox.git("mine", &["reset", "-q", "--hard", "origin/main"]);
        (
            output.status.success(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    };
    // Put together here, so nothing shaped like a real key sits in this repo.
    let token = format!("ghp_{}", "a1B2".repeat(6));
    let api_key = format!("sk-{}", "a1B2".repeat(10));

    let secrets = [
        (".env", "PORT=3000\n".to_string(), ".env"),
        ("deploy/server.pem", "certificate\n".to_string(), "deploy/server.pem"),
        ("api.js", format!("const token = '{token}';\n"), "key in code"),
        ("client.py", format!("client = Client({api_key})\n"), "key in code"),
        (
            "settings.py",
            "PASSWORD = \"correct-horse\"\n".to_string(),
            "key in code",
        ),
    ];
    for (path, content, found) in secrets {
        let (pushed, said) = push(path, &content);
        assert!(!pushed, "{path} was pushed");
        assert!(said.contains(&format!("--secret {found} --remote server)")), "{said}");
        let refusal = format!("gitceps: NOPE. That looks like a secret ({found}). Push blocked.");
        assert!(said.contains(&refusal), "{said}");
    }

    let harmless = [
        (".env.example", "PORT=3000\n".to_string()),
        ("login.js", "const password = request.body.password;\n".to_string()),
        ("notes.md", format!("see ta{api_key}\n")),
    ];
    for (path, content) in harmless {
        let (pushed, said) = push(path, &content);
        assert!(pushed, "{path}: {said}");
    }

    // A file renamed to one that holds secrets.
    sandbox.write("mine/config.local", "PORT=3000\n");
    sandbox.git("mine", &["add", "-A"]);
    sandbox.git("mine", &["commit", "-qm", "local config"]);
    sandbox.git("mine", &["push", "-q", "--no-verify"]);
    sandbox.git("mine", &["mv", "config.local", ".env"]);
    sandbox.git("mine", &["commit", "-qm", "chore: use dotenv"]);
    let output = sandbox.output("mine", &["push"]);
    sandbox.git("mine", &["reset", "-q", "--hard", "origin/main"]);
    assert!(!output.status.success(), "a renamed .env was pushed");

    // A key typed while settling a merge conflict is in no commit's own patch.
    sandbox.git("team", &["pull", "-q", "--no-rebase"]);
    sandbox.write("team/settings.py", "DEBUG = False\n");
    sandbox.git("team", &["add", "-A"]);
    sandbox.git("team", &["commit", "-qm", "theirs"]);
    sandbox.git("team", &["push", "-q"]);
    sandbox.write("mine/settings.py", "DEBUG = True\n");
    sandbox.git("mine", &["add", "-A"]);
    sandbox.git("mine", &["commit", "-qm", "mine"]);
    assert!(!sandbox.output("mine", &["pull", "-q", "--no-rebase"]).status.success());
    sandbox.write("mine/settings.py", &format!("DEBUG = False\nTOKEN = '{token}'\n"));
    sandbox.git("mine", &["add", "-A"]);
    sandbox.git("mine", &["commit", "-qm", "merge"]);
    let output = sandbox.output("mine", &["push"]);
    sandbox.git("mine", &["reset", "-q", "--hard", "origin/main"]);
    assert!(!output.status.success(), "a key added in a merge was pushed");

    // Taking a key out is fine: only added lines are read.
    sandbox.write("mine/api.js", &format!("const token = '{token}';\n"));
    sandbox.git("mine", &["add", "-A"]);
    sandbox.git("mine", &["commit", "-qm", "a key"]);
    sandbox.git("mine", &["push", "-q", "--no-verify"]);
    let (pushed, said) = push("api.js", "const token = process.env.TOKEN;\n");
    assert!(pushed, "{said}");
}

#[test]
fn only_a_fresh_clone_gets_the_clone_reaction() {
    let sandbox = Sandbox::new();
    sandbox.git(".", &["init", "-q", "--bare", "library.git"]);
    sandbox.git(".", &["clone", "-q", "library.git", "library"]);
    sandbox.commit("library", 1, "library");
    sandbox.git("library", &["push", "-q"]);
    sandbox.git("mine", &["submodule", "add", "-q", "../library.git", "library"]);
    sandbox.git("mine", &["commit", "-qm", "add the library"]);
    sandbox.git("mine", &["push", "-q"]);

    // Once for the repo, and not again for the submodule inside it.
    let said = sandbox.said(".", &["clone", "-q", "--recurse-submodules", "server.git", "fresh"]);
    let plays: Vec<&str> = said.lines().filter(|line| line.contains("would play")).collect();
    assert_eq!(
        plays,
        ["gitceps would play: clone (--ins 3 --msg add the library --remote server)"]
    );

    assert_eq!(
        sandbox.played("mine", &["worktree", "add", "-q", "../tree"]),
        "",
        "a new worktree"
    );
    assert_eq!(
        sandbox.played("mine", &["checkout", "-q", "-b", "topic"]),
        "",
        "a new branch"
    );
    sandbox.git(".", &["init", "-q", "started-empty"]);
    sandbox.git("started-empty", &["remote", "add", "origin", "../server.git"]);
    sandbox.git("started-empty", &["fetch", "-q"]);
    assert_eq!(
        sandbox.played("started-empty", &["checkout", "-q", "main"]),
        "",
        "a first checkout"
    );
}

#[test]
fn a_pull_plays_once_it_brings_fifty_lines() {
    let sandbox = Sandbox::new();
    sandbox.teammate(1);
    assert_eq!(sandbox.played("mine", &["pull", "-q", "--no-rebase"]), "", "40 lines");

    for message in ["feat: search", "feat: filters", "fix: typo"] {
        sandbox.commit("team", 40, message);
    }
    sandbox.git("team", &["push", "-q"]);
    let said = sandbox.said("mine", &["pull", "-q", "--no-rebase"]);
    let expected = "gitceps would play: pull (--ins 120 --del 0 --commits 3 --msg fix: typo \
                    --msgs fix: typo\nfeat: filters\nfeat: search --from Sam --remote server)";
    assert!(said.contains(expected), "{said}");
}

#[test]
fn a_pull_up_to_a_merge_commit_counts_both_of_its_sides() {
    let sandbox = Sandbox::new();
    sandbox.git("team", &["checkout", "-q", "-b", "feature"]);
    sandbox.commit("team", 60, "feature work");
    sandbox.git("team", &["checkout", "-q", "main"]);
    sandbox.commit("team", 60, "main work");
    sandbox.git("team", &["merge", "-q", "--no-ff", "--no-edit", "feature"]);
    sandbox.git("team", &["push", "-q"]);
    let said = sandbox.said("mine", &["pull", "-q", "--no-rebase"]);
    assert!(said.contains("pull (--ins 120 --del 0 --commits 2 "), "{said}");
}

#[test]
fn a_pull_that_rebases_my_commits_is_still_a_pull() {
    let sandbox = Sandbox::new();
    sandbox.teammate(2);
    sandbox.work(3);
    // Only the teammate's 80 lines came in. My own 90 were already here.
    let said = sandbox.said("mine", &["pull", "-q", "--rebase"]);
    assert!(
        said.contains("gitceps would play: pull (--ins 80 --del 0 --commits 2 "),
        "{said}"
    );
}

#[test]
fn a_pull_that_stopped_on_a_conflict_plays_when_the_rebase_goes_on() {
    let sandbox = Sandbox::new();
    sandbox.write("team/clash.txt", &"theirs\n".repeat(80));
    sandbox.git("team", &["add", "-A"]);
    sandbox.git("team", &["commit", "-qm", "theirs"]);
    sandbox.git("team", &["push", "-q"]);
    sandbox.write("mine/clash.txt", "mine\n");
    sandbox.git("mine", &["add", "-A"]);
    sandbox.git("mine", &["commit", "-qm", "mine"]);
    sandbox.work(3);

    assert!(!sandbox.output("mine", &["pull", "-q", "--rebase"]).status.success());
    sandbox.write("mine/clash.txt", "settled\n");
    sandbox.git("mine", &["add", "-A"]);
    assert_eq!(sandbox.played("mine", &["rebase", "--continue"]), "pull");
}

#[test]
fn merging_the_upstream_by_hand_is_a_pull() {
    let sandbox = Sandbox::new();
    sandbox.teammate(2);
    sandbox.git("mine", &["fetch", "-q"]);
    assert_eq!(sandbox.played("mine", &["merge", "-q", "origin/main"]), "pull");

    // `git pull origin main` on a branch that tracks another one.
    sandbox.git("mine", &["checkout", "-q", "-b", "feature"]);
    sandbox.git("mine", &["push", "-q", "-u", "origin", "feature"]);
    sandbox.teammate(2);
    assert_eq!(
        sandbox.played("mine", &["pull", "-q", "--no-rebase", "origin", "main"]),
        "pull"
    );
}

#[test]
fn merging_a_local_branch_stays_quiet() {
    let sandbox = Sandbox::new();
    sandbox.git("mine", &["checkout", "-q", "-b", "feature"]);
    sandbox.work(3);
    sandbox.git("mine", &["push", "-q", "-u", "origin", "feature"]);
    sandbox.git("mine", &["checkout", "-q", "main"]);
    assert_eq!(
        sandbox.played("mine", &["merge", "-q", "feature"]),
        "",
        "a pushed branch"
    );

    sandbox.git("mine", &["push", "-q"]);
    sandbox.teammate(2);
    sandbox.git("mine", &["fetch", "-q"]);
    sandbox.git("mine", &["checkout", "-q", "-b", "topic", "origin/main"]);
    sandbox.work(1);
    sandbox.git("mine", &["checkout", "-q", "main"]);
    assert_eq!(
        sandbox.played("mine", &["merge", "-q", "topic"]),
        "",
        "a branch started from newer upstream commits"
    );
}

#[test]
fn a_rebase_of_three_commits_or_more_is_juggled() {
    let sandbox = Sandbox::new();
    let branch_then_main_moves = |branch: &str, commits: usize| {
        sandbox.git("mine", &["checkout", "-q", "-b", branch, "main"]);
        sandbox.work(commits);
        sandbox.git("mine", &["checkout", "-q", "main"]);
        sandbox.work(1);
        sandbox.git("mine", &["checkout", "-q", branch]);
    };

    branch_then_main_moves("feature", 3);
    let said = sandbox.said("mine", &["rebase", "-q", "main"]);
    let expected = "gitceps would play: rebase (--commits 3 --msg my work --remote server)";
    assert!(said.contains(expected), "{said}");

    branch_then_main_moves("small", 2);
    assert_eq!(sandbox.played("mine", &["rebase", "-q", "main"]), "", "two commits");

    // Onto the upstream the branch tracks: it brings 80 new lines in, but nobody pulled.
    sandbox.git("mine", &["checkout", "-q", "-b", "tracking", "origin/main"]);
    sandbox.work(3);
    sandbox.teammate(2);
    sandbox.git("mine", &["fetch", "-q"]);
    assert_eq!(sandbox.played("mine", &["rebase", "-q", "origin/main"]), "rebase");
}

#[test]
fn amending_plays_only_when_the_commit_was_already_pushed() {
    let sandbox = Sandbox::new();
    sandbox.work(1);
    assert_eq!(
        sandbox.played("mine", &["commit", "-q", "--amend", "-m", "reworded"]),
        ""
    );
    sandbox.git("mine", &["push", "-q"]);
    let said = sandbox.said("mine", &["commit", "-q", "--amend", "-m", "fix: for real this time"]);
    let expected = "gitceps would play: amend (--commits 1 --msg fix: for real this time --remote server)";
    assert!(said.contains(expected), "{said}");
}

#[test]
fn a_switch_turns_its_reaction_off() {
    let sandbox = Sandbox::new();
    let set = |switch: &str, value: &str| sandbox.git("mine", &["config", &format!("gitceps.{switch}"), value]);

    set("push", "false");
    sandbox.work(1);
    assert_eq!(sandbox.played("mine", &["push"]), "", "push is off");
    set("push", "true");

    set("pull", "false");
    sandbox.teammate(3);
    assert_eq!(
        sandbox.played("mine", &["pull", "-q", "--no-rebase"]),
        "",
        "pull is off"
    );
    set("pull", "true");

    // "all" silences every reaction, but a secret is still stopped.
    set("all", "false");
    sandbox.teammate(3);
    assert_eq!(sandbox.played("mine", &["pull", "-q", "--no-rebase"]), "", "all is off");
    sandbox.write("mine/.env", "PORT=3000\n");
    sandbox.git("mine", &["add", "-A"]);
    sandbox.git("mine", &["commit", "-qm", "chore: add config"]);
    let output = sandbox.output("mine", &["push"]);
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success() && !said.contains("would play"), "{said}");
    set("all", "true");

    set("secrets", "false");
    assert_eq!(sandbox.played("mine", &["push"]), "tiny", "secrets is off");
}

#[test]
fn the_repos_own_hooks_still_run_and_can_refuse() {
    let sandbox = Sandbox::new();
    let commit = |repo: &str, message: &str| {
        let output = sandbox.output(repo, &["commit", "-q", "--allow-empty", "-m", message]);
        output.status.success()
    };

    sandbox.own_hook("mine", "commit-msg", "#!/bin/sh\ngrep -q ticket \"$1\"\n");
    assert!(!commit("mine", "no reference"));
    assert!(commit("mine", "ticket 12"));
    sandbox.git("mine", &["worktree", "add", "-q", "../tree"]);
    assert!(!commit("tree", "no reference"), "a worktree uses the repo's hooks");

    // Git runs a hook without a "#!" line with sh, and so must Gitceps.
    sandbox.own_hook("mine", "pre-push", "cat > pushed.txt; exit 1\n");
    let output = sandbox.output("mine", &["push"]);
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success() && !said.contains("would play"), "{said}");
    let pushed = std::fs::read_to_string(sandbox.path().join("mine/pushed.txt")).unwrap();
    assert!(pushed.starts_with("refs/heads/main "), "{pushed}");

    // A copy of Gitceps' own hook in the repo isn't the repo's hook: running it would start Gitceps again.
    // This one counts its rounds and gives up after a few, so a test that fails doesn't run for ever.
    let gitceps = env!("CARGO_BIN_EXE_gitceps").replace('\\', "/");
    let script = format!(
        "#!/bin/sh\n{}\n[ \"$(wc -l < rounds.txt)\" -ge 5 ] && exit 0\necho >> rounds.txt\n\
         exec '{gitceps}' hook pre-push \"$@\"\n",
        gitceps::hooks::MARK
    );
    sandbox.write("mine/rounds.txt", "");
    sandbox.own_hook("mine", "pre-push", &script);
    sandbox.commit("mine", 60, "ticket 13");
    assert_eq!(sandbox.played("mine", &["push"]), "normal");
    let rounds = std::fs::read_to_string(sandbox.path().join("mine/rounds.txt")).unwrap();
    assert_eq!(rounds.lines().count(), 0);
}

#[test]
fn the_repos_own_hooks_still_run_once_gitceps_is_gone() {
    let sandbox = Sandbox::new();
    let real = Path::new(env!("CARGO_BIN_EXE_gitceps"));
    let bin = sandbox.path().join("bin");
    let gitceps = bin.join(real.file_name().unwrap());
    std::fs::create_dir(&bin).unwrap();
    // Linked from a folder on the PATH, as a package manager does. Linux can refuse to start a copy written
    // while other tests start programs, so Windows alone gets one.
    #[cfg(unix)]
    std::os::unix::fs::symlink(real, &gitceps).unwrap();
    #[cfg(windows)]
    std::fs::copy(real, &gitceps).unwrap();
    let path = std::env::var_os("PATH").unwrap_or_default();
    let path = std::env::join_paths(std::iter::once(bin.clone()).chain(std::env::split_paths(&path))).unwrap();
    let on = sandbox
        .command(&gitceps.to_string_lossy())
        .env("PATH", path)
        .arg("on")
        .output()
        .unwrap();
    assert!(on.status.success());
    let hook = std::fs::read_to_string(sandbox.path().join(".gitceps/hooks/commit-msg")).unwrap();
    assert!(hook.contains(&gitceps.to_string_lossy().replace('\\', "/")), "{hook}");
    std::fs::remove_file(&gitceps).unwrap();

    sandbox.own_hook("mine", "commit-msg", "#!/bin/sh\ngrep -q ticket \"$1\"\n");
    let commit = |message: &str| sandbox.output("mine", &["commit", "-q", "--allow-empty", "-m", message]);
    assert!(!commit("no reference").status.success());
    assert!(commit("ticket 12").status.success());
}

/// Git skips its own update of the files when a hook named push-to-checkout exists, so Gitceps must not add one.
#[test]
fn a_push_into_a_checked_out_repo_still_updates_its_files() {
    let sandbox = Sandbox::new();
    sandbox.git(".", &["clone", "-q", "server.git", "live"]);
    sandbox.git("live", &["config", "receive.denyCurrentBranch", "updateInstead"]);
    sandbox.git(".", &["clone", "-q", "live", "deployer"]);
    sandbox.write("deployer/release.txt", "v2\n");
    sandbox.git("deployer", &["add", "-A"]);
    sandbox.git("deployer", &["commit", "-qm", "release"]);
    sandbox.git("deployer", &["push", "-q"]);
    assert!(sandbox.path().join("live/release.txt").exists());
}

#[test]
fn on_and_off_set_gits_hooks_folder_and_give_agents_the_tour_skill() {
    let sandbox = Sandbox::new();
    let hooks = sandbox.path().join(".gitceps/hooks");
    let skill = sandbox.path().join(".claude/skills/gitceps-tour/SKILL.md");
    std::fs::create_dir(sandbox.path().join(".claude")).unwrap();

    let (_, said) = sandbox.gitceps(".", &["on"]);
    assert!(
        said.ends_with("Gitceps is on for every repo. Try a git push.\n"),
        "{said}"
    );
    assert_eq!(sandbox.gitceps(".", &["status"]).1, "on\n");
    assert_eq!(std::fs::read_dir(&hooks).unwrap().count(), 23);
    // The skill names this very program, and only agents that are installed get it.
    let gitceps = env!("CARGO_BIN_EXE_gitceps").replace('\\', "/");
    let written = std::fs::read_to_string(&skill).unwrap();
    assert!(written.contains(&format!("run `\"{gitceps}\" status`")) && !written.contains("{gitceps}"));
    assert!(!sandbox.path().join(".codex").exists());

    // Finder leaves a file of its own in any folder that was opened.
    std::fs::write(skill.with_file_name(".DS_Store"), "").unwrap();
    let (_, said) = sandbox.gitceps(".", &["off"]);
    assert!(said.ends_with("Gitceps is off.\n"), "{said}");
    assert_eq!(sandbox.gitceps(".", &["status"]).1, "off\n");
    assert!(!skill.exists());
    assert_eq!(sandbox.gitceps(".", &["off"]).1, "Gitceps is already off.\n");
}

#[test]
fn someone_elses_hooks_folder_and_tour_skill_are_left_alone() {
    let sandbox = Sandbox::new();
    sandbox.git(".", &["config", "--global", "core.hooksPath", "/their/hooks"]);
    sandbox.write(".claude/skills/gitceps-tour/SKILL.md", "theirs\n");
    sandbox.write(".claude/skills/gitceps-tour/notes.md", "theirs\n");

    for switch in ["on", "off"] {
        let (done, said) = sandbox.gitceps(".", &[switch]);
        assert!(!done && said.contains("core.hooksPath is"), "{switch}: {said}");
    }
    let hooks = sandbox.output(".", &["config", "--global", "core.hooksPath"]).stdout;
    assert_eq!(String::from_utf8_lossy(&hooks), "/their/hooks\n");
    let skill = sandbox.path().join(".claude/skills/gitceps-tour/SKILL.md");
    assert_eq!(std::fs::read_to_string(skill).unwrap(), "theirs\n");
}

/// Version 0.5 kept its hooks as links to one script, and its skill as a link into its own folder.
#[cfg(unix)]
#[test]
fn turning_on_replaces_what_version_0_5_left() {
    use std::os::unix::fs::symlink;
    let sandbox = Sandbox::new();
    let hooks = sandbox.path().join(".gitceps/hooks");
    std::fs::remove_dir_all(&hooks).unwrap();
    sandbox.write(".gitceps/hooks/forward", "#!/bin/bash\n");
    sandbox.write(".gitceps/hooks/lib.sh", "");
    symlink("forward", hooks.join("pre-commit")).unwrap();
    sandbox.write(".gitceps/skills/gitceps-tour/SKILL.md", "old\n");
    std::fs::create_dir_all(sandbox.path().join(".claude/skills")).unwrap();
    let skill = sandbox.path().join(".claude/skills/gitceps-tour");
    symlink(sandbox.path().join(".gitceps/skills/gitceps-tour"), &skill).unwrap();

    let (on, said) = sandbox.gitceps(".", &["on"]);
    assert!(on, "{said}");
    assert!(!hooks.join("lib.sh").exists() && !hooks.join("pre-commit").is_symlink());
    assert!(!skill.is_symlink() && skill.join("SKILL.md").exists());
    sandbox.commit("mine", 60, "feat: add dark mode");
    assert_eq!(sandbox.played("mine", &["push"]), "normal");

    // Under Homebrew, version 0.5 was on through its own folder, which an upgrade removes.
    let homebrew = sandbox.path().join("homebrew/opt/gitceps/libexec/hooks");
    sandbox.git(
        ".",
        &["config", "--global", "core.hooksPath", &homebrew.to_string_lossy()],
    );
    assert_eq!(sandbox.gitceps(".", &["status"]).1, "off\n", "that folder is gone");
    let (on, said) = sandbox.gitceps(".", &["on"]);
    assert!(on, "{said}");
    let hooks_path = sandbox.output(".", &["config", "--global", "core.hooksPath"]).stdout;
    assert_eq!(Path::new(String::from_utf8_lossy(&hooks_path).trim()), hooks);
}

#[test]
fn a_switch_is_set_for_every_repo_or_for_one() {
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.gitceps(".", &["disable", "pull"]).1, "pull: off everywhere\n");
    assert_eq!(
        sandbox.gitceps("mine", &["enable", "pull", "--here"]).1,
        "pull: on in this repo\n"
    );
    let listed = |repo: &str| {
        let list = sandbox.gitceps(repo, &["list"]).1;
        list.lines().find(|line| line.starts_with("pull")).unwrap().to_string()
    };
    assert_eq!(listed("team"), "pull     off");
    assert_eq!(listed("mine"), "pull     on  (set in this repo)");

    let (done, said) = sandbox.gitceps(".", &["disable", "push", "--here"]);
    assert!(!done && said.contains("--here only works inside a git repo."), "{said}");
}
