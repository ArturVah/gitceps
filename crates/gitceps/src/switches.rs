//! Turning Gitceps on and off. He is on for every repo while git's global core.hooksPath points at his
//! folder of hooks. Single reactions are switched in git config, as gitceps.<switch>.
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::ValueEnum;

use crate::git::{ask, succeeds};
use crate::{hooks, paths};

/// A reaction, or the secret check, that can be turned off.
#[derive(Clone, Copy, ValueEnum)]
pub enum Switch {
    Push,
    Pull,
    Amend,
    Rebase,
    Clone,
    /// Every animation
    All,
    /// The secret check, which "all" leaves alone
    Secrets,
}

impl Switch {
    fn name(self) -> String {
        let name = self.to_possible_value().expect("every switch can be named");
        name.get_name().to_string()
    }

    fn key(self) -> String {
        format!("gitceps.{}", self.name())
    }

    /// Every switch is on until someone turns it off.
    pub fn on(self) -> bool {
        ask(&["config", "--type=bool", "--default", "true", "--get", &self.key()]) == "true"
    }
}

pub fn on() -> Result<()> {
    let gitceps = shell_path(&program()?);
    add_skill(&gitceps)?;
    let hooks = paths::hooks()?;
    // Written either way, so a repo can be pointed at them when the global setting belongs to someone else.
    write_hooks(&hooks, &gitceps).with_context(|| format!("can't write the hooks in {}", hooks.display()))?;
    let current = global_hooks_path();
    if !current.is_empty() && !is_gitceps(&current, &hooks) {
        bail!(
            "Your global core.hooksPath is already {current}, so Gitceps left it alone.\n\
             To use him in one repo only, run this inside it: git config core.hooksPath \"{}\"",
            hooks.display()
        );
    }
    if !succeeds(&["config", "--global", "core.hooksPath", &hooks.to_string_lossy()]) {
        bail!("can't change your global git config");
    }
    println!("Gitceps is on for every repo. Try a git push.");
    Ok(())
}

pub fn off() -> Result<()> {
    remove_skill()?;
    let hooks = paths::hooks()?;
    let current = global_hooks_path();
    if current.is_empty() {
        println!("Gitceps is already off.");
    } else if is_gitceps(&current, &hooks) {
        if !succeeds(&["config", "--global", "--unset", "core.hooksPath"]) {
            bail!("can't change your global git config");
        }
        println!("Gitceps is off.");
    } else {
        bail!(
            "Your global core.hooksPath is {current}, not Gitceps' ({}), so it was left alone.",
            hooks.display()
        );
    }
    let local = ask(&["config", "--local", "--type=path", "core.hooksPath"]);
    if is_gitceps(&local, &hooks) {
        println!("This repo still turns him on itself. To stop that too: git config --unset core.hooksPath");
    }
    Ok(())
}

pub fn status() -> Result<()> {
    let current = global_hooks_path();
    let on = is_gitceps(&current, &paths::hooks()?) && Path::new(&current).is_dir();
    println!("{}", if on { "on" } else { "off" });
    Ok(())
}

/// Each switch, and whether it's on in the folder this was run from.
pub fn list() {
    for switch in Switch::value_variants() {
        let state = if switch.on() { "on" } else { "off" };
        let here = succeeds(&["config", "--local", "--get", &switch.key()]);
        println!(
            "{:<8} {state}{}",
            switch.name(),
            if here { "  (set in this repo)" } else { "" }
        );
    }
}

/// Turns a switch on or off: for every repo, or with `here` only for the one this was run in.
pub fn set(switch: Switch, on: bool, here: bool) -> Result<()> {
    if here && !succeeds(&["rev-parse", "--git-dir"]) {
        bail!("--here only works inside a git repo.");
    }
    let scope = if here { "--local" } else { "--global" };
    if !succeeds(&["config", scope, &switch.key(), if on { "true" } else { "false" }]) {
        bail!("can't change your git config");
    }
    let state = if on { "on" } else { "off" };
    println!(
        "{}: {state} {}",
        switch.name(),
        if here { "in this repo" } else { "everywhere" }
    );
    Ok(())
}

/// The global core.hooksPath with a leading ~ filled in, or nothing when it isn't set.
fn global_hooks_path() -> String {
    ask(&["config", "--global", "--type=path", "core.hooksPath"])
}

/// Whether a hooks path is Gitceps': this version's folder, or the folder of a version 0.5 that was run from
/// a checkout (gitceps/hooks) or installed by Homebrew (gitceps/libexec/hooks).
fn is_gitceps(path: &str, hooks: &Path) -> bool {
    let old = ["gitceps/hooks", "gitceps/libexec/hooks"];
    same_folder(path, hooks) || old.iter().any(|old| Path::new(path).ends_with(old))
}

/// Paths are compared as the folders they lead to, so a link or a trailing slash makes no difference.
fn same_folder(path: &str, folder: &Path) -> bool {
    let path = Path::new(path);
    match (path.canonicalize(), folder.canonicalize()) {
        (Ok(path), Ok(folder)) => path == folder,
        _ => path == folder,
    }
}

