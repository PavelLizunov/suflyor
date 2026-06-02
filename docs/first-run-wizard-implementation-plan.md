# First-Run Wizard — RESUME DOC (self-contained)

> **To resume after a context compaction:** "Continue the first-run wizard per
> `docs/first-run-wizard-implementation-plan.md`." Everything needed is in THIS
> file. The user explicitly **authorized `computer-use` AND `workflows` for this
> task** (needed for the Layer-5 live visual verification of the new window).

Implements `docs/first-run-wizard-and-readiness-dashboard-concept.md` **Part 1**
(the 7-step guided setup). Part 2 (readiness dashboard) already shipped as the 🩺
Diagnostics tab (#131/#133).

---

## ✅ COMPLETE (commit `832a64b`, pushed to master)
The whole plan (STEP 3–10) shipped in `832a64b`. Live-verified end-to-end via
computer-use + `CopyFromScreen`: wizard renders SOLID (RISK 1 disproven), step
counter + RU ok, AI check ran live (HTTP 200), Next/Skip work, **stealth parity
round-trip** (wizard vanishes from `CopyFromScreen` when stealth on, reappears
off), summary **secret-free** (no LAN IP — the review-agent's BLOCKER fix + a
skip-refill fix both verified in the exact skip-AI scenario). The everything
below is kept as the historical implementation record.

## STATUS (original — at commit `ee62f7f`)
**DONE + committed/pushed:**
- `slint-experiment/ui/wizard.slint` — the full 7-step `WizardWindow` UI. Compiles
  clean under `clippy -D warnings`. (Window shell mirrors `text_ask.slint`.)
- `slint-experiment/ui/index.slint` — `import { WizardWindow } from "wizard.slint";`
  (after the TextAskWindow import) + `WizardWindow,` in the `export {…}` block.
- This doc.

**REMAINING (all in `slint-experiment/src/bin/overlay_host.rs` unless noted):**
STEP 3 (use+slot+scheme) · STEP 4 (`wire_wizard_steps`) · STEP 5 (`open_wizard`) ·
STEP 6 (triggers: Settings button + first-run auto-show + `open_settings` param) ·
STEP 7 (flip wizard in BOTH stealth toggles) · STEP 8 (`ru.po`) · STEP 9 (gate) ·
STEP 10 (live computer-use verify). Tree is clean at `ee62f7f`.

Because the crate **denies dead-code**, STEPs 3-7 must land as ONE green unit
(slot/helpers are unused until a trigger calls `open_wizard`). Do them together,
then gate, then commit.

---

## DECISIONS (locked)
1. Dedicated frameless `WizardWindow` (mirrors `text_ask.slint`); one step panel
   visible per `step` (0..6).
2. Auto-show on FIRST RUN only (no `config.json`) + "🪄 Запустить мастер настройки"
   button in Settings → Interface.
3. No new overlay-bar icon.
4. MVP: do not persist a last-readiness result.
5. Per-step checks auto-run on entering the step and REUSE the existing Diagnostics
   backend fns.
6. Step-6 local-AI = a button opening the EXISTING installer.

Level int convention (from Diagnostics): `-1` checking · `0` ok · `2` not-configured
· `3` neutral/— · `4` needs-attention. **An error paints level 4 + a warn string
but NEVER blocks Next.**

## THEME TOKENS (corrected — the design plan was wrong here)
`theme.slint` has **`Theme.success` / `Theme.warning`** (NOT `ok`/`warn`) and
`Theme.font-mono` ✓. `wizard.slint` already uses the corrected names.

## GENERATED RUST NAMES (Slint kebab→snake)
`step`→`get_step`/`set_step`; `ai-level`→`get_ai_level`/`set_ai_level`;
`ai-detail`→`set_ai_detail`; `mode`→`get_mode`/`set_mode`; `stealth-on`→`set_stealth_on`;
`summary-ai`→`set_summary_ai` (… `_stt`/`_mic`/`_sys`); callbacks `nav-next`→`on_nav_next`
/`invoke_nav_next`, `ai-test-clicked`→`on_ai_test_clicked`/`invoke_ai_test_clicked`,
`mode-selected`→`on_mode_selected`, `stealth-toggled`→`on_stealth_toggled`,
`install-local-clicked`→`on_install_local_clicked`, `open-diagnostics`→`on_open_diagnostics`,
`finished`→`on_finished`, `cancelled`→`on_cancelled`, `nav-back`→`on_nav_back`,
`nav-skip`→`on_nav_skip`, `stt-test-clicked`→`on_stt_test_clicked`,
`mic-test-clicked`→`on_mic_test_clicked`, `sys-test-clicked`→`on_sys_test_clicked`.

## BACKEND FNS TO REUSE (confirm exact signatures by reading the diag handler
`overlay_host.rs` ~L6744-6865 — that 3-phase body is THE template to copy):
- `overlay_backend::ai::test_connection(base, bearer, model)` → async `Result<String>`.
  Feed from `cfg.ai_endpoint(false)` (resolver) — **NOT** raw `ai_base_url` (security).
