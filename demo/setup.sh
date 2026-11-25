#!/bin/bash
# Builds Gitceps and creates a sandbox repo whose "server" is a folder on disk.
# The hooks are turned on for the sandbox only; nothing outside this project changes.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"

cargo build --release --manifest-path "$root/Cargo.toml"

# `gitceps on` writes the hook scripts and changes the global git config, so it gets a home folder of its own
# inside the sandbox, and the repo below is pointed at the hooks it wrote there.
home="$root/demo/home"
rm -rf "$root/demo/repo" "$root/demo/demo-app.git" "$home"
mkdir -p "$home"
env -u CLAUDE_CONFIG_DIR -u CODEX_HOME -u XDG_CONFIG_HOME HOME="$home" USERPROFILE="$home" \
  GIT_CONFIG_GLOBAL="$home/.gitconfig" "$root/target/release/gitceps" on >/dev/null

git init -q --bare -b main "$root/demo/demo-app.git"
git init -q -b main "$root/demo/repo"
cd "$root/demo/repo"
git config user.name "Gitceps Demo"
git config user.email "demo@example.com"
git config core.hooksPath "$home/.gitceps/hooks"
git remote add origin "$root/demo/demo-app.git"
echo "# Gitceps demo" > README.md
git add README.md
git commit -qm "first commit"
git push -q --no-verify -u origin main
echo "Ready. Now run: demo/try.sh huge"
