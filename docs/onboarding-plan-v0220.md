# Onboarding rework — plan & live state (v0.21.0 done → v0.22.0 onboarding)

_Created 2026-06-20. Survives context compaction. Keep current._

## Released (gh)
- **v0.20.0** (`86ccd3c`) — TTS rename (Озвучка→TTS → AI nav) + OCR install button (Tesseract on stable `ocr-assets` release) + read-aloud sidecar.
- **v0.20.1** (`fb2e263`, Latest) — fix: local AI dead after upgrade (orphaned `llama-server` on :8080). Job Object `KILL_ON_JOB_CLOSE` (children die with parent) + cold-boot owner-aware `switch_local_model`.

## Committed to master (LOCAL, not pushed) — 2026-06-20
User authorised committing («Комитетить разрешаю»); push + `gh release` still wait for «релизь».
- **`6c3b0f3`** feat(backend): components readiness API + base-model/gigaam SSOT + compact/suppress config.
- **`c3a01f9`** feat(ui): «Компоненты» readiness hub (read-only) + compact reader bar + listening mode (v0.21.0).
- **`effb9da`** feat(ui): inline «Установить» for voices + OCR in the Компоненты hub (O2-full first slice).
Gates: per-commit git-gate hook (fmt+clippy both crates) green; backend 332 tests; i18n_guard; release builds + runs. Adversarial reviews: full 6-dimension workflow on the hub (3 minor → 2 fixed: reused-window refresh + gigaam SSOT; 1 accepted: RU-hardcoded names), focused review on the inline-install handler = CLEAN.