- `overlay_backend::stt::test_connection_backend(&backend)` → async `Result<String>`;
  `backend = cfg.read().stt_backend()`.
- `overlay_backend::audio::record_mic_blocking(3000, device)` → `Result<Vec<i16>>`;
  `device = cfg.read().mic_device.clone()`. **Wrap in `try_acquire_mic()`/`release_mic()`**.
- `overlay_backend::audio::rms_dbfs(&samples)` → `f32`.
- system-audio: copy whatever the diag "Проверить всё" sys phase calls (the plan
  named `play_tone_and_capture(device)` — **VERIFY the real fn name** in the diag
  body before using; `device = cfg.read().system_audio_device.clone()`).
- `cfg.read().readiness()` → `ReadinessReport { ai, stt, mic, sys, stealth_on }` (bools).
- config write: `c.ai_provider`/`c.stt_provider`/`c.vision_provider` (String),
  `overlay_backend::config::save(&c)`, `overlay_backend::config::config_path()`.

## ANCHORS (current as of `ee62f7f`)
`use ui::{…}` L62-65 · `text_ask` slot L2243 · bar stealth toggle
`on_stealth_toggle_clicked`: clone-set ~L2685, text-ask flip ~L2736 · `cfg =
config::shared()` L1396 · auto-start timer L3259-3272 · `overlay.run()` L3274 ·
Settings call-site `open_settings(…)` L3073 (clones ~L3071-72) · `apply_scheme_text_ask`
ends L5260 · `open_text_ask` L5262-5342 (the template for `open_wizard`) ·
`open_settings` signature ends L5854 (`palette_ref` is the last param) · Settings
`on_stealth_changed`: clones ~L5999-6000, text-ask flip ~L6033 · diag
`on_diagnostics_check_all_clicked` L6744-6865 · `present_window_stealth_aware` L5122.
`settings_panel.slint`: window-scope callbacks ~L236, Interface tab (active-tab==3)
~L760-767. `index.slint`: done. `config.rs`: `config_path()` L1941, `load()` L1948,
`shared()` L2077.
> NOTE: these were captured by the design workflow on `ee62f7f`. If `git log`
> shows commits after `ee62f7f` touching `overlay_host.rs`, re-grep the anchors
> (search for `on_stealth_toggle_clicked`, `fn open_text_ask`, `fn open_settings`,
> `on_diagnostics_check_all_clicked`, `let cfg = config::shared`, `overlay.run()`).

---

