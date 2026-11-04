# Contributing

## How it fits together

```
git push
└── hooks/pre-push           measures the push and picks a reaction; blocks secrets
    └── app/pushbro          Mac app (app/main.swift): a see-through, click-through window
        └── overlay/overlay.html   the drawing (SVG)
            ├── engine.js          moves and draws it, one clock for everything
            └── scenes.js          what he does for each kind of push
```

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

## Build and test

```sh
demo/setup.sh                 # builds app/pushbro with Swift 6 and creates the sandbox repo
demo/try.sh forklift          # pushes a commit of that kind from the sandbox
PB_DRY=1 demo/try.sh liar     # prints the chosen reaction instead of showing it
```

The app must build with no warnings under `-swift-version 6`.
