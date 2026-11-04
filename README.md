<div align="center">

# Push Bro

**A gym bro who lifts your code every time you `git push`**

A small macOS app and git hook. He pops up in the corner of your screen, reacts to each push, and blocks pushes that contain passwords or API keys.

<img src=".github/assets/demo.gif" alt="A commit called 'fix: small typo' with 3,214 lines. He reads it, the camera zooms in on his face, he shouts SMALL?!, lifts it into the cloud and collapses." width="720">

</div>

## Install

macOS only. Requires Apple's Command Line Tools (`xcode-select --install`).

```sh
git clone https://github.com/ArturVah/push-bro.git ~/push-bro
~/push-bro/demo/setup.sh
```

`setup.sh` builds the app and creates a sandbox repo with the hook turned on. It leaves your own repos alone.

## Quickstart

Make a 3,000-line commit in the sandbox and push it:

```sh
~/push-bro/demo/try.sh huge
```

He appears in the bottom-right corner of the screen your mouse is on. The sandbox pushes to a folder on your disk, so nothing leaves your machine. Swap `huge` for `tiny`, `normal`, `forklift`, `liar`, `force`, `rejected` or `secret`.

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

## Use it in a real repo

Run this inside the repo:

```sh
git config core.hooksPath ~/push-bro/hooks
```

To turn him off again: `git config --unset core.hooksPath`. While it's set, git runs Push Bro's hook instead of that repo's own hooks, so skip repos that rely on hooks of their own (for example, Husky).

## Secret check

Before every push, the hook scans the new lines for GitHub, Stripe, AWS, Slack, Anthropic and OpenAI keys, private keys, and quoted values like `password = "..."`. It also flags added `.env` files (templates such as `.env.example` are fine), `id_rsa`, `id_ed25519`, `credentials.json` and `*.pem`. On a match, the push stops.

If the match is a fake test value, push anyway with `git push --no-verify`. The check catches common mistakes; use a dedicated scanner such as [gitleaks](https://github.com/gitleaks/gitleaks) for full coverage.

## Notes

- **No sound yet:** he grunts in text only.
- **Browser preview:** open `preview.html` to watch every reaction without installing anything.
- **Logs:** the app writes to `~/Library/Logs/push-bro.log`.

## License

MIT