## STEP 3 — use + slot + scheme helper
**3a** L62-65 — add `WizardWindow` to the `use ui::{…}` list (alphabetical-ish):
`TextAskWindow, TileWindow, WizardWindow,`.
**3b** after L2243 (the `text_ask` slot):
```rust
    // First-run wizard window, created on demand like text_ask / palette.
    let wizard: Rc<RefCell<Option<WizardWindow>>> = Rc::new(RefCell::new(None));
```
**3c** after L5260 (`apply_scheme_text_ask`):
```rust
fn apply_scheme_wizard(w: &WizardWindow, scheme: i32) {
    w.global::<ui::Theme>().set_scheme(clamp_scheme(scheme));
}
```

## STEP 4 — `wire_wizard_steps` (place right before `open_wizard`, i.e. after
`apply_scheme_wizard` / before `open_text_ask`, or after open_text_ask — any free-fn
spot). COPY each check body from the diag handler L6744-6865, swapping the weak to
`WizardWindow` + `set_diag_*`→`set_*`. Every closure uses `let Some(w)=…else{return}`
+ `if let Ok` + `match` — ZERO `unwrap`/`expect` (clippy-deny).
```rust
#[allow(clippy::too_many_arguments)]
fn wire_wizard_steps(
    win: &WizardWindow,
    cfg: &overlay_backend::config::SharedConfig,
    state: &slint_replay::app_state::SharedState,
    tiles: &TileWindows,
    overlay_weak: &slint::Weak<OverlayBarWindow>,
    palette_ref: &Rc<RefCell<Option<PaletteWindow>>>,
    text_ask_ref: &Rc<RefCell<Option<TextAskWindow>>>,
    settings_ref: &Rc<RefCell<Option<SettingsWindow>>>,
) {
    // --- nav-next: advance + auto-run the NEW step's check; fill summary at step 7 ---
    {
        let weak = win.as_weak();
        let cfg_c = cfg.clone();
        win.on_nav_next(move || {
            let Some(w) = weak.upgrade() else { return };
            let n = (w.get_step() + 1).min(6);
            w.set_step(n);
            match n {
                1 => w.invoke_ai_test_clicked(),
                2 => w.invoke_stt_test_clicked(),
                3 => w.invoke_mic_test_clicked(),
                4 => w.invoke_sys_test_clicked(),
                6 => {
                    // Summary refill — readiness() is pure config (safe on UI thread).
                    // Prefer the live *-detail the checks already painted; else fall
                    // back to a configured/not-configured word from readiness().
                    let r = cfg_c.read().readiness();
                    let pick = |detail: slint::SharedString, ok: bool| -> slint::SharedString {
                        if !detail.is_empty() { detail }
                        else if ok { slint::SharedString::from("configured") }
                        else { slint::SharedString::from("—") }
                    };
                    w.set_summary_ai(pick(w.get_ai_detail(), r.ai));
                    w.set_summary_stt(pick(w.get_stt_detail(), r.stt));
                    w.set_summary_mic(pick(w.get_mic_detail(), r.mic));
                    w.set_summary_sys(pick(w.get_sys_detail(), r.sys));
                }
                _ => {}
            }
        });
    }
    { let weak = win.as_weak(); win.on_nav_back(move || {
        if let Some(w) = weak.upgrade() { w.set_step((w.get_step() - 1).max(0)); } }); }
    { let weak = win.as_weak(); win.on_nav_skip(move || {
        if let Some(w) = weak.upgrade() { w.set_step((w.get_step() + 1).min(6)); } }); }

    // --- Step 1: mode → write provider fields + save (this CREATES config.json) ---
    {
        let cfg_c = cfg.clone();
        win.on_mode_selected(move |m| {
            let mut c = cfg_c.write();
            match m {
                0 => { c.ai_provider = "cloud".into(); c.stt_provider = "cloud".into();  c.vision_provider = "cloud".into(); }
                1 => { c.ai_provider = "local".into(); c.stt_provider = "whisper".into(); c.vision_provider = "local".into(); }
                _ => {} // Mixed: leave as-is
            }
            let _ = overlay_backend::config::save(&c);
        });
    }

    // --- Step 2: AI — REUSE ai::test_connection via the RESOLVER (security) ---
    {
        let cfg_c = cfg.clone(); let weak = win.as_weak();
        win.on_ai_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_ai_level(-1); w.set_ai_detail(SharedString::from(""));
            let (base, bearer, model) = { let c = cfg_c.read(); let e = c.ai_endpoint(false); (e.base_url, e.bearer, e.model) };
            let wr = w.as_weak();
            std::thread::spawn(move || {
                let (lvl, msg): (i32, String) = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(rt) => match rt.block_on(overlay_backend::ai::test_connection(base, bearer, model)) {
                        Ok(s) => (0, format!("[ok] {s}")),
                        Err(e) => (4, format!("[err] {e:#}").chars().take(80).collect()),
                    },
                    Err(e) => (4, format!("[err] runtime: {e}")),
                };
                let _ = slint::invoke_from_event_loop(move || { if let Some(w) = wr.upgrade() { w.set_ai_level(lvl); w.set_ai_detail(SharedString::from(msg)); } });
            });
        });
    }

    // --- Step 3: STT — REUSE stt::test_connection_backend ---
    {
        let cfg_c = cfg.clone(); let weak = win.as_weak();
        win.on_stt_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_stt_level(-1); w.set_stt_detail(SharedString::from(""));
            let backend = cfg_c.read().stt_backend(); let wr = w.as_weak();
            std::thread::spawn(move || {
                let (lvl, msg): (i32, String) = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(rt) => match rt.block_on(overlay_backend::stt::test_connection_backend(&backend)) {
                        Ok(s) => (0, format!("[ok] {s}")),
                        Err(e) => (4, format!("[err] {e:#}").chars().take(80).collect()),
                    },
                    Err(e) => (4, format!("[err] runtime: {e}")),
                };
                let _ = slint::invoke_from_event_loop(move || { if let Some(w) = wr.upgrade() { w.set_stt_level(lvl); w.set_stt_detail(SharedString::from(msg)); } });
            });
        });
    }

    // --- Step 4: mic — REUSE record_mic_blocking + rms_dbfs WITH the single-mic guard ---
    {
        let cfg_c = cfg.clone(); let weak = win.as_weak();
        win.on_mic_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_mic_level(-1); w.set_mic_detail(SharedString::from("recording 3s…"));
            let dev = cfg_c.read().mic_device.clone(); let wr = w.as_weak();
            std::thread::spawn(move || {
                let (lvl, msg): (i32, String) = if !try_acquire_mic() {
                    (4, "[!] mic busy — close PTT / dictation and retry".to_string())
                } else {
                    let r = overlay_backend::audio::record_mic_blocking(3000, dev); release_mic();
                    match r {
                        Ok(s) if s.is_empty() => (4, "[!] no audio captured".to_string()),
                        Ok(s) => { let d = overlay_backend::audio::rms_dbfs(&s);
                            if d >= -45.0 { (0, format!("[ok] heard you ({d:.0} dBFS)")) }
                            else { (0, format!("[ok] capture works · quiet ({d:.0} dBFS)")) } }
                        Err(e) => (4, format!("[err] {e}").chars().take(80).collect()),
                    }
                };
                let _ = slint::invoke_from_event_loop(move || { if let Some(w) = wr.upgrade() { w.set_mic_level(lvl); w.set_mic_detail(SharedString::from(msg)); } });
            });
        });
    }

    // --- Step 5: system audio — REUSE the diag sys-phase fn (verify its real name!) ---
    {
        let cfg_c = cfg.clone(); let weak = win.as_weak();
        win.on_sys_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_sys_level(-1); w.set_sys_detail(SharedString::from(""));
            let dev = cfg_c.read().system_audio_device.clone(); let wr = w.as_weak();
            std::thread::spawn(move || {
                // COPY the exact body the diag check-all uses for system audio
                // (L6839-6857 area). Placeholder name play_tone_and_capture — confirm.
                let (lvl, msg): (i32, String) = match overlay_backend::audio::play_tone_and_capture(dev) {
                    Ok(s) => { let d = overlay_backend::audio::rms_dbfs(&s);
                        if d > -60.0 { (0, format!("[ok] loopback heard the test tone ({d:.0} dBFS)")) }
                        else { (4, "[!] test tone not captured — output device ≠ loopback source?".to_string()) } }
                    Err(e) => (4, format!("[err] {e}").chars().take(80).collect()),
                };
                let _ = slint::invoke_from_event_loop(move || { if let Some(w) = wr.upgrade() { w.set_sys_level(lvl); w.set_sys_detail(SharedString::from(msg)); } });
            });
        });
    }

    // --- Step 2/6: Install local AI — open the EXISTING installer (DECISION 6) ---
    {
        let set = settings_ref.clone();
        win.on_install_local_clicked(move || {
            // Easiest reuse: open Settings (where the installer button + progress live).
            // If a direct trigger exists (settings.invoke_install_local_ai_clicked), call it.
            if let Some(sw) = set.borrow().as_ref() { let _ = sw.show(); }
        });
    }

    // --- Step 6: stealth — call the SAME global stealth path as on_stealth_toggle_clicked ---
    {
        let state_c = state.clone(); let tiles_c = tiles.clone(); let ow = overlay_weak.clone();
        let pal = palette_ref.clone(); let ta = text_ask_ref.clone(); let set = settings_ref.clone();
        let cfg_c = cfg.clone();
        win.on_stealth_toggled(move |on| {
            { let mut st = match state_c.lock() { Ok(g) => g, Err(p) => p.into_inner() }; st.stealth = on; }
            set_global_stealth(on);
            { let mut c = cfg_c.write(); c.stealth_enabled = on; let _ = config::save(&c); }
            if let Some(o) = ow.upgrade() { o.set_stealth_active(on); if let Ok(h) = grab_hwnd(o.window()) { let _ = set_stealth(h, on); let _ = set_skip_taskbar(h, on); } }
            for t in tiles_c.borrow().iter() { if let Ok(h) = grab_hwnd(t.window()) { let _ = set_stealth(h, on); } }
            if let Some(p) = pal.borrow().as_ref() { if let Ok(h) = grab_hwnd(p.window()) { let _ = set_stealth(h, on); } }
            if let Some(t) = ta.borrow().as_ref() { if let Ok(h) = grab_hwnd(t.window()) { let _ = set_stealth(h, on); } }
            if let Some(sw) = set.borrow().as_ref() { if let Ok(h) = grab_hwnd(sw.window()) { let _ = set_stealth(h, on); } }
            // The wizard window itself is flipped by STEP 7 (so a same-tick toggle is covered).
        });
    }

    // --- Step 7: Open diagnostics → open Settings (user navigates to the 🩺 tab) ---
    {
        let ow = overlay_weak.clone();
        win.on_open_diagnostics(move || { if let Some(o) = ow.upgrade() { o.invoke_open_settings_clicked(); } });
    }
}
```
> **VERIFY while typing:** (a) the real system-audio fn name from the diag body
> (placeholder `play_tone_and_capture`); (b) `ai_endpoint(false)` returns a struct
> with `.base_url/.bearer/.model` (it does — used at the diag AI phase); (c)
> `set_skip_taskbar` + `grab_hwnd` + `set_stealth` + `set_global_stealth` are in
> scope (they are — used by `on_stealth_toggle_clicked`); (d) `stt_backend()` exists
> on Config; (e) `SharedString` is imported (it is, used everywhere).

