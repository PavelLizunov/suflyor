---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_102d5af3955a"
source_path: "experiments/macos-gate0a/src/main.rs"
batch_id: "B15"
total_lines: 222
symbols_count: 15
review_state: validated
---

# File Map: `experiments/macos-gate0a/src/main.rs`

- **Batch:** B15
- **Physical Lines:** 222
- **Coverage:** 222/222 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (15)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `suflyor_gate0a_configure_overlay` | L26 | `fn suflyor_gate0a_configure_overlay(view: *mut c_void) -> ()` |
| function | `suflyor_gate0a_configure_tile` | L27 | `fn suflyor_gate0a_configure_tile(view: *mut c_void) -> ()` |
| function | `suflyor_gate0a_configure_settings` | L28 | `fn suflyor_gate0a_configure_settings(view: *mut c_void) -> ()` |
| function | `suflyor_gate0a_drag_window` | L29 | `fn suflyor_gate0a_drag_window(view: *mut c_void) -> ()` |
| function | `suflyor_gate0a_log_displays` | L30 | `fn suflyor_gate0a_log_displays() -> ()` |
| function | `main` | L33 | `fn main() -> Result<(), Box<dyn Error>>` |
| function | `run_macos` | L42 | `fn run_macos() -> Result<(), Box<dyn Error>>` |
| function | `seed_overlay` | L151 | `fn seed_overlay(window: &GateOverlay) -> ()` |
| function | `seed_tile` | L161 | `fn seed_tile(window: &GateTile) -> ()` |
| function | `seed_settings` | L171 | `fn seed_settings(window: &GateSettings) -> ()` |
| function | `appkit_view` | L181 | `fn appkit_view(component: &C) -> Result<*mut c_void, Box<dyn Error>>` |
| function | `configure_overlay_component` | L191 | `fn configure_overlay_component(component: &C) -> ()` |
| function | `configure_settings_component` | L199 | `fn configure_settings_component(component: &C) -> ()` |
| function | `configure_tile_component` | L207 | `fn configure_tile_component(component: &C) -> ()` |
| function | `drag_component` | L215 | `fn drag_component(component: &slint::Weak<C>) -> ()` |
