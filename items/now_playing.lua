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
    local button = sbar.add("item", def.name, {
      position = "right",
      label = { drawing = false },
      icon = { string = def.glyph or ICON_PLAY },
    })
    button:subscribe(EVENT, function(env)
      if env.LABEL == nil or env.LABEL == "" then
        button:set({ drawing = false })
      elseif def.glyph == nil then
        button:set({ drawing = true, icon = { string = toggle_glyph(env) } })
      else
        button:set({ drawing = true, icon = { string = def.glyph } })
      end
    end)
    button:subscribe("mouse.clicked", function()
      sbar.exec(BIN .. CONFIG_FLAG .. " " .. def.action)
    end)
  end

  -- The `|` between the label and the buttons. Not clickable.
  local sep = sbar.add("item", "now_playing.sep", {
    position = "right",
    label = { string = "|" },
    icon = { drawing = false },
  })
  sep:subscribe(EVENT, function(env)
    sep:set({ drawing = not (env.LABEL == nil or env.LABEL == "") })
  end)
end

local now_playing = sbar.add("item", "now_playing", {
  position = "right",
  update_freq = 10,
  scroll_texts = true,
  label = { max_chars = 40, scroll_duration = 100 },
})

-- Event path: the daemon pushes TITLE, ARTIST, LABEL, ICON, PLAYING.
-- Scrolling follows playback so paused text sits still.
now_playing:subscribe(EVENT, function(env)
  if env.LABEL == nil or env.LABEL == "" then
    now_playing:set({ drawing = false })
  else
    now_playing:set({
      drawing = true,
      label = { string = env.LABEL },
      icon = { string = env.ICON or "" },
      scroll_texts = env.PLAYING == "true",
    })
  end
end)

-- Polling fallback and post reload convergence. `sync` pushes label,
-- icon and visibility in one call, so Lua parses no output.
now_playing:subscribe("routine", function()
  sbar.exec(BIN .. CONFIG_FLAG .. " sync now_playing")
end)

-- Left click toggles, right click skips to the next track.
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
