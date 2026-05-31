# Screen-capture + Vision-AI module — implementation plan

**Status:** plan for review (2026-05-31). Not yet started.
**Ask (user):** a Lightshot-style capture module — screenshot the **whole screen**
or a **drag-selected region** — wired to its **own separate AI channel**, so the
*text* pipeline can stay on a local model while *image* understanding goes to a
**different local model, the same one, or a cloud model** (local text LLMs often
can't do vision).

---

## 1. Key architectural decision — a SEPARATE vision channel

The vision endpoint is resolved **independently** of the text endpoint. Text
keeps using `cfg.ai_endpoint(false)`; images use a new `cfg.vision_endpoint()`
that the user points wherever they want:

| `vision_provider` | Resolves to | Use case |
|---|---|---|
| `off` | feature disabled | default until configured |
| `same` | `ai_endpoint(false)` | one model does both (e.g. a cloud Claude) |
| `local` | `vision_local_*` fields | a **2nd local server** on another port running a vision model (Qwen2-VL / MiniCPM-V / LLaVA) while text stays on gemma:8080 |
| `cloud` | `vision_*` fields | cloud vision (Claude / GPT-4o) while text stays local |

This is the core of the request: **text local, vision anywhere**.

## 2. What already exists (reuse — do NOT rebuild)

The Explore pass found the request pipeline is already vision-capable:

- **`overlay-backend/src/ai.rs:109-132`** — `ChatMessage { role, content: MessageContent }`,
  where `MessageContent::Parts(Vec<ContentPart>)` and
  `ContentPart::ImageUrl { image_url: ImageUrl { url } }` with `url =
  "data:image/jpeg;base64,…"`. **The image message shape is already supported.**
- **`ai.rs:135` `stream_chat(base_url, bearer, model, messages, max_tokens) ->
  mpsc::Receiver<AiEvent>`** and **`ai.rs:574 complete(...)`** both take
  `Vec<ChatMessage>` — so an image request streams through the **existing**
  HTTP/SSE/cost/retry client. No new networking code.
- **`config.rs:422 AiEndpoint { base_url, bearer, model, is_local }`** +
  **`config.rs:462 ai_endpoint(prep)`** — mirror this for vision. There is
  already an **`ai_local_vision: bool` (config.rs:65)** flag and `is_local`'s doc
  says it's used to "gate screenshots" — wire into that instead of inventing new
  gating.
- **`overlay_host.rs:92 install_streaming_tile` + `:337 current_streaming` slot**,
  driven by **`runtime.rs:1010 ask_stream_loop`** over the
  **`AiEvent { Start, Delta, Done, Error }`** enum (`ai.rs:100`). A vision answer
  streams into a tile through the same machinery — see §6 for the separate-sink
  note.
- **`win32.rs`** already has `enum_monitors() -> Vec<MonitorRect>` (`:246`),
  `MonitorRect{left,top,right,bottom,is_primary}` (`:216`), `get_window_rect`
  (`:335`), `set_stealth(hwnd,on)` = `WDA_EXCLUDEFROMCAPTURE` (`:172`), and
  imports `Win32_Graphics_Gdi` (windows 0.62). The **capture (BitBlt)** function
  is the one Win32 piece NOT yet present.
- **`settings_panel.slint:1030-1283`** — the AI provider selector (cloud/local
  combo + base_url/bearer/model fields + Test button + `ai-*` callbacks, incl. an
  `ai-local-vision` DarkCheck at `:1205`). Clone this block for a vision section.

**Net:** ~60% of a vision feature is plumbing that already exists. New code =
capture + crop + encode, the `vision_endpoint` resolver, a thin `vision.rs`, the
selection overlay, a hotkey, and a Settings section.

## 3. New modules / files

| File | Crate | Responsibility |
|---|---|---|
| `overlay-backend/src/config.rs` (edit) | backend | add `vision_*` fields + `vision_endpoint()` resolver + tests |
| `overlay-backend/src/vision.rs` (new) | backend | build the image `Vec<ChatMessage>` (system prompt + text + base64 image part) and call `ai::stream_chat` / `complete`; expose `test_connection` reusing `ai::test_connection` |
| `slint-experiment/src/capture.rs` (new) | slint | Win32 capture: virtual-desktop bounds, full-monitor BitBlt, crop, JPEG-encode → bytes; cursor tracking for region select |
| `slint-experiment/src/win32.rs` (edit) | slint | add `capture_rect(left,top,w,h) -> Vec<u8>` (BitBlt + GetDIBits) + `virtual_screen_bounds()` + `cursor_pos()` |
| `slint-experiment/ui/capture_overlay.slint` (new) | slint | fullscreen frozen-image selection UI (Lightshot drag) — Phase 3 |
| `slint-experiment/src/bin/overlay_host.rs` (edit) | slint | new capture hotkey + bar 📷 chip + wire capture→vision→tile |
| `slint-experiment/ui/settings_panel.slint` (edit) | slint | "Vision" section mirroring the AI-provider block |
| `translations/ru/.../slint-replay.po` (edit) | slint | RU strings for new UI |

**New deps:** `base64` (encode data-URI) + a JPEG encoder. Prefer the small
`image` crate (PNG/JPEG/crop/resize) in **overlay-backend** (encode is pure data)
or, to avoid a heavy dep, encode JPEG in `capture.rs` via the `zune-jpeg`/`jpeg-encoder`
crate. `windows 0.62 Win32_Graphics_Gdi` is already enabled (has `BitBlt`,
`CreateCompatibleDC/Bitmap`, `GetDIBits`, `GetDC`, `SelectObject`, `DeleteObject`).

## 4. Capture mechanic — "freeze-first" (Lightshot model)

Lightshot grabs a full screenshot FIRST, shows it frozen, and lets you select on
the still image. That avoids the "my own selection overlay ends up in the
screenshot" problem entirely. Flow:

```
hotkey/chip
   │
   ▼
1. HIDE our own windows (bar + tiles) via ShowWindow(SW_HIDE)   ← keep app out of the shot
2. BitBlt the WHOLE virtual desktop → frozen RGB buffer          ← capture.rs
3. RESTORE our windows
   │
   ├── Phase 2 (full-monitor): crop frozen buf to the monitor under the cursor → done
   │
   └── Phase 3 (region): show capture_overlay.slint fullscreen, stealthed,
       displaying the frozen buffer; user drags a rectangle; Esc cancels;
       Enter / double-click = whole monitor; on mouse-up → crop frozen buf
   │
   ▼
4. JPEG-encode the crop (downscale long edge to ~1568px — vision models cap res)
5. base64 → vision.rs builds ChatMessage with the image part
6. spawn a tile + stream the answer via the vision channel (§6)
```

**Multi-monitor / DPI (the user has a portrait 1200×1920 secondary at x=-1200):**
- Use **virtual-screen bounds** from `GetSystemMetrics(SM_XVIRTUALSCREEN /
  SM_YVIRTUALSCREEN / SM_CXVIRTUALSCREEN / SM_CYVIRTUALSCREEN)` — origin is
  **negative**, never assume (0,0).
- Drive selection coordinates from **Win32 `GetCursorPos` (physical, virtual-screen
  space)**, NOT Slint logical mouse coords. The overlay renders the frozen image +
  the moving rectangle for *display*; the authoritative crop rect comes from the
  physical cursor. This sidesteps Slint's per-window scale-factor mismatch when one
  window spans two different-DPI monitors.
- **Process DPI awareness must be Per-Monitor-V2** (check the manifest / startup;
  set if absent) so BitBlt grabs true physical pixels.
- **Phase-3 v1 limitation:** if a region drag spans two monitors of *different*
  DPI, the displayed rectangle may look slightly off even though the crop is
  pixel-exact. Acceptable for v1; note it in the UI.

## 5. The `vision_endpoint` resolver (config.rs)

```rust
pub fn vision_endpoint(&self) -> Option<AiEndpoint> {
    match self.vision_provider.as_str() {
        "same"  => Some(self.ai_endpoint(false)),
        "cloud" => Some(AiEndpoint { base_url: self.vision_base_url.clone(),
                                     bearer: self.vision_bearer.clone(),
                                     model: self.vision_model.clone(),
                                     is_local: false }),
        "local" => Some(AiEndpoint { base_url: self.vision_local_base_url.clone(),
                                     bearer: self.vision_local_bearer.clone(),
                                     model: self.vision_local_model.clone(),
                                     is_local: true }),
        _ /* "off" */ => None,
    }
}
```
Unit-test all four branches (mirrors the existing `ai_endpoint` tests).

## 6. Streaming the vision answer into a tile

Two options; recommend **B**:

- **A.** Reuse `install_streaming_tile` + the shared `current_streaming` slot.
  Simple, but a vision answer and a live text answer would fight over the slot
  (the exact class behind bug **#135**).
- **B (recommended).** Give vision its **own** streaming sink (mirror the existing
  `PttStreamSink`, overlay_host.rs:175) and its own generation counter, so a vision
  tile and a text tile can stream **concurrently** — which is the whole point of a
  "separate channel." Vision tiles are created with `TileWindow::new()` exactly
  like F9, just fed from `vision.rs`.

Cost: if `endpoint.is_local` → force `$0` (existing pattern, runtime.rs cost
guard). Cloud vision uses image-token pricing — `cost_microcents` (`ai.rs:422`)
may not know vision models; either add their prices or show "cloud vision (cost
not metered)" for v1.

## 7. Privacy / security (this app screen-shares during interviews)

- **Egress warning:** when `vision_provider = cloud`, the screenshot **leaves the
  machine**. Show an explicit one-time warning in Settings + a 🌐 hint on the
  capture chip. Local/`same`-local = no egress.
- **Stealth:** the `capture_overlay` window gets `set_stealth(true)`
  (`WDA_EXCLUDEFROMCAPTURE`) so the meeting's screen-capture never sees the frozen
  selection UI.
- **Never log image bytes.** Log only dimensions + endpoint kind (local/cloud),
  never the base64 or the base_url (reuse tonight's redaction: vision errors must
  route through `classify_ai_error`, same as the text tile path — the
  `AiEvent::Error { message: format!("{e:#}") }` at `ai.rs:147` is only safe
  because the tile classifies it; keep that invariant for vision).
- **Hide-app-during-freeze** (§4 step 1) doubles as privacy: our own tiles never
  land in a screenshot that may go to the cloud.

## 8. Phasing (each phase independently shippable + visually verifiable)

| Phase | Scope | Verifiable by |
|---|---|---|
| **V1** | config `vision_*` fields + `vision_endpoint()` resolver + `vision.rs` (build image message, call `ai::stream_chat`) + unit tests. **No UI.** | `cargo test` (resolver branches); a hidden dev hotkey that sends a fixed test PNG to the endpoint and logs the answer |
| **V2** | `win32::capture_rect` + `capture.rs` full-virtual BitBlt + **full-monitor** capture (monitor under cursor) on a new hotkey (**F8**) → JPEG → vision channel → **tile** with the streamed answer. Hide-app-during-freeze. | live: F8 over a screen with text → tile shows the model's reading of it |
| **V3** | `capture_overlay.slint` — frozen-image **drag-region** selection (Lightshot), Esc cancel, Enter/dbl-click = full monitor, dimensions readout, stealthed overlay. | live: drag a box → only that region goes to vision |
| **V4** | Settings "Vision" section (provider combo + cloud/local fields + model dropdown via `list_models` + **Test** button + egress warning), mirroring `settings_panel.slint:1030-1283`; RU `.po`. | live: configure a 2nd local vision server, Test ✓, capture routes there |
| **V5** | polish: bar 📷 chip, cost metering for cloud vision, per-monitor-DPI drag correctness, retina downscale, config-driven prompt presets ("read it", "solve it", "explain") | full layer-5 smoke on both monitor orientations |

V1+V2 deliver a **working screenshot→vision→tile** path quickly; V3 adds the
Lightshot region UX; V4 exposes the separate-channel config; V5 is refinement.

## 9. Open questions for the user (decide before V2/V4)

1. **Default vision provider?** `off` (opt-in) vs `same` (reuse text if it's a
   cloud Claude). Recommend `off` until configured, with a Settings nudge.
2. **Capture hotkey?** Proposed **F8** (free; F3/F4/F6/F7/F9 taken). PrtScn is
   risky (OS/Snip&Sketch may grab it). OK with F8?
3. **Prompt on capture:** one fixed prompt, a small preset menu ("прочитай /
   реши / объясни"), or a quick text box before sending? v1 = one configurable
   default.
4. **2nd local vision server** — will you run one (e.g. llama.cpp/Ollama with
   Qwen2-VL on `:8082`)? That decides whether V4's `local` branch is the primary
   target vs `cloud`.
5. **Image size cap** — downscale long edge to 1568px (Claude's tile limit) by
   default? Bigger = more detail + more tokens/cost.

## 10. Effort estimate

V1 ≈ 0.5 day · V2 ≈ 1 day (BitBlt + DPI) · V3 ≈ 1.5 days (selection overlay is
the hard UI) · V4 ≈ 0.5 day · V5 ≈ 0.5 day. ~4 days total; V1+V2 (a usable
full-screen vision shot) ≈ 1.5 days.
