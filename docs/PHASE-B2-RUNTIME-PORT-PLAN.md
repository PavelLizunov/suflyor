# Phase B2 — runtime.rs port plan

Generated 2026-05-27 by background research agent. Concrete fn-by-fn
table for porting `src-tauri/src/runtime.rs` (3843 LOC, not 3919 as
the earlier estimate) into `overlay-backend` using the `RuntimeEvents`
trait already scaffolded in `overlay-backend/src/events.rs`.

## Headline numbers

- 9 public AppHandle-taking fns (matches A2 audit)
- 2 private fns must port alongside (`maybe_spawn_tile`,
  `run_post_meeting_debrief`)
- 30 `.emit_to("overlay", ...)` sites (less than the 43 prior estimate)
- 8 `tile::spawn_tile_with_stealth(...)` sites
- 4 `*::spawn` sites — 3 plain `tokio::spawn` (fine), 1
  `tauri::async_runtime::spawn` at runtime.rs:1811 deliberately for
  Tauri's TLS reactor
- **Estimated total effort: ~22 hours agent-time** spread across
  multiple sessions

## Trait surface that needs to grow

The current `RuntimeEvents { emit, spawn_tile(TileSpec) }` is too
thin. Concrete additions before port begins:

1. **`spawn_tile_full(&self, spec: TileSpec, monitor: Option<usize>,
   stealth: bool, kind: TileKind) -> Result<String, String>`** — real
   tile spawn signature with preferred-monitor + stealth flag + tile
   kind discriminant. Used at all 8 call sites.
2. **`spawn_detached(&self, fut: BoxFuture<'static, ()>)`** —
   abstracts over executor for the line-1811 site that needs
   `tauri::async_runtime::spawn` on Tauri but plain `tokio::spawn`
   in Slint binary. Three other tokio::spawn sites can stay raw.
3. **Hard-coded `"overlay"` window target** dropped from `emit`.
   Slint impl ignores window target; Tauri impl re-adds it
   internally. If a NEW target ever needs distinct routing,
   reserve `emit_to(window: &str, channel, payload)` later.
4. **CancellationToken** for task abort — OPTIONAL. Current code
   stores `JoinHandle` + calls `h.abort()`, which is fine.

## Per-fn port table

| Fn | Line | Emits | Spawns | Tiles | LOC chg | Deps | Diff |
|---|---:|---:|---:|---:|---:|---|---|
| `start_session` | 314 | 5 | 2 | 0 | 80 | `maybe_spawn_tile` | hard |
| `maybe_spawn_tile` (priv) | 746 | 3 | 0 | 2 | 120 | — | hard |
| `reask_last` | 1567 | 3 | 0 | 1 | 30 | — | easy |
| `stop_session` | 1718 | 1 | 1 | 0 | 40 | `run_post_meeting_debrief` | medium |
| `run_post_meeting_debrief` (priv) | 1899 | 0 | 0 | 1 | 25 | — | easy |
| `manual_ask_source` | 2016 | 3 | 0 | 1 | 40 | — | medium |
| `manual_ask_window_end` | 2255 | 9 | 0 | 1 | 90 | — | hard |
| `manual_spawn_tile` | 2572 | 3 | 0 | 1 | 35 | — | medium |
| `ask` | 2705 | 3 | 1 | 0 | 40 | — | medium |

## Recommended port order

1. **`run_post_meeting_debrief`** — smallest, private, 0 emits, 1
   tile spawn. Exercises spawn_tile trait extension end-to-end with
   low risk.
2. **`reask_last`** — 3 emits, 1 tile spawn, no deps. Second-easiest,
   proves the trait round-trip.
3. **`manual_spawn_tile`** — similar shape to reask_last; validates
   trait under F6 path.
4. **`ask`** — no tile spawn, only emit + 1 tokio::spawn. Isolates
   the executor question from the tile question.
