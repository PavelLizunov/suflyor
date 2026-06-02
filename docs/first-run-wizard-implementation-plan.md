# First-Run Wizard — implementation plan + progress

Implements `docs/first-run-wizard-and-readiness-dashboard-concept.md` **Part 1**
(the 7-step guided setup). The readiness-dashboard half (Part 2) already shipped
as the 🩺 Diagnostics tab (#131/#133). Full, anchor-verified design plan from the
design workflow is in the run output (workflow `first-run-wizard-plan`,
run `wf_7e04439d-3a6`); this file is the working checklist + status.

## Decisions (locked)
1. Dedicated frameless `WizardWindow` (mirrors `text_ask.slint`), one step panel
   visible per `step` index (0..6). NOT a Settings mode.
2. Auto-show on FIRST RUN only (no `config.json`) + a "🪄 Запустить мастер
   настройки" button in Settings → Interface.
3. No new overlay-bar icon.
4. MVP: do not persist a last-readiness result.
5. Per-step checks auto-run on entry and REUSE the existing Diagnostics backend
   fns (`ai::test_connection`, `stt::test_connection_backend`,
   `record_mic_blocking`+`rms_dbfs`, `play_tone_and_capture`+`rms_dbfs`).
6. Step-6 local-AI = a button that opens the EXISTING installer.

Level int convention (from Diagnostics): -1 checking · 0 ok · 2 not-configured ·
3 neutral/— · 4 needs-attention. **An error paints level 4 + a warn string but
never blocks Next.**

## Theme tokens (corrected vs the plan)
`theme.slint` has **`Theme.success` / `Theme.warning`** (NOT `ok`/`warn`) and
`Theme.font-mono` ✓. `wizard.slint` uses the corrected names.

## Progress
- [x] STEP 1 — `ui/wizard.slint` (new) — full 7-step component. DONE.
- [x] STEP 2 — `ui/index.slint` import + re-export of `WizardWindow`. DONE.
- [ ] STEP 3 — `overlay_host.rs`: `use ui::{… WizardWindow}` (L62-65) + `wizard`
  slot `Rc<RefCell<Option<WizardWindow>>>` (after the `text_ask` slot ~L2243) +
  `fn apply_scheme_wizard` (after `apply_scheme_text_ask` ~L5260).
- [ ] STEP 4 — `fn wire_wizard_steps(win, cfg, settings_ref, state, tiles,
  overlay_weak, palette_ref, text_ask_ref)` (before `open_wizard`): nav state
  machine (`on_nav_next` advances + auto-runs the new step's check; `on_nav_back`;
  `on_nav_skip` advances WITHOUT a check) + `on_mode_selected` (writes
  `ai_provider`/`stt_provider`/`vision_provider` + `config::save` → creates
  config.json) + the 5 reused checks (each copies the matching diag phase body
  L6767-6857, swapping the weak to `WizardWindow` + `set_diag_*`→`set_*`; AI uses
  `cfg.ai_endpoint(false)` NOT raw base_url; mic wraps `try_acquire_mic`/
  `release_mic`) + `on_stealth_toggled` (mirrors `on_stealth_toggle_clicked`:
  state + `set_global_stealth` + persist + flip every window) + step-7 summary
  fill (read `readiness()` + live `*-detail`, inline in the `nav_next` n==6 arm).
- [ ] STEP 5 — `fn open_wizard(...)` (adapts `open_text_ask` L5269-5342): singleton
  guard → `WizardWindow::new()` → `apply_scheme_wizard` → `set_stealth_on(global_stealth())`
  → `wire_wizard_steps` → wire `on_finished`/`on_cancelled` (hide + drop slot) →
  `present_window_stealth_aware(&win, focus_window)` → store slot.
- [ ] STEP 6 — triggers: (a) `settings_panel.slint` `callback open-wizard-clicked()`
  (~L236) + a "🪄 Run setup wizard" Button in the Interface tab (active-tab==3,
  ~L767); (b) `open_settings` gains a `wizard_ref` param (sig ~L5854 + call-site
  ~L3072/3081) and wires `win.on_open_wizard_clicked(|| open_wizard(...))`;
  (c) first-run capture `let first_run = config_path().map(|p|!p.exists())` BEFORE
  `config::shared()` (~L1396) + a `Timer::single_shot(2200ms, || open_wizard(...))`
  before `overlay.run()` (~L3274) when `first_run`.
- [ ] STEP 7 — flip the wizard window in BOTH stealth toggles (bar chip ~L2685/2736,
  Settings `on_stealth_changed` ~L5999/6037) — the #111-class parity step.
- [ ] STEP 8 — `ru.po`: one msgid/msgstr per new `@tr` (dedupe `Install local AI`/
  `Back`/`Next`/`OK`/`Done`/`Record test` if already present — grep first).
- [ ] STEP 9 — gate: fmt + clippy `-D warnings` + test (both crates) + review-agent.
- [ ] STEP 10 — LIVE computer-use verify (user authorized): rename config.json →
  .bak to force first-run; launch; confirm "first run detected" log; screenshot
  the wizard renders SOLID + centered on primary (RISK 1: DWM invisible); drive
  Back/Skip/Next; confirm AI + mic + sys checks actually run; toggle stealth from
  bar + Settings while the wizard is open and `CopyFromScreen`-check it vanishes
  (RISK 2). Restore config.json.bak. Then commit + push (NO release).

## 3 risks
1. **DWM created-but-invisible** frameless window → SOLID `Theme.bg-surface` (done
   in wizard.slint); verify live FIRST.
2. **Two-path stealth parity hole** (security) → STEP 7 patches BOTH toggles.
3. **Mic contention / panics** → mic guard verbatim; every closure uses
   `let Some(w)=…else{return}` + `if let Ok` + `match` (no unwrap/expect).

## Generated Rust names (kebab→snake)
`ai-level`→`set_ai_level`/`get_ai_level`; `nav-next`→`on_nav_next`/`invoke_nav_next`;
`mode-selected`→`on_mode_selected`; `ai-test-clicked`→`on_ai_test_clicked`/
`invoke_ai_test_clicked`; `stealth-on`→`set_stealth_on`; etc.