/// Where this program is, as the shell finds it. A package manager's link on the PATH, such as Homebrew's
/// bin/gitceps, outlives upgrades, while the file it leads to sits in a folder named after the version.
fn program() -> Result<PathBuf> {
    let this = std::env::current_exe().context("can't find where gitceps is")?;
    let Ok(real) = this.canonicalize() else { return Ok(this) };
    let path = std::env::var_os("PATH").unwrap_or_default();
    let on_path = std::env::split_paths(&path)
        .filter(|folder| folder.is_absolute())
        .map(|folder| folder.join(this.file_name().unwrap_or_default()))
        .find(|candidate| candidate.canonicalize().is_ok_and(|candidate| candidate == real));
    Ok(on_path.unwrap_or(this))
}

/// A path as a shell script and an agent's shell take it on every system: Windows accepts forward slashes
/// too, and a backslash would need escaping.
fn shell_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Writes one script per hook name. Git runs them with its own shell on every system, Windows included.
/// If Gitceps was removed without `gitceps off`, each runs the repo's own hook as git would without him.
fn write_hooks(folder: &Path, gitceps: &str) -> std::io::Result<()> {
    // Emptied first: version 0.5 kept links here, and writing to one would land in the file it points at.
    if folder.exists() {
        std::fs::remove_dir_all(folder)?;
    }
    std::fs::create_dir_all(folder)?;
    let gitceps = format!("'{}'", gitceps.replace('\'', r"'\''"));
    for name in hooks::NAMES {
        let hook = folder.join(name);
        let script = format!(
            "#!/bin/sh\n\
             {}\n\
             gitceps={gitceps}\n\
             [ -x \"$gitceps\" ] && exec \"$gitceps\" hook {name} \"$@\"\n\
             own=\"$(git rev-parse --git-common-dir)/hooks/{name}\"\n\
             [ -x \"$own\" ] && ! [ \"$own\" -ef \"$0\" ] && exec \"$own\" \"$@\"\n\
             exit 0\n",
            hooks::MARK
        );
        std::fs::write(&hook, script)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))?;
        }
    }
    Ok(())
}

/// What tells a coding agent how to write a tour. "{gitceps}" stands for the path of this program.
const SKILL: &str = include_str!("../assets/SKILL.md");

/// Where the tour skill goes in each coding agent's folder: Claude Code's and Codex's, and a second profile
/// the shell names, such as a work account in CLAUDE_CONFIG_DIR.
fn skill_folders() -> Result<Vec<PathBuf>> {
    let home = paths::home()?;
    let mut agents = vec![home.join(".claude"), home.join(".codex")];
    for profile in ["CLAUDE_CONFIG_DIR", "CODEX_HOME"] {
        let profile = std::env::var_os(profile).map(PathBuf::from);
        if let Some(profile) = profile.filter(|profile| !profile.as_os_str().is_empty() && !agents.contains(profile)) {
            agents.push(profile);
        }
    }
    Ok(agents
        .into_iter()
        .map(|agent| agent.join("skills").join("gitceps-tour"))
        .collect())
}

/// Whether a skill folder is one Gitceps put there: the link version 0.5 made into its own skills folder, or a
/// folder with nothing but the skill in it, not counting hidden files such as Finder's .DS_Store. Anything
/// else is the user's own.
fn is_ours(skill: &Path) -> bool {
    let visible = || {
        let names = skill.read_dir().ok()?.flatten().map(|file| file.file_name());
        Some(
            names
                .filter(|name| !name.to_string_lossy().starts_with('.'))
                .collect::<Vec<_>>(),
        )
    };
    let old_link = skill
        .read_link()
        .is_ok_and(|target| target.ends_with("skills/gitceps-tour"));
    old_link || (!skill.is_symlink() && visible().is_some_and(|files| files == ["SKILL.md"]))
}

/// Tours don't need the hooks, so the skill comes and goes with on and off whatever core.hooksPath says.
fn add_skill(gitceps: &str) -> Result<()> {
    for skill in skill_folders()? {
        let skills = skill.parent().expect("a skill sits in the skills folder");
        if !skills.parent().is_some_and(Path::is_dir) {
            continue; // that agent isn't on this computer
        }
        if skill.symlink_metadata().is_ok() && !is_ours(&skill) {
            eprintln!(
                "{} already exists, so the tour skill wasn't added there.",
                skill.display()
            );
            continue;
        }
        let write = || -> std::io::Result<()> {
            if skill.symlink_metadata().is_ok() {
                std::fs::remove_dir_all(&skill)?; // a link is removed without touching what it points at
            }
            std::fs::create_dir_all(&skill)?;
            std::fs::write(skill.join("SKILL.md"), SKILL.replace("{gitceps}", gitceps))
        };
        // Tours are extra: the hooks still go on without them.
        match write() {
            Ok(()) => println!("Tour skill added to {}.", skills.display()),
            Err(error) => eprintln!("Can't write the tour skill in {}: {error}", skill.display()),
        }
    }
    Ok(())
}

fn remove_skill() -> Result<()> {
    for skill in skill_folders()? {
        if !is_ours(&skill) {
            continue;
        }
        match std::fs::remove_dir_all(&skill) {
            Ok(()) => println!(
                "Tour skill removed from {}.",
                skill.parent().unwrap_or(&skill).display()
            ),
            Err(error) => eprintln!("Can't remove {}: {error}", skill.display()),
        }
    }
    Ok(())
}
