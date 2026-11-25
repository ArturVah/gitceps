# Contributing

## How it fits together

Two programs, built from one Cargo workspace:

```
git push / pull / commit --amend / rebase / clone
└── ~/.gitceps/hooks/NAME            a small script per hook name, written by `gitceps on`
    └── gitceps hook NAME            crates/gitceps/src/hooks/: runs the repo's own hook first, then
        ├── push.rs                  measures the push, picks a reaction, blocks secrets (secrets.rs)
        ├── pull.rs                  pull (merge)
        ├── rewrite.rs               amend, rebase, pull --rebase
        ├── checkout.rs              clone
        └── window.rs                starts the window program and never waits for it
            └── gitceps-window play  crates/gitceps-window/src/overlay.rs: a see-through, click-through window
                └── pages/overlay/overlay.html   the drawing (SVG)
                    ├── engine.js    moves and draws it, one clock for everything
                    └── scenes.js    what he does for each reaction

"tour PR #18" in Claude Code or Codex
└── SKILL.md                         crates/gitceps/assets/: how the agent reviews the change and writes the tour file
    └── gitceps tour FILE            crates/gitceps/src/tour/: checks the tour against the diff (check.rs, reading
        │                            diff.rs and git.rs) and builds what the page shows (page.rs)
        └── gitceps-window show      crates/gitceps-window/src/tour.rs: the floating panel
            ├── pages/tour/tour.html steps in the sidebar, code in the main area; tour.js runs it
            └── pages/overlay/overlay.html?stage=tour   Push Bro in a see-through window beside the panel, playing REVIEW from scenes.js
```

The hooks and the command are one program and the windows another, so a git hook never loads a window library. Where the windows can't run, the hooks and the secret check still do.

`gitceps on` writes the hook scripts and points git's global `core.hooksPath` at them (`switches.rs`). Each script runs `gitceps hook NAME`, and if Gitceps has been removed it runs the repo's own hook instead. A pull is whatever git itself labels a pull (`git pull`, merge or rebase), or a merge of the branch's upstream, as an IDE's fetch-then-merge does.

`gitceps on` also writes the tour skill into `~/.claude/skills` and `~/.codex/skills` when those apps are installed (and into the profile `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names), with the path of the `gitceps` that wrote it, and `gitceps off` removes it. A tour file names line ranges, never code: Gitceps reads the lines from git, so the window always shows what's really in the commit.

The pages are built into `gitceps-window` (`pages.rs`) and talk to it through `window.ipc.postMessage`. `preview.html` plays the same overlay files in a browser, next to a fake terminal. The scripts are plain scripts rather than ES modules, so the pages also work when opened straight from disk. What macOS, Windows and Linux each do their own way is in `crates/gitceps-window/src/platform/`.

## Change a reaction

Each reaction is a list of steps in `crates/gitceps-window/pages/overlay/scenes.js`. The step format is described at the top of that file. To watch one scene, open it in a browser:

```
crates/gitceps-window/pages/overlay/overlay.html?mode=huge&ins=3214&del=0&msg=fix:%20small%20typo
```

## Add a new kind of push

1. Add a scene to `SCENES` in `scenes.js`.
2. Choose when it plays in `crates/gitceps/src/hooks/push.rs`, where `mode` is set.
3. Add it to `PUSHES` in `preview.html`, as a case in `demo/try.sh`, and to the test of push sizes in `crates/gitceps/tests/hooks.rs`.

A reaction to another git command gets its own hook, like `pull.rs`, `rewrite.rs` and `checkout.rs`: a function that runs the repo's own hook first, then checks its switch. Add the switch to `Switch` in `switches.rs`.

## Build and test

Rust 1.90 or newer. On Linux the windows also need `libwebkit2gtk-4.1-dev` and `libgtk-3-dev`.

```sh
cargo test                    # the hooks and the tour check, run by real git commands on throwaway repos
cargo clippy --all-targets -- -D warnings
cargo fmt

demo/setup.sh                 # builds both programs and creates a sandbox repo with the hooks on
demo/try.sh forklift          # pushes a commit of that kind from the sandbox
GITCEPS_DRY=1 demo/try.sh liar     # prints the chosen reaction instead of showing it
```

CI runs the same three Cargo commands on macOS, Linux and Windows.

To use a build of your own, run `cargo build --release`, keep `target/release/gitceps` and `gitceps-window` in one folder, and run `gitceps on` from there.

## Try a tour

`demo/tour.json` is a tour of this repo's own `v0.5.2..v0.5.3`, so it needs a clone with tags:

```sh
demo/setup.sh                              # builds both programs
target/release/gitceps tour demo/tour.json # checks it, then opens it
```

`GITCEPS_DRY=1 target/release/gitceps tour demo/tour.json` prints what the window would get. To change what Push Bro does on a step, edit `REVIEW` in `scenes.js`, and keep the list of names in `crates/gitceps/src/tour/check.rs` and the table in `SKILL.md` the same.

## Release

1. Bump `version` in `Cargo.toml`, run `cargo build` so `Cargo.lock` follows, and commit. Don't push `main` yet when `install.sh` or `install.ps1` changed: the install commands run the copy on `main`, and it must not ask for files a release doesn't have yet.
2. Tag and push the tag: `git tag v0.6.0 && git push origin v0.6.0`.
3. Publish the release: `gh release create v0.6.0 --title "Gitceps 0.6.0" --notes "..."`. The Release workflow then builds both programs for every system and attaches the archives with their checksums. Wait for it: `gh run watch`.
4. Push `main`.
5. In [ArturVah/homebrew-tap](https://github.com/ArturVah/homebrew-tap), update the `url` and the `sha256` of the macOS archive in `Formula/gitceps.rb` from the release's `SHA256SUMS`.

To try the archives before anyone gets them, publish with `--prerelease`, which leaves `releases/latest` where it was, and install from that release: `GITCEPS_RELEASE=https://github.com/ArturVah/gitceps/releases/download/v0.6.0 bash install.sh`. Then unmark it: `gh release edit v0.6.0 --prerelease=false --latest`.

The archives carry no version in their names, so `install.sh` and `install.ps1` always fetch `releases/latest/download/…` and never need editing for a release. GitHub counts downloads of attached files, so this shows installs per release (reinstalls and upgrades included):

```sh
gh api repos/ArturVah/gitceps/releases -q '.[] | "\(.tag_name) \([.assets[].download_count] | add // 0)"'
```
