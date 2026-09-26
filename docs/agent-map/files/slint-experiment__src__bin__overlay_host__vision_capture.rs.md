---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_6c65ee883a2b"
source_path: "slint-experiment/src/bin/overlay_host/vision_capture.rs"
batch_id: "B01"
total_lines: 917
symbols_count: 8
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/vision_capture.rs`

- **Batch:** B01
- **Physical Lines:** 917
- **Coverage:** 917/917 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (8)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `bgra_to_slint_image` | L51 | `fn bgra_to_slint_image(bgra: &[u8], w: u32, h: u32) -> slint::Image` |
| function | `local_ocr_available` | L67 | `fn local_ocr_available() -> bool` |
| function | `run_local_ocr` | L82 | `fn run_local_ocr(bgra: &[u8], width: u32, height: u32) -> Result<String, String>` |
| function | `spawn_vision_notice` | L100 | `fn spawn_vision_notice(events: &Arc<dyn RuntimeEvents>, cfg: &overlay_backend::config::SharedConfig, source: &str, ru: &str, en: &str,) -> ()` |
| function | `fire_f8_vision_capture` | L141 | `fn fire_f8_vision_capture(bridge: &Arc<OverlayBarBridge>, events: &Arc<dyn RuntimeEvents>, cfg: &overlay_backend::config::SharedConfig, slint_rt: &SharedSlintRuntime, rt_handle: &tokio::runtime::Handle, tiles: &TileWindows, weak_overlay: &slint::Weak<OverlayBarWindow>, capture_overlay: &Rc<RefCell<Option<CaptureOverlay>>>, mode: vision::VisionMode,) -> ()` |
| function | `vision_tile_copy` | L418 | `fn vision_tile_copy(mode: vision::VisionMode, is_ru: bool,) -> (&'static str, &'static str, &'static str, &'static str)` |
| function | `launch_vision_for_bgra` | L475 | `fn launch_vision_for_bgra(shot: slint_replay::capture::CapturedBgra, ep: Option<overlay_backend::config::AiEndpoint>, mode: vision::VisionMode, bridge: &Arc<OverlayBarBridge>, events: &Arc<dyn RuntimeEvents>, cfg: &overlay_backend::config::SharedConfig, slint_rt: &SharedSlintRuntime, rt_handle: &tokio::runtime::Handle, tiles: &TileWindows, weak_overlay: &slint::Weak<OverlayBarWindow>,) -> ()` |
| function | `deterministic_vision_copy_follows_ui_language` | L902 | `fn deterministic_vision_copy_follows_ui_language() -> ()` |

## Configuration Access

- Configuration read at L624: `&& !cfg.read().ai_bearer.trim().is_empty()`
