-- now_playing.lua: Now Playing item for SbarLua based configs.
--
-- Needs the sketchybar-now-playing binary on PATH (or set BIN below).
-- Drop this file into ~/.config/sketchybar/items/ and load it from
-- init.lua after the bar setup:
--
--   require("items.now_playing")
--
-- Then start the event daemon once (init.lua, guarded so reloads do
-- not stack daemons):
--
--   sbar.exec("pgrep -f '[s]ketchybar-now-playing daemon' >/dev/null || "
--     .. "sketchybar-now-playing daemon >>/tmp/sketchybar-now-playing.log 2>&1 &")

-- sbar.exec inherits the bar's minimal PATH, so resolve the binary the
-- same way the shell plugin does: explicit env wins, then the common
-- install prefixes, then plain PATH. The os.execute probe degrades
-- gracefully (both Lua 5.1 numeric and 5.2+ boolean results accepted).
local function find_binary()
  local override = os.getenv("NOW_PLAYING_BIN")
  if override ~= nil and override ~= "" then
    return override
  end
  local home = os.getenv("HOME") or ""
  local candidates = {
    home .. "/.cargo/bin/sketchybar-now-playing",
    home .. "/.local/bin/sketchybar-now-playing",
    "/opt/homebrew/bin/sketchybar-now-playing",
    "/usr/local/bin/sketchybar-now-playing",
  }
  for _, path in ipairs(candidates) do
    local ok = os.execute("[ -x '" .. path .. "' ]")
    if ok == true or ok == 0 then
      return path
    end
  end
  return "sketchybar-now-playing"
end

local BIN = find_binary()
local EVENT = "now_playing_change"

-- Optional config file, forwarded to every invocation. Paths with a
-- single quote are not supported here; use the shell plugin if needed.
local CONFIG_FLAG = ""
do
  local config_path = os.getenv("NOW_PLAYING_CONFIG")
  if config_path ~= nil and config_path ~= "" then
    CONFIG_FLAG = " --config '" .. config_path .. "'"
  end
end

sbar.add("event", EVENT)

-- Transport glyphs (Nerd Font set, same as icons.rs). The daemon event
-- carries the live TOGGLE_ICON; these are the pre-event / fallback faces.
local ICON_PREV = ""
local ICON_PLAY = ""
local ICON_PAUSE = ""
local ICON_NEXT = ""

-- Set to 0 to keep the single track item with no transport buttons.
local CONTROLS = os.getenv("NOW_PLAYING_CONTROLS") ~= "0"

-- Last playback state from the event feed. Ground truth only: updated on
-- each event, never flipped on click, so `scroll_texts` strictly follows
-- PLAYING. Long lived Lua state, so no query or state file is needed,
-- unlike the shell plugin.
local playing_state = false

local function toggle_glyph(env)
  if env.TOGGLE_ICON ~= nil and env.TOGGLE_ICON ~= "" then
    return env.TOGGLE_ICON
  elseif env.PLAYING == "true" then
    return ICON_PAUSE
  else
    return ICON_PLAY
  end
end

-- Transport buttons `| prev play next`, each button its own item so every
-- one is clickable. Added rightmost-first: right-side items stack
-- leftwards, so this lands as `label | prev play next` left to right.
local control_defs = {
  { name = "now_playing.next", action = "next", glyph = ICON_NEXT },
  { name = "now_playing.toggle", action = "toggle", glyph = nil },
  { name = "now_playing.prev", action = "prev", glyph = ICON_PREV },
}

