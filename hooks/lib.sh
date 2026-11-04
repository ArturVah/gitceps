# Shared by the hooks in this folder, which load it with `. lib.sh`. Git never runs it on its own,
# because "lib.sh" isn't a hook name.
LOG="$HOME/Library/Logs/gitceps.log"
ZERO=0000000000000000000000000000000000000000

# Where the repo keeps its own hooks. Git starts hooks in the repo's top folder, so in a normal
# checkout that's ./.git; worktrees, submodules and bare repos need git to say. Empty outside a
# repo, which happens when bin/gitceps loads this file.
if [ -z "${GIT_DIR:-}" ] && [ -d .git ]; then GIT_COMMON=.git
else GIT_COMMON="$(git rev-parse --git-common-dir 2>/dev/null || true)"
fi

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
  local mode=$1 home; shift
  if [ -n "$GITCEPS_DRY" ]; then echo "gitceps would play: $mode ($*)" >&2; return; fi
  home="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
  ( "$home/app/gitceps-app" --mode "$mode" "$@" >>"$LOG" 2>&1 & )
}

# The pull reaction. It's a pull when the branch now contains its upstream (e.g. origin/main) and
# the upstream brought commits that weren't here before (BASE is where the branch was). Returns 1
# when this isn't a pull, so the caller can react another way. Pulls under 50 lines stay quiet.
pull_reaction() {
  local up log ins del
  up=$(git rev-parse -q --verify '@{upstream}' 2>/dev/null) || return 1
  git merge-base --is-ancestor "$up" HEAD || return 1
  log=$(git log --no-merges --format='%an%x09%s' "$1..$up")   # author <tab> subject, newest first
  [ -n "$log" ] || return 1
  enabled all && enabled pull || return 0
  read -r ins del < <(git diff --numstat "$1...$up" | awk '$1 ~ /^[0-9]+$/ { i += $1; d += $2 } END { print i + 0, d + 0 }')
  [ $((ins + del)) -ge 50 ] || return 0
  play pull --ins "$ins" --del "$del" --commits "$(grep -c . <<<"$log")" \
    --msg "$(head -1 <<<"$log" | cut -f2)" --msgs "$(head -3 <<<"$log" | cut -f2)" \
    --from "$(cut -f1 <<<"$log" | sort | uniq -c | sort -rn | head -1 | sed -E 's/^ *[0-9]+ //')" \
    --remote "$(repo_name)"
}