## STEP 5 — `open_wizard` (adapt `open_text_ask` L5262-5342)
```rust
#[allow(clippy::too_many_arguments)]
fn open_wizard(
    slot_ref: &Rc<RefCell<Option<WizardWindow>>>,
    cfg: &overlay_backend::config::SharedConfig,
    state: &slint_replay::app_state::SharedState,
    tiles: &TileWindows,
    overlay_weak: &slint::Weak<OverlayBarWindow>,
    palette_ref: &Rc<RefCell<Option<PaletteWindow>>>,
    text_ask_ref: &Rc<RefCell<Option<TextAskWindow>>>,
    settings_ref: &Rc<RefCell<Option<SettingsWindow>>>,
) {
    {
        let slot = slot_ref.borrow();
        if let Some(existing) = slot.as_ref() {
            let _ = existing.show();
            if let Ok(hwnd) = grab_hwnd(existing.window()) { focus_window(hwnd); }
            return;
        }
    }
    let win = match WizardWindow::new() {
        Ok(w) => w,
        Err(e) => { eprintln!("[overlay-host] WizardWindow::new failed: {e}"); return; }
    };
    apply_scheme_wizard(&win, global_scheme());
    win.set_stealth_on(global_stealth());
    wire_wizard_steps(&win, cfg, state, tiles, overlay_weak, palette_ref, text_ask_ref, settings_ref);
    { let weak = win.as_weak(); let slot = slot_ref.clone();
      win.on_finished(move || { if let Some(w) = weak.upgrade() { let _ = w.hide(); } *slot.borrow_mut() = None; }); }
    { let weak = win.as_weak(); let slot = slot_ref.clone();
      win.on_cancelled(move || { if let Some(w) = weak.upgrade() { let _ = w.hide(); } *slot.borrow_mut() = None; }); }
    present_window_stealth_aware(&win, |hwnd| { focus_window(hwnd); });
    *slot_ref.borrow_mut() = Some(win);
}
```
> VERIFY: `global_scheme()` + `global_stealth()` + `focus_window` are in scope
> (used by open_text_ask / present_window_stealth_aware). `present_window_stealth_aware`
> signature: copy the exact call shape from `open_text_ask` (L5262-5342).

