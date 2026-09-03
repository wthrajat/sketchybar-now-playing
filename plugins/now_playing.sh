#!/bin/sh
# now_playing.sh: SketchyBar item script (POSIX sh, no bashisms).
# Env from SketchyBar: $NAME (item), $SENDER (event), plus trigger payload:
#   $TITLE $ARTIST $ALBUM $BUNDLE $PLAYING $LABEL $ICON
# Also handles `routine` ticks and mouse clicks.
#
# Optional env:
#   NOW_PLAYING_BIN     explicit binary path (auto resolved when unset)
#   NOW_PLAYING_CONFIG  config file forwarded to every invocation
#
# Dispatch is O(1) on $SENDER; every branch is a single action.

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

# Thin wrapper so --config is honored without repeating the conditional.
run_bin() {
  if [ -n "$NOW_PLAYING_CONFIG" ]; then
    "$BIN" --config "$NOW_PLAYING_CONFIG" "$@"
  else
    "$BIN" "$@"
  fi
}

set_label() {
  # $1=label $2=icon $3=playing ("true" scrolls, anything else stays put)
  if [ -z "$1" ]; then
    sketchybar --set "$NAME" drawing=off
  elif [ "$3" = "true" ]; then
    sketchybar --set "$NAME" label="$1" icon="$2" scroll_texts=on drawing=on
  else
    sketchybar --set "$NAME" label="$1" icon="$2" scroll_texts=off drawing=on
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
        run_bin next >/dev/null 2>&1
      else
        run_bin toggle >/dev/null 2>&1
      fi
    fi
    ;;
  routine|forced|*)
    # Fallback and post reload convergence: pushes label, icon and
    # visibility in one call, so the shell parses no output.
    if [ -n "$BIN" ]; then
      run_bin sync "$NAME" >/dev/null 2>&1
    fi
    ;;
esac
