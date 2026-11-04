# Shared by the hooks in this folder, which load it with `. lib.sh`. Git never runs it on its own,
# because "lib.sh" isn't a hook name.
GITCEPS_HOME="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="$HOME/Library/Logs/gitceps.log"
ZERO=0000000000000000000000000000000000000000
GIT_COMMON="$(git rev-parse --git-common-dir 2>/dev/null)"   # shared by all worktrees of a repo

# Is a switch on? Switches live in git config (see `gitceps list`), and all are on by default.
enabled() { [ "$(git config --type=bool --default true --get "gitceps.$1")" = true ]; }

# Runs the repo's own hook of the same name, if it has one, and returns its exit status.
# With Gitceps on, git looks for hooks here instead of in the repo, so this keeps them working.
run_own() {
  local own="$GIT_COMMON/hooks/$(basename "$0")"
  if [ -x "$own" ] && ! [ "$own" -ef "$0" ]; then "$own" "$@"; fi
}

# The repo's name as the cloud shows it, from a remote URL or origin's:
# git@github.com:you/app.git -> app
repo_name() {
  local url=${1:-$(git remote get-url origin 2>/dev/null)}
  url=${url%/}
  basename "${url:-origin}" .git
}

# Starts the animation in the background, so git never waits for it.
# GITCEPS_DRY=1 prints what would play instead (for testing without the overlay).
play() {
  local mode=$1; shift
  if [ -n "$GITCEPS_DRY" ]; then echo "gitceps would play: $mode ($*)" >&2
  else ( "$GITCEPS_HOME/app/gitceps-app" --mode "$mode" "$@" >>"$LOG" 2>&1 & )
  fi
}

# The pull reaction, for the commits between BASE and TIP. Returns 1 when that isn't a pull
# (nothing came in, or some of it only exists locally) so the caller can react another way.
# A pull of under 50 lines counts as a pull but stays quiet.
pull_reaction() {
  local range="$1..$2" ins del
  [ -n "$(git rev-list --no-merges "$range" | head -1)" ] || return 1
  [ -z "$(git rev-list --no-merges "$range" --not --remotes | head -1)" ] || return 1
  enabled all && enabled pull || return 0
  read -r ins del < <(git diff --numstat "$1...$2" | awk '$1 ~ /^[0-9]+$/ { i += $1; d += $2 } END { print i + 0, d + 0 }')
  [ $((ins + del)) -ge 50 ] || return 0
  play pull --ins "$ins" --del "$del" \
    --commits "$(git rev-list --no-merges --count "$range")" \
    --msg "$(git log --no-merges -1 --format=%s "$range")" \
    --msgs "$(git log --no-merges -3 --format=%s "$range")" \
    --from "$(git log --no-merges --format=%an "$range" | sort | uniq -c | sort -rn | head -1 | sed -E 's/^ *[0-9]+ //')" \
    --remote "$(repo_name)"
}
