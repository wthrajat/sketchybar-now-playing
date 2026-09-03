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
#   NOW_PLAYING_CONTROLS  set to 0 to keep the single track item with no
#     transport buttons (default: 1, i.e. `label | prev play next`)

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
MAX="${NOW_PLAYING_MAX:-40}"
CONTROLS="${NOW_PLAYING_CONTROLS:-1}"
PLUGIN_DIR="$(cd "$(dirname "$0")/../plugins" && pwd)"

sketchybar --add event "$EVENT"

add_main() {
  sketchybar --add item now_playing "$POS" \
    --set now_playing \
      script="$PLUGIN_DIR/now_playing.sh" \
      click_script="$PLUGIN_DIR/now_playing.sh" \
      update_freq=10 \
      scroll_texts=on \
      label.max_chars="$MAX" \
      label.scroll_duration=100 \
    --subscribe now_playing "$EVENT" mouse.clicked
}

add_control() {
  # $1=suffix (prev|toggle|next) $2=initial icon. Label stays off: the
  # button is icon only. Same event/click plumbing as the main item;
  # the plugin tells siblings apart via $NAME.
  sketchybar --add item "now_playing.$1" "$POS" \
    --set "now_playing.$1" \
      script="$PLUGIN_DIR/now_playing.sh" \
      click_script="$PLUGIN_DIR/now_playing.sh" \
      update_freq=10 \
      label.drawing=off \
      icon="$2" \
    --subscribe "now_playing.$1" "$EVENT" mouse.clicked
}

add_sep() {
  # The `|` between the label and the buttons. Not clickable.
  sketchybar --add item now_playing.sep "$POS" \
    --set now_playing.sep \
      script="$PLUGIN_DIR/now_playing.sh" \
      update_freq=10 \
      label="|" \
      icon.drawing=off \
    --subscribe now_playing.sep "$EVENT"
}

if [ "$CONTROLS" = "1" ]; then
  # Initial glyphs (Nerd Font transport set); the daemon event swaps the
  # toggle between play and pause on every change.
  if [ "$POS" = "right" ]; then
    # Right-side items stack leftwards, so add rightmost first to end up
    # with `label | prev play next` left to right.
    add_control next ""
    add_control toggle ""
    add_control prev ""
    add_sep
    add_main
  else
    add_main
    add_sep
    add_control prev ""
    add_control toggle ""
    add_control next ""
  fi
  # Group the pill so it can be styled as one unit. Unstyled by default
  # to respect the host theme; uncomment for a solid pill background:
  #   sketchybar --set now_playing_bracket background.color=0xff2b3a55 \
  #     background.corner_radius=6 background.height=26
  sketchybar --add bracket now_playing_bracket \
    now_playing now_playing.sep now_playing.prev now_playing.toggle now_playing.next
else
  add_main
fi

# Start the daemon once (no-op if already running). The pgrep pattern
# matches the bare name so it hits no matter which prefix resolved, and
# the [s] trick keeps pgrep from matching this very script while it runs.
if [ -n "$BIN" ] && ! pgrep -f "[s]ketchybar-now-playing daemon" >/dev/null 2>&1; then
  "$BIN" daemon --event "$EVENT" >/tmp/sketchybar-now-playing.log 2>&1 &
fi
