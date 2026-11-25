#!/bin/bash
# Installs Gitceps into ~/.gitceps and turns it on for every git repo:
#   curl -fsSL https://raw.githubusercontent.com/ArturVah/gitceps/main/install.sh | bash
# Run again (or `gitceps update`) to update. An update leaves him on or off, as he was.
# Everything runs inside main(), so a download cut off halfway does nothing.
set -euo pipefail

main() {
  local asset
  dest="$HOME/.gitceps"   # not local, nor is tmp below: the EXIT trap runs after main() has returned
  # Where the release files are. Another address, or a folder as file:///path, serves a mirror or a test.
  local from="${GITCEPS_RELEASE:-https://github.com/ArturVah/gitceps/releases/latest/download}"

  case "$(uname -s) $(uname -m)" in
    Darwin\ *)                     asset=gitceps-macos.tar.gz ;;
    Linux\ x86_64)                 asset=gitceps-linux-x86_64.tar.gz ;;
    Linux\ aarch64 | Linux\ arm64) asset=gitceps-linux-arm64.tar.gz ;;
    *)
      echo "There's no ready-made Gitceps for $(uname -s) on $(uname -m)." >&2
      echo "With Rust installed, build it: cargo install gitceps gitceps-window" >&2
      exit 1 ;;
  esac
  if [ -e "$dest" ] && [ ! -x "$dest/bin/gitceps" ] && [ ! -d "$dest/hooks" ]; then
    echo "$dest already exists and isn't Gitceps. Move it out of the way and run this again." >&2; exit 1
  fi
  local fresh=true was_on=false
  if [ -x "$dest/bin/gitceps" ]; then
    fresh=false
    [ "$("$dest/bin/gitceps" status 2>/dev/null)" = on ] && was_on=true
  fi

  echo "Downloading Gitceps..."
  # Inside the Gitceps folder, so putting a program in place is one rename and never a copy between disks.
  mkdir -p "$dest/bin"
  tmp=$(mktemp -d "$dest/.new.XXXXXX")
  trap 'rm -rf "$tmp"; rmdir "$dest/bin" "$dest" 2>/dev/null || true' EXIT   # the folders stay unless still empty
  curl -fsSL "$from/$asset" -o "$tmp/$asset"
  curl -fsSL "$from/SHA256SUMS" -o "$tmp/SHA256SUMS"
  local sum
  if command -v sha256sum >/dev/null; then sum=$(sha256sum "$tmp/$asset"); else sum=$(shasum -a 256 "$tmp/$asset"); fi
  if ! grep -q "^${sum%% *}  $asset\$" "$tmp/SHA256SUMS"; then
    echo "The download doesn't match its checksum, so nothing was installed." >&2; exit 1
  fi
  mkdir "$tmp/new"
  tar -xzf "$tmp/$asset" -C "$tmp/new"
  if ! "$tmp/new/gitceps" --version >/dev/null 2>&1; then
    echo "This build of Gitceps doesn't run on this system, so nothing was installed." >&2
    echo "With Rust installed, build it: cargo install gitceps gitceps-window" >&2
    exit 1
  fi

  # Each program is put in place with one rename, so a hook that runs meanwhile finds the old one or the new.
  local program
  for program in gitceps gitceps-window; do
    mv -f "$tmp/new/$program" "$dest/bin/$program"
  done
  local from_0_5=false
  [ ! -f "$dest/hooks/lib.sh" ] || from_0_5=true
  rm -rf "$dest/app" "$dest/overlay" "$dest/tour" "$dest/skills" "$dest/VERSION"   # what version 0.5 kept here

  local version; version=$("$dest/bin/gitceps" --version)
  # Without its library the window program can't start, and the system's loader says so.
  case "$("$dest/bin/gitceps-window" 2>&1 || true)" in
    *"shared libraries"*)
      echo "His windows need the WebKitGTK 4.1 library (libwebkit2gtk-4.1-0 on Debian and Ubuntu, webkit2gtk4.1 on Fedora)."
      echo "Until it's installed the secret check still works, without the animations." ;;
  esac
  if $fresh || $was_on; then
    "$dest/bin/gitceps" on >/dev/null
    echo "${version/gitceps/Gitceps} is installed and on for every repo. Try a git push."
    $fresh && echo "To turn him off: ~/.gitceps/bin/gitceps off"
  else
    # He stays off, but a repo may point at version 0.5's hooks by itself, and `on` is what writes the new ones.
    if $from_0_5 && "$dest/bin/gitceps" on >/dev/null 2>&1; then "$dest/bin/gitceps" off >/dev/null; fi
    echo "${version/gitceps/Gitceps} is installed. He's off. To turn him on: ~/.gitceps/bin/gitceps on"
  fi
  return 0
}

main "$@"
