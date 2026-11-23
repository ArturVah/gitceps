# Contributing

## How it fits together

```
git push / pull / commit --amend / rebase / clone
└── hooks/                       one script per git event; each runs the repo's own hook first
    ├── pre-push                 measures the push, picks a reaction, blocks secrets
    ├── post-merge               pull (merge)
    ├── post-rewrite             amend, rebase, pull --rebase
    ├── post-checkout            clone
    ├── forward                  stands in for every other hook name, so the repo's own hooks still run
    └── lib.sh                   shared: switches, running the repo's own hook, pull detection, starting the app
        └── app/gitceps-app      Mac app (app/Overlay.swift): a see-through, click-through window
            └── overlay/overlay.html the drawing (SVG)
                ├── engine.js    moves and draws it, one clock for everything
                └── scenes.js    what he does for each reaction

"tour PR #18" in Claude Code or Codex
└── skills/gitceps-tour/SKILL.md how the agent reviews the change and writes the tour file
    └── bin/gitceps tour FILE
        └── app/gitceps-app --tour   checks the tour against the diff (TourCheck.swift, reading Diff.swift and Git.swift),
            │                        then hands it to a copy of itself started with --show (TourViewer.swift)
            ├── tour/tour.html       the floating panel's page: steps in the sidebar, code in the main area; tour.js runs it
            └── overlay/overlay.html?stage=tour   Push Bro in a see-through window beside the panel, playing REVIEW from scenes.js
```

`bin/gitceps on` points git's global `core.hooksPath` at `hooks/`. The other hook names in that folder are symlinks to `hooks/forward`. A pull is whatever git itself labels a pull (`git pull`, merge or rebase), or a merge of the branch's upstream, as an IDE's fetch-then-merge does. `install.sh` downloads a tagged release into `~/.gitceps` and builds the app there. A first install runs `gitceps on`; `gitceps update` reruns the installer, which leaves that setting alone. The Homebrew formula lives in [ArturVah/homebrew-tap](https://github.com/ArturVah/homebrew-tap).

`gitceps on` also links `skills/gitceps-tour` into `~/.claude/skills` and `~/.codex/skills` when those apps are installed (and into the profile `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names), and `gitceps off` removes the links. The skill runs `skills/gitceps-tour/gitceps`, a link to `bin/gitceps`, so it always uses the Gitceps it came with: a skill linked from a checkout runs that checkout. A tour file names line ranges, never code: the app reads the lines from git, so the window always shows what's really in the commit.

`preview.html` plays the same `overlay/` files in a browser, next to a fake terminal. There are no dependencies and no build step for the animation. The scripts are plain scripts rather than ES modules, so the pages also work when opened straight from disk.

## Change a reaction

Each reaction is a list of steps in `overlay/scenes.js`. The step format is described at the top of that file. To watch one scene, open it in a browser:

```
overlay/overlay.html?mode=huge&ins=3214&del=0&msg=fix:%20small%20typo
```

## Add a new kind of push

1. Add a scene to `SCENES` in `overlay/scenes.js`.
2. Choose when it plays in `hooks/pre-push`, where `mode` is set.
3. Add it to `PUSHES` in `preview.html` and as a case in `demo/try.sh`.

A reaction to another git command gets its own hook, like `hooks/post-merge` (pull), `hooks/post-rewrite` (amend, rebase) and `hooks/post-checkout` (clone): a script that runs the repo's own hook first, then checks its switch with `enabled <name>`. Add the switch name to `SWITCHES` in `bin/gitceps`.

## Build and test

```sh
demo/setup.sh                 # builds app/gitceps-app with Swift 6 and creates the sandbox repo
demo/try.sh forklift          # pushes a commit of that kind from the sandbox
GITCEPS_DRY=1 demo/try.sh liar     # prints the chosen reaction instead of showing it
```

The app must build with no warnings under `-swift-version 6`.

## Try a tour

`demo/tour.json` is a tour of this repo's own `v0.5.2..v0.5.3`, so it needs a clone with tags:

```sh
demo/setup.sh                    # builds the app
bin/gitceps tour demo/tour.json  # checks it, then opens it
```

`GITCEPS_DRY=1 app/gitceps-app --tour demo/tour.json` prints what the window would get. To change what Push Bro does on a step, edit `REVIEW` in `overlay/scenes.js`, and keep the list of names in `app/TourCheck.swift` and the table in `SKILL.md` the same.

## Release

1. Bump `version` in `install.sh` and commit, but don't push `main` yet. The curl command and `gitceps update` run the `install.sh` on `main`, so it must not name a release whose archive isn't attached yet.
2. Tag and push the tag: `git tag v0.2.0 && git push origin v0.2.0`.
3. Publish the release: `gh release create v0.2.0 --title "Gitceps 0.2.0" --notes "..."`. A workflow then attaches `gitceps-0.2.0.tar.gz` to it, which is what `install.sh` and Homebrew download. Wait for it: `gh run watch`.
4. Push `main`.
5. In `homebrew-tap`, update `url` and `sha256` in `Formula/gitceps.rb`, and keep its build and install lines the same as `install.sh`'s: it compiles `app/*.swift` and installs `app`, `overlay`, `tour`, `skills`, `hooks` and `bin`. Its caveats tell people to run `gitceps on` after an upgrade, since that's what adds the tour skill. The checksum comes from `curl -sL https://github.com/ArturVah/gitceps/releases/download/v0.2.0/gitceps-0.2.0.tar.gz | shasum -a 256`.
6. Wait about five minutes before announcing: `raw.githubusercontent.com` keeps serving the old `install.sh` until its cache expires.

GitHub counts downloads of attached files, so this shows installs per release (reinstalls and upgrades included):

```sh
gh api repos/ArturVah/gitceps/releases -q '.[] | "\(.tag_name) \([.assets[].download_count] | add // 0)"'
```
