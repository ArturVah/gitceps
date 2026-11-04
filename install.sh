#!/bin/bash
# Installs Gitceps into ~/.gitceps and turns it on for every git repo:
#   curl -fsSL https://raw.githubusercontent.com/ArturVah/gitceps/main/install.sh | bash
# Everything runs inside main(), so a download cut off halfway does nothing.
set -euo pipefail

main() {
  local version="0.4.0" dest="$HOME/.gitceps"

  if [ "$(uname)" != Darwin ]; then
    echo "Gitceps only runs on macOS." >&2; exit 1
  fi
  local swift_error
  if ! swift_error=$(xcrun --find swiftc 2>&1 >/dev/null); then
    echo "Gitceps needs Apple's Swift compiler, and xcrun couldn't find it:" >&2
    echo "  ${swift_error:-no details}" >&2
    echo "If the Command Line Tools aren't installed: xcode-select --install" >&2
    exit 1
  fi
  if [ -e "$dest" ] && [ ! -x "$dest/bin/gitceps" ]; then
    echo "$dest already exists and isn't Gitceps. Move it out of the way and run this again." >&2; exit 1
  fi

  echo "Downloading Gitceps $version..."
  tmp=$(mktemp -d)   # not local: the EXIT trap runs after main() has returned
  trap 'rm -rf "$tmp"' EXIT
  curl -fsSL "https://github.com/ArturVah/gitceps/archive/refs/tags/v$version.tar.gz" | tar -xz -C "$tmp" --strip-components 1

  echo "Building the app..."
  xcrun swiftc -O -swift-version 6 "$tmp/app/main.swift" -o "$tmp/app/gitceps-app" -framework AppKit -framework WebKit
  rm -rf "$dest"
  mkdir -p "$dest"
  cp -R "$tmp/app" "$tmp/overlay" "$tmp/hooks" "$tmp/bin" "$dest/"

  "$dest/bin/gitceps" on
  echo "To turn him off: ~/.gitceps/bin/gitceps off"
}

main "$@"
