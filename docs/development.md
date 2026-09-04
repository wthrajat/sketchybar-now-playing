# Development

Notes for working on sketchybar-now-playing itself. User docs
(install, bar setup, config) live in [README.md](../README.md).

## Local setup

```sh
cargo build
cargo test
cargo run -- get --json
```

The checks below run in CI and must pass locally first:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Shell and Lua files are checked with:

```sh
sh -n plugins/now_playing.sh items/now_playing.sh install.sh
luac -p items/now_playing.lua
```

## Architecture

* `src/main.rs`: CLI entry, exit codes only, no business logic.
* `src/cli.rs`: clap subcommands.
* `src/error.rs`: custom `Error` enum, no `anyhow`.
* `src/config.rs`: defaults plus TOML load.
* `src/track.rs`: `Track` snapshot, label building, diffing.
* `src/icons.rs`: `bundle_id` to glyph map, zero alloc, plus the
  transport glyphs (`ICON_PREV` / `PLAY` / `PAUSE` / `NEXT`) and the
  playback-aware `toggle_icon` helper.
* `src/media.rs`: `MediaSource` trait plus the `NowPlayingPerl`
  backend. `Drop` reaps the helper process so short lived commands
  never orphan it.
* `src/sketchybar.rs`: `trigger`, `set` and `sync` via direct exec,
  no shell. `trigger` also carries `PREV/TOGGLE/NEXT_ICON`; `set`
  renders `<base>.prev` / `.toggle` / `.next` / `.sep` control items
  by name suffix, and `set_with_controls` fans `daemon --set` out to
  the siblings (best effort).
* `plugins/now_playing.sh`: item script for shell configs. Dispatches
  on `$NAME` so the main item and each control sibling converge
  independently; any non-click, non-routine `$SENDER` is treated as the
  change event, so custom `$NOW_PLAYING_EVENT` names work.
* `items/now_playing.sh`: bar wiring for shell configs. Adds the
  control items plus a styling bracket (`NOW_PLAYING_CONTROLS=0`
  keeps the single track item).
* `items/now_playing.lua`: same item for SbarLua configs.
* `install.sh`: one line installer from GitHub Release tarballs.

Event flow: the daemon subscribes to MediaRemote notifications, diffs
each snapshot against the previous one, and calls
`sketchybar --trigger` only on real change. The `update_freq` fallback
calls `sync`, which pushes label, icon and visibility in one call.

## How it works

macOS exposes one global Now Playing state through the private
MediaRemote framework. Since macOS 15.4 that API needs a special
entitlement, so the binary reads it through the system `perl` loader,
which keeps it working on current releases including macOS 26. The
daemon subscribes to change notifications, compares the new snapshot
with the previous one, and calls `sketchybar --trigger` only on real
change. All SketchyBar calls use direct process execution with no
shell, so track titles can never inject commands.

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

Item stays hidden while media plays
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
sketchybar --remove now_playing.next
sketchybar --remove now_playing.toggle
sketchybar --remove now_playing.prev
sketchybar --remove now_playing.sep
sketchybar --remove now_playing
sketchybar --remove bracket now_playing_bracket
sketchybar --remove event now_playing_change
pkill -f "[s]ketchybar-now-playing daemon"
rm ~/.local/bin/sketchybar-now-playing
rm ~/.config/sketchybar/plugins/now_playing.sh ~/.config/sketchybar/items/now_playing.sh
```

## Release process

Bumping `version` in `Cargo.toml` on `main` is the release. CI tags
`vX.Y.Z`, publishes to crates.io, builds per arch tarballs and creates
the GitHub Release. It needs the `CRATES_IO_TOKEN` repo secret. The
install flow must keep working from a cold machine, so test
`install.sh` against a local file server before changing its layout
(see AGENTS.md).
