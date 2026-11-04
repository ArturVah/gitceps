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
        └── app/gitceps-app      Mac app (app/main.swift): a see-through, click-through window
            └── overlay/overlay.html the drawing (SVG)
                ├── engine.js    moves and draws it, one clock for everything
                └── scenes.js    what he does for each reaction
```

`bin/gitceps on` points git's global `core.hooksPath` at `hooks/`. The other hook names in that folder are symlinks to `hooks/forward`. A pull is whatever git itself labels a pull (`git pull`, merge or rebase), or a merge of the branch's upstream, as an IDE's fetch-then-merge does. `install.sh` downloads a tagged release into `~/.gitceps`, builds the app there and runs `gitceps on`. The Homebrew formula lives in [ArturVah/homebrew-tap](https://github.com/ArturVah/homebrew-tap).

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

## Release

1. Bump `version` in `install.sh` and commit.
2. Tag and push: `git tag v0.2.0 && git push origin v0.2.0`.
3. In `homebrew-tap`, update `url` and `sha256` in `Formula/gitceps.rb`. The checksum comes from `curl -sL https://github.com/ArturVah/gitceps/archive/refs/tags/v0.2.0.tar.gz | shasum -a 256`.
4. Wait about five minutes before announcing: `raw.githubusercontent.com` keeps serving the old `install.sh` until its cache expires.
