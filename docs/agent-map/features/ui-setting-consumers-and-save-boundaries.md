# UI setting consumers and save boundaries: selected source chains

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Four manually traced UI setting keys plus [unresolved name-edge census](<../schema/config-ui-name-edges.md>). No Rust/native/UI execution, config read/write or injected save failure. Names/quotes/comments are not authoritative when executable source differs.

## Name navigation is not consumer resolution

Across 175 Rust sources, census yields 1,232 Config-field-name member candidates and 1,719 generated Slint-method-name candidates (2,951 total). Receiver identifier/shape, containing function and closure/assignment context preserved, but receiver type/cfg/include/macro/alias/dynamic dispatch not resolved. 176 UI candidates match declarations in more than one component. Same `value` or `set_text` can be unrelated type; no callgraph/coverage percentage inferred.

84 Config field names represented; `auto_export_on_quit` has **zero selected Rust member-expression candidates**. Targeted repository source search found only [declaration/comment](<../../../overlay-backend/src/config.rs#L334-L345>) and [constructor](<../../../overlay-backend/src/config.rs#L690-L701>). Therefore its comment describing automatic quit export is uncorroborated by selected current Rust consumers; not proof all macro/generated/external consumers absent. No deletion/remediation or feature absence assertion beyond this scoped observation.

## Interface language: live selection before persistent save

[UI callback/property](<../../../slint-experiment/ui/settings_panel.slint#L593-L604>) and [widget](<../../../slint-experiment/ui/settings_panel.slint#L1180-L1198>) feed ru/en index. [Population](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L1908-L1914>) maps only exact en to index 1; other values display ru. [Startup](<../../../slint-experiment/src/bin/overlay_host_windows.rs#L764-L778>) passes stored language straight to bundled selection, merely logs error. Invalid config language can therefore have display-selection mismatch; runtime fallback not visually accepted.

[Callback](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L556-L610>) first selects bundle; selection Err returns without Config mutation. Success mutates `c.ui_language`, attempts save, logs save failure **without rollback/return**, then refreshes token/TTS/audio and tray/menu and recreates visible help/palette Rust text. Thus current Config memory/live bundle may differ from disk after save failure. This is not “saved first before async confirmation”; selection is synchronous and precedes save. AI answer language is separate: [Config::ui_is_ru](<../../../overlay-backend/src/config.rs#L521-L529>) compares exact ru; [AI prompt](<../../../overlay-backend/src/ai/prompt.rs#L1-L32>) uses response_language only.

## Color scheme: Config mutation, persistence, globals then existing windows

[Config field/default](<../../../overlay-backend/src/config.rs#L469-L475>) is i32. [Startup global](<../../../slint-experiment/src/bin/overlay_host_windows.rs#L725-L738>) seeds saved scheme; [global clamp](<../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L213-L241>) limits 0..3. [Callback](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L616-L644>) clamps selection, mutates Config, saves, returns on Err before process-global/bar/registry repaint. No Config rollback in Err branch: memory retains chosen scheme while global/live remains old; later Settings population may use memory on reopen. Native disk failure behavior remains unexecuted.

Successful save updates global for future windows plus bar/registry for current surfaces. [Tile constructor/presentation](<../../../slint-experiment/src/bin/overlay_host/tile_window.rs#L205-L220>) applies global scheme; [per-window setters](<../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L458-L490>) update Theme globals individually. Global setting alone does not automatically update every Slint window. Full registry completeness/colors/contrast/tofu unaccepted.

## Tile opacity: save gate for globals/live, but mutation already happened

[Config metadata](<../../../overlay-backend/src/config.rs#L484-L498>) and [Slint property](<../../../slint-experiment/ui/settings_panel.slint#L356-L369>) separate body opacity from entire HWND alpha. [Population](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L1695-L1697>) copies current Config; [startup](<../../../slint-experiment/src/bin/overlay_host_windows.rs#L595-L599>) seeds clamped global. [Callback](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L496-L518>) clamps 0.5..1, mutates Config, saves, returns on failure before global/live tile setters; no rollback. Same disk/memory/live divergence caveat as scheme; slider’s own optimistic binding is source/UI-state not natively tested here.

[Global atomic](<../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L35-L53>) stores f32 bits; [tile presentation](<../../../slint-experiment/src/bin/overlay_host/tile_window.rs#L279-L286>) reads it. Callback updates currently registered tile list; other tile construction paths can read Config directly. Atomic success is not proof all window renderers/layout respect body-only alpha.

## Tile monitor: runtime first, save failure intentionally nonblocking

[Callback](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L521-L546>) converts dropdown to current monitor origin pin, updates process global **before** Config/save, stores origin string in memory, logs save failure without rollback. Unlike scheme/opacity, runtime/new tiles intentionally follow selection even on failed persistence. [Startup](<../../../slint-experiment/src/bin/overlay_host_windows.rs#L600-L608>) parses saved string; [global pin](<../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L175-L211>) is a packed AtomicI64 with auto sentinel, read as Option. Monitor existence/ordering/fallback resolved when enum/pick helpers run; mixed-DPI/hotplug/native monitor acceptance still open.

## Verification limits

Seven new fixtures: member/method discrimination, nonConfig collision explicitly unresolved, receiver/closure/Unicode ranges, macro/string/comment exclusions, invalid syntax, real source order for language/scheme/opacity/monitor. They verify navigation and code ordering, **not Config type ownership, save failure reproduction, native visible state or independent acceptance**. Other 80 Config keys, every Slint setter/callback consumer, dynamic translation and Windows/indirect native paths remain unsettled; original Grok statuses unchanged.
