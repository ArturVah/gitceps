#!/bin/bash
# Installs Gitceps into ~/.gitceps and turns it on for every git repo:
#   curl -fsSL https://raw.githubusercontent.com/ArturVah/gitceps/main/install.sh | bash
# Run again (or `gitceps update`) to update. An update leaves him on or off, as he was.
# Everything runs inside main(), so a download cut off halfway does nothing.
set -euo pipefail

main() {
  local dest="$HOME/.gitceps" asset
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
  tmp=$(mktemp -d)   # not local: the EXIT trap runs after main() has returned
  trap 'rm -rf "$tmp"' EXIT
  curl -fsSL "$from/$asset" -o "$tmp/$asset"
  curl -fsSL "$from/SHA256SUMS" -o "$tmp/SHA256SUMS"
  local sum
  if command -v sha256sum >/dev/null; then sum=$(sha256sum "$tmp/$asset"); else sum=$(shasum -a 256 "$tmp/$asset"); fi
  if ! grep -q "^${sum%% *}  $asset\$" "$tmp/SHA256SUMS"; then
    echo "The download doesn't match its checksum, so nothing was installed." >&2; exit 1
  fi
  mkdir "$tmp/new"
  tar -xzf "$tmp/$asset" -C "$tmp/new"

  # Each program is put in place with one rename, so a hook that runs meanwhile finds the old one or the new.
  # What version 0.5 kept here goes: its hooks are rewritten by `gitceps on` below.
  mkdir -p "$dest/bin"
  local program
  for program in gitceps gitceps-window; do
    mv -f "$tmp/new/$program" "$dest/bin/$program"
  done
  rm -rf "$dest/app" "$dest/overlay" "$dest/tour" "$dest/skills" "$dest/VERSION"

  local version; version=$("$dest/bin/gitceps" --version)
  if [ "$(uname -s)" = Linux ] && ! ldconfig -p 2>/dev/null | grep -q 'libwebkit2gtk-4\.1'; then
    echo "His windows need the WebKitGTK 4.1 library (libwebkit2gtk-4.1-0 on Debian and Ubuntu, webkit2gtk4.1 on Fedora)."
    echo "Until it's installed the secret check still works, without the animations."
  fi
  if $fresh || $was_on; then
    "$dest/bin/gitceps" on >/dev/null
    echo "${version/gitceps/Gitceps} is installed and on for every repo. Try a git push."
    $fresh && echo "To turn him off: ~/.gitceps/bin/gitceps off"
  else
    echo "${version/gitceps/Gitceps} is installed. He's off. To turn him on: ~/.gitceps/bin/gitceps on"
  fi
  return 0
}

main "$@"
