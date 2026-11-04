<div align="center">

# Push Bro

**A gym bro who lifts your code every time you `git push`**

A small macOS app and git hook. He pops up in the corner of your screen, reacts to each push, and blocks pushes that contain passwords or API keys.

<img src=".github/assets/demo.gif" alt="A commit called 'fix: small typo' with 3,214 lines. He reads it, the camera zooms in on his face, he shouts SMALL?!, lifts it into the cloud and collapses." width="720">

</div>

## Install

macOS only. Requires Apple's Command Line Tools (`xcode-select --install`).

```sh
curl -fsSL https://raw.githubusercontent.com/ArturVah/push-bro/main/install.sh | bash
```

Or with Homebrew:

```sh
brew install ArturVah/tap/push-bro && push-bro on
```

Both build the app on your Mac and turn Push Bro on for every repo. Each repo's own hooks keep running. Repos that set their own hooks folder, such as those using Husky, skip him.

## Quickstart

Push from any repo, in the terminal or your editor:

```sh
git push
```

He appears in the bottom-right corner of the screen your mouse is on, for a few seconds. The push never waits for him.

## Reactions

| Push | Reaction |
|---|---|
| 1 to 10 lines | Lifts it, bored: "this? seriously?" |
| A normal push | "light weight, baby!" |
| 400+ lines | Sweats, shakes, gets it up, then falls over. |
| 10,000+ lines | Gives up and comes back driving a forklift. |
| A big push whose message says "small", "typo" or "quick" | Reads the message out loud, then the camera zooms in: "SMALL?!" |
| `git push --force` | Screams while the screen shakes. |
| A push git refuses because someone pushed first | The bar bounces off the cloud and lands on his foot. |
| A password or API key | Says "NOPE", throws your code away and stops the push. |

## Settings

```sh
push-bro list                   # each switch and whether it's on
push-bro disable push           # no animation when you push, in every repo
push-bro disable all --here     # no animation in the repo you're in
push-bro disable secrets        # skip the secret check
push-bro off                    # turn Push Bro off completely
```

`enable` turns a switch back on, and `on` brings him back. `all` mutes the animation but keeps the secret check. Settings are stored in git config under `[pushbro]`, and `--here` writes to the current repo's `.git/config`. With the curl install, the command is `~/.push-bro/bin/push-bro`.

To uninstall, run `push-bro off` first, then `rm -rf ~/.push-bro` or `brew uninstall push-bro`. Skipping `off` leaves git looking for hooks in a folder that no longer exists, and your repos' own hooks stop running.

## Secret check

Before every push, the hook scans the new lines for GitHub, Stripe, AWS, Slack, Anthropic and OpenAI keys, private keys, and quoted values like `password = "..."`. It also flags added `.env` files (templates such as `.env.example` are fine), `id_rsa`, `id_ed25519`, `credentials.json` and `*.pem`. On a match, the push stops.

If the match is a fake test value, push anyway with `git push --no-verify`. The check catches common mistakes; use a dedicated scanner such as [gitleaks](https://github.com/gitleaks/gitleaks) for full coverage.

## Notes

- **No sound yet:** he grunts in text only.
- **Homebrew and Xcode:** Homebrew won't build from source while an installed Xcode is older than your macOS. Update Xcode, or use the curl installer.
- **Browser preview:** clone the repo and open `preview.html` to watch every reaction without installing anything.
- **Logs:** the app writes to `~/Library/Logs/push-bro.log`.

## License

MIT