if CONTROLS then
  for _, def in ipairs(control_defs) do
    -- No update_freq and no routine tick: buttons are purely event
    -- driven, and the main item's `sync` tick fans out to them.
    -- Starts hidden; the first track reveals it, idle never hides it.
    local button = sbar.add("item", def.name, {
      position = "right",
      drawing = false,
      label = { drawing = false },
      icon = { string = def.glyph or ICON_PLAY, padding_left = 8, padding_right = 8 },
    })
    button:subscribe(EVENT, function(env)
      -- Sticky last track: empty LABEL means idle. Refresh the toggle
      -- glyph to the paused set but leave `drawing` untouched, so a shown
      -- bar stays shown frozen and a never-shown bar stays hidden.
      if env.LABEL == nil or env.LABEL == "" then
        if def.glyph == nil then
          button:set({ icon = { string = toggle_glyph(env) } })
        end
      elseif def.glyph == nil then
        button:set({ drawing = true, icon = { string = toggle_glyph(env) } })
      else
        button:set({ drawing = true, icon = { string = def.glyph } })
      end
    end)
    button:subscribe("mouse.clicked", function()
      -- No optimistic scroll flip: scroll strictly follows PLAYING from
      -- the event feed and the `sync` tick.
      sbar.exec(BIN .. CONFIG_FLAG .. " " .. def.action)
    end)
  end

  -- The `|` between the label and the buttons. Not clickable.
  -- Starts hidden; idle leaves it exactly as-is.
  local sep = sbar.add("item", "now_playing.sep", {
    position = "right",
    drawing = false,
    label = { string = "|" },
    icon = { drawing = false },
  })
  sep:subscribe(EVENT, function(env)
    if not (env.LABEL == nil or env.LABEL == "") then
      sep:set({ drawing = true })
    end
  end)
end

local now_playing = sbar.add("item", "now_playing", {
  position = "right",
  drawing = false,
  update_freq = 10,
  scroll_texts = false,
  label = { max_chars = 40, scroll_duration = 100 },
})

-- Event path: the daemon pushes TITLE, ARTIST, LABEL, ICON, PLAYING.
-- Scrolling strictly follows playback: on only while playing, off while
-- paused or idle. Empty LABEL means idle: keep the last label/icon,
-- only stop motion, no `drawing` change (hidden-until-first-play).
now_playing:subscribe(EVENT, function(env)
  if env.LABEL == nil or env.LABEL == "" then
    playing_state = false
    now_playing:set({ scroll_texts = false })
  else
    playing_state = (env.PLAYING == "true")
    now_playing:set({
      drawing = true,
      label = { string = env.LABEL },
      icon = { string = env.ICON or "" },
      scroll_texts = playing_state,
    })
  end
end)

-- Polling fallback and post reload convergence. The periodic tick skips
-- the heavy `sync` (binary + perl adapter + bar update) while the event
-- daemon is alive: one `pgrep` instead of a full snapshot. Falls back to
-- polling the moment the daemon is gone.
now_playing:subscribe("routine", function()
  sbar.exec("pgrep -f '[s]ketchybar-now-playing daemon' >/dev/null 2>&1 || "
    .. BIN .. CONFIG_FLAG .. " sync now_playing")
end)

-- One immediate convergence at load so a reloaded bar with a live daemon
-- shows the current track without waiting one `update_freq` period.
sbar.exec(BIN .. CONFIG_FLAG .. " sync now_playing")

-- Left click toggles, right click skips to the next track. No optimistic
-- scroll flip here either: the event confirms the new PLAYING state.
now_playing:subscribe("mouse.clicked", function(env)
  if env.BUTTON == "right" then
    sbar.exec(BIN .. CONFIG_FLAG .. " next")
  else
    sbar.exec(BIN .. CONFIG_FLAG .. " toggle")
  end
end)

if CONTROLS then
  -- Group the pill so it can be styled as one unit. Unstyled to respect
  -- the host theme; add e.g. `background = { color = 0xff2b3a55,
  -- corner_radius = 6, height = 26 }` as the fourth arg for a solid
  -- pill background.
  sbar.add("bracket", "now_playing_bracket", {
    "now_playing",
    "now_playing.sep",
    "now_playing.prev",
    "now_playing.toggle",
    "now_playing.next",
  }, {})
end
