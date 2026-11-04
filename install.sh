#!/bin/bash
# Installs Push Bro into ~/.push-bro and turns it on for every git repo:
#   curl -fsSL https://raw.githubusercontent.com/ArturVah/push-bro/main/install.sh | bash
# Everything runs inside main(), so a download cut off halfway does nothing.
set -euo pipefail

main() {
  local version="0.2.0" dest="$HOME/.push-bro"

  if [ "$(uname)" != Darwin ]; then
    echo "Push Bro only runs on macOS." >&2; exit 1
  fi
  if ! xcrun --find swiftc >/dev/null 2>&1; then
    echo "Push Bro needs Apple's Command Line Tools. Install them with: xcode-select --install" >&2; exit 1
  fi
  if [ -e "$dest" ] && [ ! -x "$dest/bin/push-bro" ]; then
    echo "$dest already exists and isn't Push Bro. Move it out of the way and run this again." >&2; exit 1
  fi

  echo "Downloading Push Bro $version..."
  tmp=$(mktemp -d)   # not local: the EXIT trap runs after main() has returned
  trap 'rm -rf "$tmp"' EXIT
  curl -fsSL "https://github.com/ArturVah/push-bro/archive/refs/tags/v$version.tar.gz" | tar -xz -C "$tmp" --strip-components 1

  echo "Building the app..."
  xcrun swiftc -O -swift-version 6 "$tmp/app/main.swift" -o "$tmp/app/pushbro" -framework AppKit -framework WebKit
  rm -rf "$dest"
  mkdir -p "$dest"
  cp -R "$tmp/app" "$tmp/overlay" "$tmp/hooks" "$tmp/bin" "$dest/"

  "$dest/bin/push-bro" on
  echo "To turn him off: ~/.push-bro/bin/push-bro off"
}

main "$@"
