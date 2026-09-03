#!/bin/sh
# now_playing.sh: SketchyBar item script (POSIX sh, no bashisms).
# Env from SketchyBar: $NAME (item), $SENDER (event), plus trigger payload:
#   $TITLE $ARTIST $ALBUM $BUNDLE $PLAYING $LABEL $ICON
# Also handles `routine` ticks (polling fallback) via `get`.
#
# Dispatch is O(1) on $SENDER; every branch is a single `sketchybar --set`.

# SketchyBar.app runs with a minimal PATH, so resolve the binary once:
# explicit env wins, then PATH, then the common install prefixes.
BIN="${NOW_PLAYING_BIN:-}"
if [ -z "$BIN" ]; then
  if command -v sketchybar-now-playing >/dev/null 2>&1; then
    BIN="sketchybar-now-playing"
  else
    for dir in "$HOME/.local/bin" /opt/homebrew/bin /usr/local/bin; do
      if [ -x "$dir/sketchybar-now-playing" ]; then
        BIN="$dir/sketchybar-now-playing"
        break
      fi
    done
  fi
fi

set_label() {
  # $1=label $2=icon $3=playing(true/false/"")
  if [ -z "$1" ]; then
    sketchybar --set "$NAME" drawing=off
  else
    sketchybar --set "$NAME" label="$1" icon="$2" drawing=on
  fi
}

case "$SENDER" in
  now_playing_change)
    set_label "$LABEL" "$ICON" "$PLAYING"
    ;;
  mouse.clicked)
    # Left toggles, right skips. Fire and forget; the daemon's
    # change event converges the bar, so no output parsing here.
    if [ -n "$BIN" ]; then
      if [ "${BUTTON:-left}" = "right" ]; then
        "$BIN" next >/dev/null 2>&1
      else
        "$BIN" toggle >/dev/null 2>&1
      fi
    fi
    ;;
  routine|forced|*)
    # Polling fallback when the daemon is not running.
    if [ -n "$BIN" ]; then
      OUT="$("$BIN" get 2>/dev/null)"
      case "$OUT" in
        ""|"No player available") sketchybar --set "$NAME" drawing=off ;;
        *) sketchybar --set "$NAME" label="$OUT" drawing=on ;;
      esac
    fi
    ;;
esac
