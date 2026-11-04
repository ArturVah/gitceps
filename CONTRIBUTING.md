# Contributing

## How it fits together

```
git push
└── hooks/pre-push           runs the repo's own pre-push, measures the push, picks a reaction, blocks secrets
    └── app/pushbro          Mac app (app/main.swift): a see-through, click-through window
        └── overlay/overlay.html   the drawing (SVG)
            ├── engine.js          moves and draws it, one clock for everything
            └── scenes.js          what he does for each kind of push
```

`bin/push-bro on` points git's global `core.hooksPath` at `hooks/`. Every other hook name in that folder is a symlink to `hooks/forward`, which runs the repo's own hook of the same name. `install.sh` downloads a tagged release into `~/.push-bro`, builds the app there and runs `push-bro on`. The Homebrew formula lives in [ArturVah/homebrew-tap](https://github.com/ArturVah/homebrew-tap).

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

A reaction to another git command gets its own hook: replace that hook's symlink in `hooks/` with a script that runs the repo's own hook first (as `pre-push` does), then checks its switch with `enabled <name>`. Add the switch name to `SWITCHES` in `bin/push-bro`.

## Build and test

```sh
demo/setup.sh                 # builds app/pushbro with Swift 6 and creates the sandbox repo
demo/try.sh forklift          # pushes a commit of that kind from the sandbox
PB_DRY=1 demo/try.sh liar     # prints the chosen reaction instead of showing it
```

The app must build with no warnings under `-swift-version 6`.

## Release

1. Bump `version` in `install.sh` and commit.
2. Tag and push: `git tag v0.2.0 && git push origin v0.2.0`.
3. In `homebrew-tap`, update `url` and `sha256` in `Formula/push-bro.rb`. The checksum comes from `curl -sL https://github.com/ArturVah/push-bro/archive/refs/tags/v0.2.0.tar.gz | shasum -a 256`.