## STEP 6 — triggers
**6a `settings_panel.slint`:** add `callback open-wizard-clicked();` at window scope
(~L236, near other callbacks) + a button in the Interface tab (`active-tab == 3`,
~L767, after the colour-scheme row):
```slint
    HorizontalLayout { alignment: start;
        Button { text: @tr("🪄 Run setup wizard"); clicked => { root.open-wizard-clicked(); } }
    }
```
**6b `overlay_host.rs` `open_settings`:** add a `wizard_ref: &Rc<RefCell<Option<WizardWindow>>>`
param (after `palette_ref`, sig ~L5854) and, inside the body, wire:
```rust
    { let wz = wizard_ref.clone(); let cfg_w = cfg.clone(); let st = settings_ref.clone();
      let state_w = state.clone(); let tiles_w = tiles_ref.clone(); let ow = overlay_weak.clone();
      let pal = palette_ref.clone(); let ta = text_ask_ref.clone();
      win.on_open_wizard_clicked(move || { open_wizard(&wz, &cfg_w, &state_w, &tiles_w, &ow, &pal, &ta, &st); }); }
```
> `open_settings`'s local param names: check them (the call-site passes `state`,
> `settings_ref`, `tiles_ref`, `cfg`, `overlay_weak`, `text_ask_ref`, `palette_ref`).
> Use whatever names are in scope inside `open_settings`.
**6b call-site** L3073: add `let wizard_for_settings = wizard.clone();` by the other
clones (~L3071) and `&wizard_for_settings,` as the new last arg.
**6c first-run auto-show:** BEFORE `let cfg = config::shared();` (L1396):
```rust
    let first_run = overlay_backend::config::config_path().map(|p| !p.exists()).unwrap_or(false);
```
Then just BEFORE `overlay.run()` (L3274), after the auto-start timer:
```rust
    if first_run {
        eprintln!("[overlay-host] first run detected — auto-opening setup wizard");
        let wz = wizard.clone(); let cfg_w = cfg.clone(); let st = settings.clone();
        let state_w = state.clone(); let tiles_w = tiles.clone(); let ow = overlay.as_weak();
        let pal = palette.clone(); let ta = text_ask.clone();
        Timer::single_shot(Duration::from_millis(2200), move || {
            open_wizard(&wz, &cfg_w, &state_w, &tiles_w, &ow, &pal, &ta, &st);
        });
    }
```
> Confirm the in-scope names at L3273: `wizard`, `cfg`, `settings`, `state`, `tiles`,
> `overlay`, `palette`, `text_ask`. `Timer`/`Duration` already imported (used L3261).

