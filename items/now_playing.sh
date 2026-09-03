# now_playing.sh: bar wiring. Source from sketchybarrc, or run standalone.
# Event-driven (preferred): daemon pushes `now_playing_change`.
# Falls back to `update_freq` polling when the daemon is absent.
#
# Usage in sketchybarrc:
#   source ~/.config/sketchybar/items/now_playing.sh
#
# Env knobs (all optional):
#   NOW_PLAYING_BIN    explicit binary path (auto resolved when unset)
#   NOW_PLAYING_POS    bar position (default: right)
#   NOW_PLAYING_EVENT  custom event name (default: now_playing_change)
#   NOW_PLAYING_MAX    max chars before native scroll (default: 20)

# Same resolver as the plugin script: explicit env wins, then PATH,
# then the common install prefixes. sketchybarrc often runs outside a
# login shell, so PATH alone is not reliable.
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
POS="${NOW_PLAYING_POS:-right}"
EVENT="${NOW_PLAYING_EVENT:-now_playing_change}"
MAX="${NOW_PLAYING_MAX:-20}"
PLUGIN_DIR="$(cd "$(dirname "$0")/../plugins" && pwd)"

sketchybar --add event "$EVENT" \
  --add item now_playing "$POS" \
  --set now_playing \
    script="$PLUGIN_DIR/now_playing.sh" \
    click_script="$PLUGIN_DIR/now_playing.sh" \
    update_freq=10 \
    scroll_texts=on \
    label.max_chars="$MAX" \
    label.scroll_duration=100 \
  --subscribe now_playing "$EVENT" mouse.clicked

# Start the daemon once (no-op if already running). The pgrep pattern
# matches the bare name so it hits no matter which prefix resolved.
if [ -n "$BIN" ] && ! pgrep -f "sketchybar-now-playing daemon" >/dev/null 2>&1; then
  "$BIN" daemon --event "$EVENT" >/tmp/sketchybar-now-playing.log 2>&1 &
fi
