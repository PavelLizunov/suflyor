# RC4 Codex reasoning and Vision UI audit

Scope: Settings > AI providers, the Codex account/model/reasoning surface, and
the Vision route/model surface.

## Baseline

The reported RC3 state was captured at 720x600, Russian, 100% scale, Light
Frost, with a real signed-in account but no secrets or answers visible:

- `before-reasoning-ru-720.png`: reasoning metadata was embedded into each
  model name, making the picker hard to scan.
- `before-vision-ru-720.png`: Vision used a separate legacy route list instead
  of the AI-provider catalog.

## RC4 validation environment

- VM: Winbrat, 1280x768 desktop, 100% scale
- Isolated config/profile and synthetic `codex.exe`; no OAuth, API keys,
  account traffic, user transcript, or external inference
- Renderer: Slint `winit-software`
- Exact QA binary: debug `ui-mcp` build with `SLINT_EMIT_DEBUG_INFO=1`
- Binary SHA-256:
  `3645B3027F264392A4E1FF513421FFBB1E50282AEB38AEEE7C88EFC516688BD4`

## RC4 result

- `after-bar-luna-en-1280.png`: the active stack names the selected Luna model.
- `after-reasoning-luna-en-720.png`: model names are clean and reasoning is a
  separate `Low (fastest available)` control.
- `after-reasoning-spark-ru-720.png`: the Russian Spark state uses the clean
  model name and a separate supported reasoning value.
- `after-vision-luna-en-720.png`: image-capable Luna enables and selects
  `Same as text model above`.
- `after-vision-spark-ru-720.png`: text-only Spark disables the same-model
  route with a reason, while the opened Vision provider picker uses the same
  catalog as text AI providers.

Settings was fixed at 720x600 and the bar at 1280x64. No tested control or
explanation was clipped or outside the settings viewport. All 13 application
hotkeys registered in the isolated run. F8 dispatch itself was not invoked
because the synthetic Codex sidecar intentionally implements account/model
metadata only; routing and image filtering are covered by the Rust tests.