## STEP 7 — flip the WIZARD window in BOTH stealth toggles (the #111-class parity)
**7a bar chip** `on_stealth_toggle_clicked`: clone `let wizard_for_stealth = wizard.clone();`
by the `text_ask_for_stealth` clone (~L2685); flip block after the text-ask flip (~L2736):
```rust
    if let Some(w) = wizard_for_stealth.borrow().as_ref() {
        if let Ok(hwnd) = grab_hwnd(w.window()) { let _ = set_stealth(hwnd, new_stealth); }
        w.set_stealth_on(new_stealth);
    }
```
**7b Settings** `on_stealth_changed` (inside `open_settings`): clone
`let wizard_st = wizard_ref.clone();` (~L5999); flip block by the `text_ask_st` flip (~L6033):
```rust
    if let Some(w) = wizard_st.borrow().as_ref() {
        if let Ok(hwnd) = grab_hwnd(w.window()) { let _ = set_stealth(hwnd, on); }
        w.set_stealth_on(on);
    }
```

## STEP 8 — `ru.po` (append; one msgid/msgstr per NEW `@tr` literal in wizard.slint +
the Settings button "🪄 Run setup wizard"). **Grep first** — `Back`/`Next`/`OK`/
`Done`/`Install local AI`/`Record test`/`AI`/`Microphone`/`Speech recognition (STT)`/
`System audio`/`Not configured`/`Needs attention`/`checking…` may already exist;
gettext keys are file-global, a DUPLICATE msgid is a warning + the 2nd is ignored.
New strings to ensure exist (RU on the right):
```
Setup wizard → Мастер настройки
🪄 Run setup wizard → 🪄 Запустить мастер настройки
Step {} of 7 → Шаг {} из 7
Choose how overlay-mvp runs → Выберите режим работы overlay-mvp
Cloud — Groq STT + Claude bridge (fast start, needs keys + internet) → Cloud — Groq STT + Claude bridge (быстрый старт, нужны ключи и интернет)
Fully local — local AI + local STT (nothing leaves the machine, needs models) → Полностью локально — локальный AI + локальный STT (ничего не уходит наружу, нужны модели)
Mixed — e.g. local STT + cloud AI (configure manually) → Смешанно — например локальный STT + cloud AI (настроить вручную)
Now: cloud recognition + cloud answers. → Сейчас: облачное распознавание + облачные ответы.
Now: local recognition + local answers. Audio stays on this machine. → Сейчас: локальное распознавание + локальные ответы. Аудио не уходит с этого компьютера.
Now: mixed. Tune providers in Settings. → Сейчас: смешанно. Настройте провайдеров в Settings.
Configure the AI provider in Settings; here we just confirm it answers. → Настройте AI-провайдера в Settings; здесь только проверяем, что он отвечает.
Test AI → Проверить AI
Configure STT in Settings; here we confirm it connects. → Настройте STT в Settings; здесь проверяем подключение.
Records 3 seconds. Speak during the test to see your level. → Запишет 3 секунды. Говорите во время теста, чтобы увидеть уровень.
Check system audio → Проверить системный звук
Needed to hear the other side. Plays a short test tone and checks loopback. → Нужно, чтобы слышать собеседника. Проигрывает короткий тон и проверяет loopback.
Overlay & stealth → Overlay и stealth
Hide overlay from screen capture → Скрывать overlay от захвата экрана
Press Print Screen or open a screen-share preview — the overlay should vanish from the capture but stay on your screen. We can't verify stealth automatically. → Нажмите Print Screen или откройте предпросмотр screen-share — overlay должен исчезнуть из захвата, но остаться на вашем экране. Автоматически проверить stealth нельзя.
Start working → Начать работу
Open diagnostics → Открыть диагностику
Speech recognition (STT) → Распознавание речи (STT)  [may already exist]
```

