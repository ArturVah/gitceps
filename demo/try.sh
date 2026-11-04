#!/bin/bash
# Makes a commit of the chosen kind in the sandbox repo (see setup.sh) and pushes it.
# Usage: demo/try.sh tiny | normal | huge | forklift | liar | force | rejected | secret
cd "$(dirname "$0")/repo" || { echo "Run demo/setup.sh first."; exit 1; }

# Writes N lines of fake code to a file (different every run, so the diff is always N+ lines).
fake_code() { awk -v n="$1" -v r="$RANDOM" 'BEGIN { for (i = 1; i <= n; i++) print "const thing" i " = doSomething(" i ", " r ");" }' > "$2"; }

case "$1" in
  tiny)
    echo "small fix $RANDOM" >> README.md
    git commit -qam "fix: typo in README" ;;
  normal)
    fake_code 60 feature.js
    git add feature.js && git commit -qm "feat: add dark mode" ;;
  huge)
    fake_code 3214 big.js
    git add big.js && git commit -qm "refactor: rewrite everything" ;;
  forklift)
    fake_code 12000 bigger.js
    git add bigger.js && git commit -qm "chore: vendor the whole internet" ;;
  liar)
    fake_code 3214 typo.js
    git add typo.js && git commit -qm "fix: small typo" ;;
  force)
    echo "oops $RANDOM" >> README.md
    git commit -q --amend -am "WIP do not merge"
    git push --force
    exit ;;
  rejected)
    # A "teammate" pushes first from a throwaway clone, so the server has a commit we don't.
    teammate=$(mktemp -d)
    git clone -q ../demo-app.git "$teammate"
    echo "teammate $RANDOM" >> "$teammate/teammate.txt"
    git -C "$teammate" add teammate.txt
    git -C "$teammate" -c user.name=Teammate -c user.email=teammate@example.com commit -qm "teammate's change"
    git -C "$teammate" push -q --no-verify   # the teammate's push shouldn't show Push Bro
    rm -rf "$teammate"
    echo "my change $RANDOM" >> README.md
    git commit -qam "feat: my change"
    git push
    git pull -q --rebase && git push -q --no-verify   # catch up quietly so the next demo starts clean
    exit ;;
  secret)
    # Fake key, assembled at runtime so no key-shaped string lives in this repo.
    printf 'STRIPE_KEY=%s_%s_%s\n' sk live 51FAKEfakeFAKE0000demo > .env
    git add .env && git commit -qm "chore: add config"
    git push
    git reset -q --hard HEAD~1   # drop the fake-secret commit so the demo keeps working
    exit ;;
  *)
    echo "usage: demo/try.sh tiny|normal|huge|forklift|liar|force|rejected|secret"; exit 1 ;;
esac
git push
