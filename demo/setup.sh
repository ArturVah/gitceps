#!/bin/bash
# Builds the app and creates a sandbox repo whose "server" is a folder on disk.
# The hook is turned on for the sandbox only; nothing outside this project changes.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"

xcrun swiftc -O -swift-version 6 "$root/app/main.swift" -o "$root/app/pushbro" -framework AppKit -framework WebKit

rm -rf "$root/demo/repo" "$root/demo/demo-app.git"
git init -q --bare -b main "$root/demo/demo-app.git"
git init -q -b main "$root/demo/repo"
cd "$root/demo/repo"
git config user.name "Push Bro Demo"
git config user.email "demo@example.com"
git config core.hooksPath "$root/hooks"
git remote add origin "$root/demo/demo-app.git"
echo "# Push Bro demo" > README.md
git add README.md
git commit -qm "first commit"
git push -q --no-verify -u origin main
echo "Ready. Now run: demo/try.sh huge"
