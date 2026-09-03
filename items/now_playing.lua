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
--   sbar.exec("pgrep -f 'sketchybar-now-playing daemon' >/dev/null || "
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

sbar.add("event", EVENT)

local now_playing = sbar.add("item", "now_playing", {
  position = "right",
  update_freq = 10,
  scroll_texts = true,
  label = { max_chars = 20, scroll_duration = 100 },
})

-- Event path: the daemon pushes TITLE, ARTIST, LABEL, ICON, PLAYING.
now_playing:subscribe(EVENT, function(env)
  if env.LABEL == nil or env.LABEL == "" then
    now_playing:set({ drawing = false })
  else
    now_playing:set({
      drawing = true,
      label = { string = env.LABEL },
      icon = { string = env.ICON or "" },
    })
  end
end)

-- Polling fallback: keeps the bar working when the daemon is absent.
now_playing:subscribe("routine", function()
  sbar.exec(BIN .. " get", function(result)
    local text = result:gsub("%s+$", "")
    if text == "" or text:match("^No player") then
      now_playing:set({ drawing = false })
    else
      now_playing:set({ drawing = true, label = { string = text } })
    end
  end)
end)

-- Left click toggles, right click skips to the next track.
now_playing:subscribe("mouse.clicked", function(env)
  if env.BUTTON == "right" then
    sbar.exec(BIN .. " next")
  else
    sbar.exec(BIN .. " toggle")
  end
end)
