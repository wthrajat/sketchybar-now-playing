#!/bin/sh
# install.sh: one line installer for sketchybar-now-playing.
#
#   curl -fsSL https://raw.githubusercontent.com/wthrajat/sketchybar-now-playing/main/install.sh | sh
#
# Env knobs:
#   VERSION         release tag, defaults to latest (example: VERSION=v0.1.0)
#   PREFIX          binary dir, defaults to $HOME/.local/bin
#   SKETCHYBAR_DIR  bar config dir, defaults to $HOME/.config/sketchybar
#   RELEASE_BASE    download base override (forks and tests)
set -eu

REPO="wthrajat/sketchybar-now-playing"
PREFIX="${PREFIX:-$HOME/.local/bin}"
SKETCHYBAR_DIR="${SKETCHYBAR_DIR:-$HOME/.config/sketchybar}"
RELEASE_BASE="${RELEASE_BASE:-https://github.com/$REPO/releases/download}"

die() {
  echo "install: $*" >&2
  exit 1
}

[ "$(uname -s)" = "Darwin" ] || die "macOS only"
case "$(uname -m)" in
  arm64) TARGET="aarch64-apple-darwin" ;;
  x86_64) TARGET="x86_64-apple-darwin" ;;
  *) die "unsupported arch: $(uname -m)" ;;
esac
command -v curl >/dev/null 2>&1 || die "curl is required"
command -v tar >/dev/null 2>&1 || die "tar is required"

if [ -z "${VERSION:-}" ]; then
  VERSION="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -1)"
  [ -n "$VERSION" ] || die "could not resolve latest release (pin one with VERSION=vX.Y.Z)"
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT INT TERM
ARCHIVE="sketchybar-now-playing-$TARGET.tar.gz"
echo "install: fetching $VERSION ($TARGET)"
curl -fsSL --retry 3 -o "$WORK/$ARCHIVE" "$RELEASE_BASE/$VERSION/$ARCHIVE"
tar -xzf "$WORK/$ARCHIVE" -C "$WORK"
ROOT="$WORK/sketchybar-now-playing-$TARGET"

# Atomic binary install: never rewrite a path the bar may be executing.
mkdir -p "$PREFIX" "$SKETCHYBAR_DIR/plugins" "$SKETCHYBAR_DIR/items"
install -m 755 "$ROOT/bin/sketchybar-now-playing" "$PREFIX/"
install -m 644 "$ROOT/plugins/now_playing.sh" "$SKETCHYBAR_DIR/plugins/"
install -m 644 "$ROOT/items/now_playing.sh" "$ROOT/items/now_playing.lua" "$SKETCHYBAR_DIR/items/"

echo "install: done"
echo "install: wire it up, then run: sketchybar --reload"
echo "  shell: source \"\$SKETCHYBAR_DIR/items/now_playing.sh\" from sketchybarrc"
echo "  lua:   require(\"items.now_playing\") from init.lua"
case ":$PATH:" in
  *":$PREFIX:"*) ;;
  *) echo "install: note $PREFIX is not on PATH; add it or set NOW_PLAYING_BIN" ;;
esac
