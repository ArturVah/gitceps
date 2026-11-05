#!/bin/bash
# Installs Gitceps into ~/.gitceps and turns it on for every git repo:
#   curl -fsSL https://raw.githubusercontent.com/ArturVah/gitceps/main/install.sh | bash
# Run again (or `gitceps update`) to update. An update leaves him on or off, as he was.
# Everything runs inside main(), so a download cut off halfway does nothing.
set -euo pipefail

main() {
  local version="0.5.3" dest="$HOME/.gitceps"

  if [ "$(uname)" != Darwin ]; then
    echo "Gitceps only runs on macOS." >&2; exit 1
  fi
  [ -e "$dest" ] || [ ! -e "$dest.old" ] || mv "$dest.old" "$dest"   # an update cut off between its two renames
  if [ -e "$dest" ] && [ ! -x "$dest/bin/gitceps" ]; then
    echo "$dest already exists and isn't Gitceps. Move it out of the way and run this again." >&2; exit 1
  fi
  local fresh=true was_on=false
  if [ -e "$dest" ]; then
    fresh=false
    [ "$("$dest/bin/gitceps" status 2>/dev/null)" = on ] && was_on=true
  fi
  local turn_on="He's off. To turn him on: ~/.gitceps/bin/gitceps on"
  if [ "$(cat "$dest/VERSION" 2>/dev/null)" = "$version" ]; then
    echo "Gitceps $version is already installed."
    $was_on || echo "$turn_on"
    return
  fi
  local swift_error
  if ! swift_error=$(xcrun --find swiftc 2>&1 >/dev/null); then
    echo "Gitceps needs Apple's Swift compiler, and xcrun couldn't find it:" >&2
    echo "  ${swift_error:-no details}" >&2
    echo "If the Command Line Tools aren't installed: xcode-select --install" >&2
    exit 1
  fi

  echo "Downloading Gitceps $version..."
  tmp=$(mktemp -d) next="$dest.new"   # not local: the EXIT trap runs after main() has returned
  trap 'rm -rf "$tmp" "$next"' EXIT
  curl -fsSL "https://github.com/ArturVah/gitceps/releases/download/v$version/gitceps-$version.tar.gz" | tar -xz -C "$tmp" --strip-components 1

  echo "Building the app..."
  xcrun swiftc -O -swift-version 6 "$tmp/app/main.swift" -o "$tmp/app/gitceps-app" -framework AppKit -framework WebKit
  # The new copy is built next to the old one and swapped in with two renames, so git never sees a
  # half-copied hooks folder. If an update stops between the renames, the next run restores the old copy.
  rm -rf "$next" "$dest.old"
  mkdir -p "$next"
  cp -R "$tmp/app" "$tmp/overlay" "$tmp/hooks" "$tmp/bin" "$next/"
  echo "$version" > "$next/VERSION"
  [ ! -e "$dest" ] || mv "$dest" "$dest.old"
  mv "$next" "$dest"
  rm -rf "$dest.old"

  if $fresh; then
    "$dest/bin/gitceps" on
    echo "To turn him off: ~/.gitceps/bin/gitceps off"
  elif $was_on; then
    "$dest/bin/gitceps" on >/dev/null   # points git at the new hooks again, in case a release moved them
    echo "Gitceps is now $version."
  else
    echo "Gitceps is now $version. $turn_on"
  fi
}

main "$@"