## Done on disk, NOT committed/released — awaiting explicit «релизь»
**v0.21.0 — compact reader mode + suppress-tiles.** Version already bumped to 0.21.0 in `slint-experiment/Cargo.toml` + `scripts/slint-installer.nsi`.
- **Compact bar** (`config.compact_bar`, default false): bar chip ▭ (`minimize.svg`) collapses 1200px bar → **340×46 pill** (`tts-status` "TTS" + Stop `stop.svg` + Expand `maximize.svg`).
  - `ui/overlay_bar.slint`: full content wrapped `if !root.compact-bar : VerticalLayout {…}`; new `if root.compact-bar : HorizontalLayout {…}` pill; `min-width: root.compact-bar ? 300px : 720px` (the static 720 was clamping the resize — fixed).
  - `bin/overlay_host.rs`: `apply_bar_size(overlay, compact)` (set_size 340/1200) + `recenter_when_sized(weak, target_w, attempt)` polls until the OS window reaches target width, then centers on the OS-reported primary. `on_compact_toggle_clicked` (flip cfg + save + apply) + startup sync + `on_tts_stop_clicked` (`tts::stop()` + `reset_pause()`).
  - **Centering is correct**: measured 340×46, centered for whatever the OS reports as primary (tool session sees a 1200×1920 portrait → x=430 IS centered there; on the user's real display it centers on their real primary). Earlier "off-center" alarm was a diagnostic-session display mismatch, NOT a bug.
- **Suppress-tiles** ("listening mode", `config.suppress_tiles`, default false): gate = early-return in `src/slint_session.rs::maybe_spawn_auto_tile` — suppresses ONLY auto `TileKind::Ai`; manual F6/F9/PTT, KB, Summary, Error untouched (verified by review). Settings: `settings_panel.slint` DarkCheck "Listening mode — don't show tiles" + desc in Auto-tiles tab; `settings_controller.rs` `on_suppress_tiles_changed` + `set_suppress_tiles(snap.suppress_tiles)` seed; ru.po has both strings.
- **F1 Help** (`ui/help.slint`): added Shift+Alt+1/2/3 rows + section "Озвучка · распознавание · компактный режим". (Hardcoded RU, matches existing help style — no @tr needed.)
- **Gate**: clippy clean both crates, 329 backend + i18n_guard green, release build OK. 2-reviewer adversarial pass clean after fixes.
- **Follow-up nits (not blockers)**: live `tts-status` (🔊 reading / ⏸ paused) wiring from TTS state; pill a11y labels are raw English (intentional).

## NEXT — "Полный онбординг" (user-chosen). v0.22.0. Fixes 3 user pains: stale F1 (partly done), scattered/undocumented installs, overloaded "AI мост" tab.
- **O1 — readiness API ✅ DONE (on disk)**: `overlay-backend/src/components.rs` → `status(cfg) -> Vec<ComponentStatus{kind, installed, detail}>` for Engine / LocalModel / Stt / Voices / Ocr, reusing the SAME checks the install buttons use (`installed_engine_build`, new `local_ai::base_model_present` (4B) + `quality_model_present` (12B), GigaAM `model.int8.onnx` stat, `tts_install::any_voice_installed`, `ocr::is_available`). Detail is locale-neutral ("b9637", "Gemma 4B + 12B", "GigaAM", "Tesseract"), empty when absent. `+ any_core_missing(cfg)` for O3. Pure label helpers unit-tested (3 tests). Module registered in lib.rs. clippy clean.
- **O2 — «Компоненты» Settings section 🟡 FIRST CUT DONE (read-only, on disk)**: new nav tab (tab-index **15**, top of AI group, `update.svg` icon) renders `component-rows` (built in `settings_controller.rs` from `components::status`) — one card per component: status dot (green=installed / grey=absent) + RU name + detail/where-to-install-hint + "установлено/не установлено". Struct `ComponentRow{name,detail,installed,hint}` in settings_panel.slint; imported via `ui::{…ComponentRow…}` in overlay_host.rs + controller `use super`. New @tr: "Components"/"installed"/"not installed"/desc → all in ru.po (i18n_guard green). Gate: clippy + i18n + 24/48 slint tests pass; release built + app runs.
  - **DONE (O2-full slice 1, `effb9da`)**: inline «Установить» for Voices + OCR (the light single-call installers) right in their cards — worker thread + live status + auto-refresh to green; shared busy-state guards concurrency; generic error on failure. `installable` flag on ComponentRow (true only for voices/ocr).
  - **REMAINING (O2-full)**: engine / local-model / STT are still status-only in the hub (their heavy installers — multi-GB downloads, cancel/progress, lifecycle locks — stay in their dedicated panels). Next: add a per-card «Открыть ›» jump (set active-tab to that component's settings tab) so every card is actionable; then **slim "AI мост"** by moving its readiness clutter out (provider/model/vision/Test stay).
  - **NOT yet visually verified live**: the panel compiled (clippy validated every binding/property in the new `.slint`) + i18n + tests pass + the build runs, but the gear is an unreliable synth-click target in this masked/multi-monitor tool session (CLAUDE.md time-sink) — the panel's exact pixels still want the user's eyes (or Slint-MCP at release time). Open Settings → it's the FIRST item under "AI".
- **O3 — readiness dashboard at start**: launch/first-run → if components missing, small «Готовность» panel/wizard step (installed-vs-needed) + «Открыть Компоненты». `wizard.slint` scaffold exists.
- **O4 — wizard step → Компоненты**; keep extending F1 as features land.
- **O5 — process guard → promote to CLAUDE.md**: RULE — _new global shortcut ⇒ update `help.slint`; new on-demand install ⇒ add to Компоненты hub + mention in wizard._ (This is why help/onboarding drifted; enforce it.)

Each O-phase: own 5-layer gate + adversarial review + live smoke. Release v0.22.0 only on «релизь».

## BACKLOG (consolidated 2026-06-20) — single source of truth for what's left

### Onboarding (functional)
- **O2-full ✅ DONE**: jump-buttons (`<commit>` "Открыть jump") — every «Компоненты» card actionable: voices/OCR inline install (`effb9da`), engine/model/STT «Открыть» → their tab (engine/model→AI мост 11, STT→STT 12). `ComponentRow.jump-tab`; 4-way action area. **"Slim AI мост" reclassified → P2 #7 density**: the heavy installers must STAY in AI мост (that's where «Открыть» lands), so it's a layout-declutter (subjective, user's eye), not a removal.
- **O3 — readiness dashboard at launch (NEXT, needs a UX decision)**: `components::any_core_missing(cfg)` is built. The trap: engine/model missing is NORMAL for a cloud-only user (local AI is optional), so an unconditional nag is wrong. Proposed gate: prompt ONLY when `cfg.ai_provider == "local"` (user chose local) AND core missing. Surface options: (a) one-time dismissible info-tile (reuse tile infra + a `readiness_notice_shown` config flag) — least intrusive; (b) auto-open Settings→Компоненты once; (c) mirror `recover_offer.slint` as a small offer window. First-run already auto-opens the wizard (overlay_host.rs:3318). **Recommend (a).** Implement fresh — needs the config flag + startup hook + the anti-annoyance gate, best with a clear head + a quick user nod on aggressiveness.
- **O4** — wizard step → Компоненты; keep extending F1 as features land.
- **O5** — process guard → promote to CLAUDE.md (new shortcut ⇒ update help.slint; new install ⇒ add to Компоненты hub + wizard).

### Design audit (docs/design-review-2026-06-20.md) — remaining
- **#4 emoji → SVG (Rust-side)**: memory category glyphs (📝🔁⚙⭐📌 in `settings_memory.rs:200-205`, prefix every memory row — most visible), the ➕ memory statuses (`settings_memory.rs:76,112`), tile statuses (⏳ etc. in `tile_ask.rs`/`tile_ptt.rs`), bar 🔄 restart chip. NEEDS a style decision: SVG asset vs uniform «•» vs short text-tag vs keep. (Settings + capture-overlay emoji already cleaned in `7c8cf3f`.)
- **P2 polish (subjective, taste + live-verify on user's monitor)**: #6 form label grid (TTS/Interface fixed label column 64-72px); #7 tab density (airy tabs → compact cards, dense → more height/ScrollView); #9 palette empty-state too big (shrink until results); #10 9-10px text → 11-12 for instructions; #11 bar chip click height 22px → 28-30 (NB conflicts with the new 64px bar height — re-measure); #12 tile header overloaded (3-4 persistent actions + overflow); #13 unify spacing to 4/8/12/16 grid. Take ONE at a time with the user's eye.

### Older pending (pre-existing, low-pri)
- `#135` F9/follow-up per-tile streams + lifecycle resets (deferred). `#145` audit minor correctness nits. `#180` P1.7 server-settings export+import preview. `#76` Settings Card/Row titled sections.

## Release protocol
NEVER auto-publish. Build + self-gate + review + live smoke → show user evidence → wait for explicit «релизь» → then commit+push+`gh release`. Accumulate; no marathons. config.json holds live secrets — never print.
