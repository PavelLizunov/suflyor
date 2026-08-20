#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

const HOST: &str = include_str!("../src/bin/overlay_host_windows.rs");
const LIFECYCLE: &str = include_str!("../src/bin/overlay_host/mlx_lifecycle.rs");
const ASK: &str = include_str!("../src/bin/overlay_host/tile_ask.rs");
const PTT: &str = include_str!("../src/bin/overlay_host/tile_ptt.rs");
const FOLLOWUP: &str = include_str!("../src/bin/overlay_host/tile_followup.rs");
const VISION: &str = include_str!("../src/bin/overlay_host/vision_capture.rs");
const AI: &str = include_str!("../../overlay-backend/src/ai.rs");
const RUNTIME: &str = include_str!("../../overlay-backend/src/mlx_runtime.rs");

#[test]
fn every_ask_family_crosses_the_mlx_route_boundary() {
    assert!(HOST.contains("mod mlx_lifecycle;"));
    assert!(ASK.matches("resolve_route_endpoint(").count() >= 3);
    assert!(PTT.contains("resolve_route_endpoint(AskRoute::Text"));
    assert!(FOLLOWUP.matches("resolve_route_endpoint(route").count() >= 2);
    assert!(VISION.contains("resolve_route_endpoint(AskRoute::Vision"));
    assert!(VISION.contains("route_needs_mlx(AskRoute::Vision"));
    assert!(AI.contains("resolve_managed_mlx_endpoint(endpoint.clone()).await?"));
    assert!(AI.contains("resolve_managed_mlx_endpoint(endpoint).await"));
    assert!(AI.contains("crate::mlx_runtime::acquire_request(&model)"));
}

#[test]
fn request_lease_prevents_cross_model_teardown_and_reaps_dead_children() {
    assert!(RUNTIME.contains("active_requests"));
    assert!(RUNTIME.contains("pub fn acquire_request"));
    assert!(RUNTIME.contains("pub struct MlxRequestLease"));
    assert!(RUNTIME.contains("impl Drop for MlxRequestLease"));
    assert!(RUNTIME.contains("bail!(\"MLX model is busy\")"));
    assert!(RUNTIME.contains("reap_exited_child(&mut state)"));
    assert!(RUNTIME.contains("pub fn stop_if_idle() -> bool"));
}

#[test]
fn lifecycle_is_owned_exact_and_never_downloads_or_persists_runtime_data() {
    assert!(LIFECYCLE.contains("active_endpoint_for_model(model).is_none()"));
    assert!(LIFECYCLE.contains("mlx_runtime::start(model)"));
    assert!(!LIFECYCLE.contains("mlx_install::install("));
    assert!(!LIFECYCLE.contains("config::save"));
    for forbidden in ["base_url =", "bearer =", "pkill", "killall"] {
        assert!(
            !LIFECYCLE.contains(forbidden),
            "forbidden lifecycle seam: {forbidden}"
        );
    }
}

#[test]
fn clean_host_shutdown_stops_the_owned_mlx_child() {
    let enter_lock = HOST
        .find("LockAction::EnterDeepLock")
        .expect("missing deep-lock transition");
    assert!(HOST[enter_lock..].contains("stop_mlx_model();"));
    let event_loop = HOST
        .find("let result = slint::run_event_loop_until_quit();")
        .expect("missing canonical event loop exit");
    let stop = HOST[event_loop..]
        .find("stop_mlx_model();")
        .expect("clean shutdown must stop MLX")
        + event_loop;
    let runtime_shutdown = HOST
        .find("tokio_rt.shutdown_timeout")
        .expect("missing runtime teardown");
    assert!(event_loop < stop && stop < runtime_shutdown);
}

#[test]
fn deep_unlock_serializes_the_only_temporary_guard_lowering() {
    assert!(HOST.contains("start_mlx_for_unlock(&cfg_unlock)"));
    let unlock = LIFECYCLE
        .find("pub(crate) fn start_mlx_for_unlock")
        .expect("missing MLX deep-unlock boundary");
    let body = &LIFECYCLE[unlock..];
    let lock = body
        .find("lifecycle_lock()")
        .expect("missing lifecycle lock");
    let lower = body
        .find("set_deep_lock_active(false)")
        .expect("missing temporary deep-lock lowering");
    let start = body
        .find("mlx_runtime::start(&model)")
        .expect("missing exact MLX start");
    let raise = body
        .find("set_deep_lock_active(true)")
        .expect("missing deep-lock restore");
    assert!(lock < lower && lower < start && start < raise);
}