## STEP 9 — GATE (both crates), then review-agent (UI+geometry+stealth → ALL 5 layers)
```pwsh
~/.cargo/bin/cargo.exe fmt --manifest-path overlay-backend\Cargo.toml
~/.cargo/bin/cargo.exe fmt --manifest-path slint-experiment\Cargo.toml
~/.cargo/bin/cargo.exe clippy --manifest-path overlay-backend\Cargo.toml --all-targets -- -D warnings
~/.cargo/bin/cargo.exe clippy --manifest-path slint-experiment\Cargo.toml --all-targets -- -D warnings
~/.cargo/bin/cargo.exe test --manifest-path overlay-backend\Cargo.toml
~/.cargo/bin/cargo.exe test --manifest-path slint-experiment\Cargo.toml
```
Pre-empt: `too_many_arguments` (annotated). Then an independent review-agent on the
full diff. The git-gate hook (`git commit` via the Bash tool) re-runs fmt+clippy.

## STEP 10 — LIVE computer-use verification (Layer 5; user authorized computer-use)
1. Force first-run: rename `%APPDATA%\overlay-mvp\config.json` → `.bak`.
2. `cargo run --bin overlay-host`; capture stderr ~6s → confirm
   `first run detected — auto-opening setup wizard`.
3. Load computer-use tools (`ToolSearch query:"computer-use" max_results:30`),
   `request_access` for the overlay app. Screenshot → wizard renders **SOLID**
   (NOT invisible — RISK 1) + centered on the PRIMARY 1920×1080, header "Шаг 1 из 7".
