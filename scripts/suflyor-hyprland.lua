-- Source this from the user's Hyprland Lua configuration before starting Suflyor.
-- Wayland clients cannot force floating placement or topmost policy themselves.
hl.window_rule({
    name = "suflyor-native-app",
    match = { class = "^suflyor$" },
    float = true,
    size = "720 600",
    center = true,
})
hl.window_rule({
    name = "suflyor-native-windows",
    match = { title = "^[Ss]uflyor.*$" },
    float = true,
    size = "720 600",
    center = true,
})
hl.window_rule({
    name = "suflyor-native-bar",
    match = { title = "^suflyor \\(Slint\\).*$" },
    size = "1280 64",
    move = "20 40",
})
hl.window_rule({
    name = "suflyor-native-question",
    match = { title = "^(Suflyor.*question|Суфлёр.*вопрос)$" },
    size = "480 112",
})
hl.window_rule({
    name = "suflyor-native-tile",
    match = { class = "^suflyor$", title = "^tile.*$" },
    size = "460 360",
})