5. **`manual_ask_source`** — 3 emits + 1 tile spawn. Depends on
   stable tile trait from step 1.
6. **`manual_ask_window_end`** — 9 emits, biggest single PTT fn.
   Port last among the manual-ask family.
7. **`maybe_spawn_tile`** — private but heaviest tile logic. Port
   together with step 8.
8. **`start_session`** — tightly couples `maybe_spawn_tile` via the
   transcript-forwarder closure. Must land together with #7 or
   behind a feature flag.
9. **`stop_session`** — depends on `run_post_meeting_debrief`
   already ported. Last.

## Biggest risks

1. **`tauri::async_runtime::spawn` at line 1811 (debrief task)**
   explicitly needs Tauri's TLS reactor per the inline comment.
   Moving it to overlay-backend means tokio runtime context must
   be guaranteed at call time, otherwise Slint-side panics with
   "no reactor running". Mitigation: `spawn_detached` trait method
   above.
2. **SharedTiles coupling** — `SharedTiles` is a Tauri-specific
   tile registry (HashMap of tile labels → window handles).
   Decoupling means EITHER (a) move tile registry into
   overlay-backend (large surface), OR (b) hide entirely behind
   `spawn_tile` trait method (Tauri impl maintains registry
   internally). Option (b) is correct but rewrites tile.rs's
   public API.
3. **`start_session` transcript-forwarder hot loop** — emits inside
   the per-frame loop. Arc<dyn RuntimeEvents> dynamic dispatch in
   this path is fine but ANY clone or lock contention will be
   visible as transcript jitter. Benchmark before/after.
4. **30 emit sites all target hard-coded window "overlay"** —
   Slint impl must silently accept and route. If a NEW window
   target ever lands (e.g. tile-* gets a direct emit), trait
   signature `emit(channel, payload)` loses information.
   Mitigate by reserving `emit_to(window, channel, payload)` slot.
5. **Test coverage** — only 4 of 9 ported fns have direct unit
   tests; manual-ask + start-session paths exercised only via
   integration. **Add spec tests BEFORE porting each fn**
   otherwise regressions land silently. Methodology rule.

## Shim strategy after port

`src-tauri/src/runtime.rs` becomes a thin re-export module:

```rust
pub use overlay_backend::runtime::{
    start_session, stop_session, ask, reask_last,
    manual_ask_source, manual_ask_window_end, manual_spawn_tile,
};
```

`TauriEvents` adapter lives in `src-tauri/src/lib.rs`:

```rust
struct TauriEvents {
    app: AppHandle,
    tiles: SharedTiles,
}

impl RuntimeEvents for TauriEvents {
    fn emit(&self, channel: &str, payload: serde_json::Value) {
        let _ = self.app.emit_to("overlay", channel, payload);
    }
    fn spawn_tile_full(&self, spec, monitor, stealth, kind) -> Result<String, String> {
        tile::spawn_tile_with_stealth(&self.app, &self.tiles, spec, monitor, stealth, kind)
    }
    fn spawn_detached(&self, fut: BoxFuture<'static, ()>) {
        tauri::async_runtime::spawn(fut);
    }
}
```

`#[tauri::command]` wrappers keep their current signatures `(app:
AppHandle, cfg: tauri::State<SharedConfig>, ...)` but construct
`Arc::new(TauriEvents { app: app.clone(), tiles: tiles.clone() })
as Arc<dyn RuntimeEvents>` and pass into the relocated
`overlay_backend` fn.

**Net effect:** zero call-site changes on the React side, zero Tauri
command signature changes, but runtime.rs's 3843 LOC moves out of
src-tauri into overlay-backend and becomes consumable by the Slint
binary too.

## When to execute

After current session's Phase D1 settings_panel batch lands. Allow
1 dedicated multi-hour session per the "hard" fns; the 4 "easy" or
"medium" ports can land incrementally per the order above with
review-agent before each commit.
