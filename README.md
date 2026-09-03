# sketchybar-now-playing

Show what is playing on macOS in SketchyBar. Spotify, Apple Music,
browser tabs with video or music, and anything else that appears in
Control Center.

One small Rust binary plus two shell scripts. It is event driven, so it
idles at zero CPU and updates the bar the moment the track changes.

## Features

* Current title and artist in the bar, with a per player icon
* Play, pause, toggle, next and previous controls
* Left click toggles, right click skips to the next track
* Hides itself when nothing plays (optional)
* Native bar scrolling for long titles, no extra tooling
* JSON output and a live change feed for scripting

## Requirements

* macOS 13 or later (tested on macOS 26)
* [SketchyBar](https://github.com/FelixKratz/SketchyBar) installed and running
* Rust 1.75 or later, only needed to build
* A Nerd Font in the bar config, only needed for the player icons

## Install

Build the binary:

```sh
cargo build --release
```

Put it on your `PATH` with an atomic rename. Never copy over the
destination in place while the bar may be executing it, or macOS can
refuse to launch that file path afterwards. `install` and `mv` are
atomic. Bare `cp` onto a live path is not.

```sh
mkdir -p ~/.local/bin
install -m 755 target/release/sketchybar-now-playing ~/.local/bin/
```

Confirm it sees your media (play something first):

```sh
sketchybar-now-playing get
sketchybar-now-playing get --json
```

Example output:

```sh
Alfred Hall - Pearl Diver - 777tv
{"title":"Alfred Hall - Pearl Diver","artist":"777tv","bundle_id":"org.mozilla.firefox","playing":true}
```

## Integrate with SketchyBar

There are two parts: the bar item (what you see) and the daemon
(what watches for track changes). The steps below wire both.

### 1. Copy the scripts

Shell setup (Lua users skip to step 5):

```sh
cp plugins/now_playing.sh ~/.config/sketchybar/plugins/now_playing.sh
cp items/now_playing.sh ~/.config/sketchybar/items/now_playing.sh
chmod +x ~/.config/sketchybar/plugins/now_playing.sh
```

### 2. Source the wiring from sketchybarrc

Add this after your bar setup, before the final `sketchybar --update`:

```sh
source "$HOME/.config/sketchybar/items/now_playing.sh"
```

That one line does all of the following: it creates a custom
`now_playing_change` event, adds a `now_playing` item on the right side,
subscribes it to the event and to mouse clicks, enables native scrolling,
and starts the daemon if it is not already running.

### 3. Reload the bar

```sh
sketchybar --reload
```

Play a song or video. The bar shows the label within a moment.

### 4. Manual setup (alternative)

If you prefer explicit config over the sourced script, put this in
`sketchybarrc`:

```sh
PLUGIN_DIR="$HOME/.config/sketchybar/plugins"
BIN="$HOME/.local/bin/sketchybar-now-playing"

sketchybar --add event now_playing_change \
  --add item now_playing right \
  --set now_playing \
    script="$PLUGIN_DIR/now_playing.sh" \
    click_script="$PLUGIN_DIR/now_playing.sh" \
    update_freq=10 \
    scroll_texts=on \
    label.max_chars=20 \
    label.scroll_duration=100 \
  --subscribe now_playing now_playing_change mouse.clicked

pgrep -f "[s]ketchybar-now-playing daemon" >/dev/null 2>&1 || \
  "$BIN" daemon --event now_playing_change >>/tmp/sketchybar-now-playing.log 2>&1 &
```

### 5. Lua setup (SbarLua)

For Lua based configs, use `items/now_playing.lua` instead of the shell
scripts:

```sh
cp items/now_playing.lua ~/.config/sketchybar/items/now_playing.lua
```

Load it from `init.lua` after the bar setup, then autostart the daemon
once (the `pgrep` guard keeps reloads from stacking daemons):

```lua
require("items.now_playing")

sbar.exec("pgrep -f '[s]ketchybar-now-playing daemon' >/dev/null || "
  .. "sketchybar-now-playing daemon >>/tmp/sketchybar-now-playing.log 2>&1 &")
```

The item subscribes to the same `now_playing_change` event, so the Rust
daemon works unchanged. Click and polling fallback behavior match the
shell plugin.

### 6. Tune with environment variables

The sourced wiring script reads these optional variables:

| Variable          | Default              | Purpose                              |
| ----------------- | -------------------- | ------------------------------------ |
| `NOW_PLAYING_BIN` | auto resolved | Explicit binary path. Otherwise `PATH`, then `~/.local/bin`, `/opt/homebrew/bin`, `/usr/local/bin` |
| `NOW_PLAYING_CONFIG` | unset | Config file forwarded to every binary call |
| `NOW_PLAYING_POS` | `right`              | Bar position of the item             |
| `NOW_PLAYING_EVENT` | `now_playing_change` | Custom event name                  |
| `NOW_PLAYING_MAX` | `20`                 | Chars before the label scrolls       |

### Polling fallback

When the daemon runs, the item updates through events and costs nothing
while idle. If the daemon is ever absent, the same item falls back to
`update_freq` polling through `sync`, which pushes label, icon and
visibility in one call. The same call reconverges a freshly reloaded
item within one tick.

## Configuration

Copy the example and pass it with `--config`:

```sh
cp config.example.toml ~/.config/sketchybar/now-playing.toml
```

```toml
separator = " - "
hide_output = false
max_chars = 20

[icons]
"com.spotify.client" = ""
```

`separator` joins the fields. `hide_output` hides the item instead of
showing a placeholder when nothing plays. Entries under `[icons]` map a
player bundle id to a glyph and win over the built ins.

## Commands

| Command    | Purpose                                              |
| ---------- | ---------------------------------------------------- |
| `get`      | Print the current track once                         |
| `get --json` | Print it as compact JSON, `null` when idle        |
| `stream`   | Print a JSON line per change, for pipes and scripts  |
| `daemon`   | Push changes into SketchyBar in a loop               |
| `daemon --set ITEM` | Update the item directly, no event needed   |
| `sync ITEM`    | Snapshot once and push label, icon and visibility into ITEM |
| `play`, `pause`, `toggle`, `next`, `prev` | Control playback |

Every command accepts a global `--config PATH` flag.

## Click behavior

| Click        | Action               |
| ------------ | -------------------- |
| Left         | Toggle play and pause |
| Right        | Skip to next track   |

Set the binary location for clicks with `NOW_PLAYING_BIN` if it is not
on the default `PATH` inside SketchyBar.

## Verify the setup

```sh
pgrep -f "[s]ketchybar-now-playing daemon"   # daemon is running
sketchybar --query now_playing             # item exists
tail -20 /tmp/sketchybar-now-playing.log   # daemon log is clean
```

To test the item without playing media, fire the event by hand:

```sh
sketchybar --trigger now_playing_change \
  TITLE="Pearl Diver" ARTIST="777tv" LABEL="Pearl Diver - 777tv" PLAYING=true
```

## Troubleshooting

Item never appears
: Make sure the `source` line sits before the final
`sketchybar --update` in `sketchybarrc`, then run `sketchybar --reload`.

Bar shows `No player available` while media plays
: Give the daemon a couple of seconds after starting. It waits for the
first system payload on launch. Browser media also needs an active media
session in the tab.

Icons show as boxes
: Install a Nerd Font and set it as the item font. Labels work with any
font. Icons need the Nerd Font glyphs.

Controls print `no active Now Playing client`
: Nothing is loaded in any player. Open the player or tab first, then
retry.

Log shows `spawn sketchybar --set` failures
: SketchyBar is not running, or the binary cannot find it on `PATH`.
Start the bar and retry.

## Uninstall

```sh
sketchybar --remove now_playing
sketchybar --remove event now_playing_change
pkill -f "[s]ketchybar-now-playing daemon"
rm ~/.local/bin/sketchybar-now-playing
rm ~/.config/sketchybar/plugins/now_playing.sh ~/.config/sketchybar/items/now_playing.sh
```

## How it works

macOS exposes one global Now Playing state through the private
MediaRemote framework. Since macOS 15.4 that API needs a special
entitlement, so the binary reads it through the system `perl` loader,
which keeps it working on current releases including macOS 26. The
daemon subscribes to change notifications, compares the new snapshot
with the previous one, and calls `sketchybar --trigger` only on real
change. All SketchyBar calls use direct process execution with no
shell, so track titles can never inject commands.

## License

MIT. See [LICENSE](LICENSE).