4. Drive: click Далее → 2/7 + AI pill flips `проверка…`→`OK`/`Требует внимания`
   (proves the reused check ran). Назад → 1/7. Пропустить → advances WITHOUT a check.
   Step 4 (mic): auto-records, dBFS result appears. Step 5: sys check. Step 6: toggle
   stealth. Step 7: summary table + buttons. Начать работу → closes; `config.json`
   now exists; relaunch → wizard does NOT auto-open.
5. Stealth parity: open wizard via Settings "🪄" button; toggle stealth from the
   BAR chip then the SETTINGS tab → `CopyFromScreen` the wizard HWND rect each time:
   it must vanish from capture (STEP 7). **Colour ground-truth = `CopyFromScreen`,
   NOT computer-use screenshots** (they mis-render the overlay per CLAUDE.md).
6. Restore `config.json.bak`. Commit + push. **Do NOT cut a release** — leave for
   the user's go-ahead.

## 3 RISKS
1. **DWM created-but-invisible** new frameless window → SOLID `Theme.bg-surface`
   (done in wizard.slint). Verify FIRST in STEP 10.3 via `CopyFromScreen`. Fallback:
   wrap the whole layout in an opaque `Rectangle { background: Theme.bg-surface; }`.
2. **Two-path stealth parity hole** (security, #111 class) → STEP 7 patches BOTH
   toggles + STEP 5 seeds `set_stealth_on(global_stealth())`. Verify both paths live.
3. **Mic contention / panics** → mic guard verbatim (STEP 4); every closure
   `let Some(w)=…else{return}` + `if let Ok` + `match` (no unwrap/expect).

## SECURITY (per CLAUDE.md — non-negotiable)
- No bearer/key/`ai_base_url` LAN-IP ever rendered in the wizard. AI step uses
  `cfg.ai_endpoint(false)` resolver; all reused backend test fns already return
  secret-free errors, so `[err] {e}` strings are safe.
- The wizard window MUST be stealth-able (STEP 7) — it's a screen-shareable surface.
