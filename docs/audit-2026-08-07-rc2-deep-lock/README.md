# v0.36.1-rc.2 deep-lock visual audit

Captured on Winbrat at 1024×768, scale factor 1.0, with the Glacier dark
theme. The active provider was Suflyor's managed local AI, deep lock was
enabled, and the transcript was empty. Language changes and compact/full
transitions were performed through the UI; no configuration contents were
read.

The `before-deep-*` and `before-f6-*` images are from the v0.36.1-rc.1 UI-MCP
build. They show the two confirmed defects: the deep state is only an icon, and
F6 reports an empty transcript instead of the unloaded local AI. The
`before-worstcase-*` images are from the intermediate rc.2 candidate at the old
1200/560 px sizes and document the overlap found during adversarial review. The
`after-*` images are from the final v0.36.1-rc.2 candidate UI-MCP build.

Pairs:

- `before-deep-*-full.png` / `after-deep-*-full.png`: full bar in RU and EN.
- `before-deep-*-compact.png` / `after-deep-*-compact.png`: compact bar in RU
  and EN. The corrected compact width is 680 px instead of 500 px so the full
  state label, running-session controls, and Expand never overlap.
- `before-worstcase-ru-*.png` / `after-worstcase-ru-*.png`: the adversarial
  running-session + open-tile state that exposed overlap at 1200/560 px. The
  final 1280/680 px full and compact bars keep `recording`, Pause, the complete
  deep-lock label, Close all, and the right-edge window controls distinct.
- `before-f6-empty-deep-*.png` / `after-f6-empty-deep-*.png`: F6 with an empty
  transcript while deep-locked, in RU and EN.
- `after-manual-tile-empty-deep-en.png`: the separate `+ tile` path shows the
  same deep-lock notice before its empty-transcript precondition.

All screenshots are direct Slint MCP window captures. No personal transcript,
endpoint, key, or desktop content is present.
