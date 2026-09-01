//! Exact-child lifecycle owner for the native macOS MLX sidecar.

use anyhow::{bail, Context, Result};
use parking_lot::Mutex;
use std::sync::OnceLock;

#[cfg(any(target_os = "macos", test))]
use serde::Deserialize;
#[cfg(target_os = "macos")]
use serde::Serialize;

#[cfg(any(target_os = "macos", test))]
const PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, PartialEq, Eq)]
pub struct MlxEndpoint {
    pub base_url: String,
    pub bearer: String,
    pub model: String,
}

impl std::fmt::Debug for MlxEndpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MlxEndpoint")
            .field("base_url", &self.base_url)
            .field("bearer", &"[redacted]")
            .field("model", &self.model)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MlxMemorySample {
    pub app_bytes: Option<u64>,
    pub mlx_bytes: Option<u64>,
}

#[cfg(target_os = "macos")]
extern "C" {
    fn suflyor_process_footprint(pid: u32, bytes: *mut u64) -> i32;
}

#[cfg(target_os = "macos")]
fn process_footprint(pid: u32) -> Option<u64> {
    let mut bytes = 0_u64;
    let ok = unsafe { suflyor_process_footprint(pid, &mut bytes) };
    (ok == 1 && bytes != 0).then_some(bytes)
}

/// Current macOS physical footprint for the host and its exact owned MLX child.
/// Values are separate process-accounting samples, not additive GPU-only usage.
#[must_use]
pub fn memory_sample() -> MlxMemorySample {
    #[cfg(target_os = "macos")]
    {
        let child_pid = {
            let mut runtime = state().lock();
            reap_exited_child(&mut runtime);
            runtime.child.as_ref().map(std::process::Child::id)
        };
        MlxMemorySample {
            app_bytes: process_footprint(std::process::id()),
            mlx_bytes: child_pid.and_then(process_footprint),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        MlxMemorySample::default()
    }
}

/// Duration of the most recent successful resident-model cold start.
#[must_use]
pub fn last_load_ms() -> Option<u64> {
    state().lock().last_load_ms
}

#[derive(Default)]
struct RuntimeState {
    generation: u64,
    active_requests: usize,
    endpoint: Option<MlxEndpoint>,
    last_owned_base_url: Option<String>,
    last_load_ms: Option<u64>,
    #[cfg(target_os = "macos")]
    child: Option<std::process::Child>,
    #[cfg(target_os = "macos")]
    stdin: Option<std::process::ChildStdin>,
}

fn state() -> &'static Mutex<RuntimeState> {
    static STATE: OnceLock<Mutex<RuntimeState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(RuntimeState::default()))
}

fn lifecycle() -> &'static Mutex<()> {
    static LIFECYCLE: OnceLock<Mutex<()>> = OnceLock::new();
    LIFECYCLE.get_or_init(|| Mutex::new(()))
}

#[must_use]
pub struct MlxRequestLease {
    generation: u64,
}

impl Drop for MlxRequestLease {
    fn drop(&mut self) {
        release_request(&mut state().lock(), self.generation);
    }
}

#[must_use]
pub fn active_endpoint_for_model(model: &str) -> Option<MlxEndpoint> {
    let mut state = state().lock();
    reap_exited_child(&mut state);
    endpoint_matching_model(state.endpoint.as_ref(), model)
}

fn endpoint_matching_model(endpoint: Option<&MlxEndpoint>, model: &str) -> Option<MlxEndpoint> {
    endpoint.filter(|endpoint| endpoint.model == model).cloned()
}

#[must_use]
pub fn is_owned_endpoint(base_url: &str) -> bool {
    let mut state = state().lock();
    reap_exited_child(&mut state);
    owned_endpoint_matches(
        state.endpoint.as_ref(),
        state.last_owned_base_url.as_deref(),
        base_url,
    )
}

fn owned_endpoint_matches(
    active: Option<&MlxEndpoint>,
    last_owned: Option<&str>,
    base_url: &str,
) -> bool {
    active.is_some_and(|endpoint| endpoint.base_url == base_url) || last_owned == Some(base_url)
}

#[must_use]
pub fn selected_model() -> Option<String> {
    let mut state = state().lock();
    reap_exited_child(&mut state);
    state
        .endpoint
        .as_ref()
        .map(|endpoint| endpoint.model.clone())
}

#[must_use]
pub fn begin_intent() -> u64 {
    let mut state = state().lock();
    state.generation = state.generation.wrapping_add(1);
    state.generation
}

#[must_use]
pub fn intent_is_current(generation: u64) -> bool {
    state().lock().generation == generation
}

pub fn start(model: &str) -> Result<MlxEndpoint> {
    let _lifecycle = lifecycle().lock();
    start_locked(model)
}

fn start_locked(model: &str) -> Result<MlxEndpoint> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = model;
        bail!("MLX runtime is unavailable on this platform");
    }
    #[cfg(target_os = "macos")]
    {
        start_macos(model)
    }
}

pub fn stop() {
    let _lifecycle = lifecycle().lock();
    stop_locked();
}

fn stop_locked() {
    #[cfg(target_os = "macos")]
    let owned = {
        let mut state = state().lock();
        state.generation = state.generation.wrapping_add(1);
        state.active_requests = 0;
        state.last_load_ms = None;
        clear_endpoint(&mut state);
        (state.stdin.take(), state.child.take())
    };
    #[cfg(not(target_os = "macos"))]
    {
        let mut state = state().lock();
        state.generation = state.generation.wrapping_add(1);
        state.active_requests = 0;
        state.last_load_ms = None;
        clear_endpoint(&mut state);
    }
    #[cfg(target_os = "macos")]
    stop_child(owned);
}

/// Stop the owned sidecar only when no request is using it. Settings uses
/// this when leaving MLX so a provider change never tears down an answer.
#[must_use]
pub fn stop_if_idle() -> bool {
    let _lifecycle = lifecycle().lock();
    {
        let mut state = state().lock();
        reap_exited_child(&mut state);
        if state.active_requests != 0 {
            return false;
        }
    }
    stop_locked();
    true
}

/// Resolve the selected model and reserve the single resident sidecar for the
/// complete request lifetime. A different model cannot replace it until the
/// returned lease is dropped.
pub fn acquire_request(model: &str) -> Result<(MlxEndpoint, MlxRequestLease)> {
    let _lifecycle = lifecycle().lock();
    let endpoint = start_locked(model)?;
    let generation = {
        let mut state = state().lock();
        reap_exited_child(&mut state);
        if endpoint_matching_model(state.endpoint.as_ref(), model).is_none() {
            bail!("MLX sidecar is unavailable");
        }
        state.active_requests = state
            .active_requests
            .checked_add(1)
            .context("too many MLX requests")?;
        state.generation
    };
    Ok((endpoint, MlxRequestLease { generation }))
}

fn release_request(state: &mut RuntimeState, generation: u64) {
    if state.generation == generation {
        state.active_requests = state.active_requests.saturating_sub(1);
    }
}

fn clear_endpoint(state: &mut RuntimeState) {
    if let Some(endpoint) = state.endpoint.take() {
        state.last_owned_base_url = Some(endpoint.base_url);
    }
}

#[cfg(target_os = "macos")]
fn reap_exited_child(state: &mut RuntimeState) {
    let exited = state
        .child
        .as_mut()
        .is_some_and(|child| child.try_wait().is_ok_and(|status| status.is_some()));
    if exited {
        state.generation = state.generation.wrapping_add(1);
        state.active_requests = 0;
        state.last_load_ms = None;
        clear_endpoint(state);
        let _stdin = state.stdin.take();
        let _child = state.child.take();
    }
}

#[cfg(not(target_os = "macos"))]
fn reap_exited_child(_state: &mut RuntimeState) {}

#[cfg(target_os = "macos")]
#[derive(Serialize)]
struct StartupWire<'a> {
    version: u32,
    bearer: &'a str,
    model: &'a str,
    snapshot: &'a str,
}

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Deserialize, PartialEq, Eq)]
struct ReadyWire {
    event: String,
    version: u32,
    port: u16,
    model: String,
}

#[cfg(any(target_os = "macos", test))]
fn parse_ready(line: &str, expected_model: &str) -> Result<u16> {
    if line.len() > 64 * 1024 {
        bail!("invalid MLX startup response");
    }
    let ready: ReadyWire = serde_json::from_str(line).context("invalid MLX startup response")?;
    if ready.event != "READY"
        || ready.version != PROTOCOL_VERSION
        || ready.port == 0
        || ready.model != expected_model
    {
        bail!("invalid MLX startup response");
    }
    Ok(ready.port)
}

#[cfg(any(target_os = "macos", test))]
fn is_valid_mlx_token(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= 96
        && token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-')
}

#[cfg(any(target_os = "macos", test))]
fn is_valid_mlx_code(code: &str) -> bool {
    !code.is_empty() && code.len() <= 96 && code.parse::<i64>().is_ok()
}

#[cfg(target_os = "macos")]
const MLX_GENERIC_FAILURE: &str = "suflyor-mlx failed";

#[cfg(any(target_os = "macos", test))]
fn parse_mlx_stderr_line(line: &str) -> Option<&str> {
    let parts: Vec<&str> = line.split(' ').collect();
    if parts.len() != 8 {
        return None;
    }
    if parts[0] != "MLX" || parts[1] != "failure" {
        return None;
    }
    let scope = parts[2].strip_prefix("scope=")?;
    if !is_valid_mlx_token(scope) {
        return None;
    }
    let phase = parts[3].strip_prefix("phase=")?;
    if !is_valid_mlx_token(phase) {
        return None;
    }
    let type_ = parts[4].strip_prefix("type=")?;
    if !is_valid_mlx_token(type_) {
        return None;
    }
    let domain = parts[5].strip_prefix("domain=")?;
    if !is_valid_mlx_token(domain) {
        return None;
    }
    let code = parts[6].strip_prefix("code=")?;
    if !is_valid_mlx_code(code) {
        return None;
    }
    let sidecar = parts[7].strip_prefix("sidecar=")?;
    if !is_valid_mlx_token(sidecar) {
        return None;
    }
    Some(line)
}

#[cfg(target_os = "macos")]
fn drain_mlx_stderr(stderr: std::process::ChildStderr) {
    use std::io::{BufRead, BufReader, Read};
    let mut reader = BufReader::new(stderr);
    let mut line = String::new();
    let mut prev_was_generic = false;
    {
        // Bound parsed diagnostics; keep draining the pipe below without allocating.
        let mut bounded = reader.by_ref().take(65_536);
        while bounded.read_line(&mut line).is_ok_and(|bytes| bytes > 0) {
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if !trimmed.is_empty() {
                if let Some(structured) = parse_mlx_stderr_line(trimmed) {
                    log::warn!("{structured}");
                    prev_was_generic = false;
                } else if !prev_was_generic {
                    log::warn!("{MLX_GENERIC_FAILURE}");
                    prev_was_generic = true;
                }
            }
            line.clear();
        }
    }
    let _ = std::io::copy(&mut reader, &mut std::io::sink());
}

#[cfg(target_os = "macos")]
fn random_token() -> Result<String> {
    let mut bytes = [0_u8; 32];
    getrandom::getrandom(&mut bytes).map_err(|_| anyhow::anyhow!("create MLX session token"))?;
    Ok(crate::download::hex(&bytes))
}

#[cfg(target_os = "macos")]
fn start_macos(model: &str) -> Result<MlxEndpoint> {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::process::{Command, Stdio};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    if crate::deep_lock::deep_lock_active() {
        bail!(crate::deep_lock::BLOCKED_ERROR);
    }
    {
        let mut state = state().lock();
        reap_exited_child(&mut state);
        if let Some(endpoint) = endpoint_matching_model(state.endpoint.as_ref(), model) {
            return Ok(endpoint);
        }
        if state.active_requests != 0 {
            bail!("MLX model is busy");
        }
    }
    let load_started = Instant::now();
    state().lock().last_load_ms = None;
    let catalog = crate::mlx_install::catalog_model(model).context("unsupported MLX model")?;
    let snapshot =
        crate::mlx_install::installed_snapshot(model).context("MLX model is not installed")?;
    let generation = begin_intent();
    let old_child = {
        let mut state = state().lock();
        clear_endpoint(&mut state);
        (state.stdin.take(), state.child.take())
    };
    stop_child(old_child);
    let executable = std::env::current_exe()
        .context("locate MLX sidecar")?
        .parent()
        .context("locate MLX sidecar")?
        .join("suflyor-mlx");
    if !executable.is_file() {
        bail!("MLX sidecar is unavailable");
    }
    let token = random_token()?;
    let snapshot_text = snapshot.to_str().context("invalid MLX snapshot path")?;
    let wire = serde_json::to_string(&StartupWire {
        version: PROTOCOL_VERSION,
        bearer: &token,
        model: catalog.id,
        snapshot: snapshot_text,
    })
    .context("encode MLX startup")?;
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("start MLX sidecar")?;
    let Some(stderr) = child.stderr.take() else {
        let _ = child.kill();
        let _ = child.wait();
        bail!("open MLX sidecar stderr");
    };
    if let Err(error) = std::thread::Builder::new()
        .name("suflyor-mlx-stderr".into())
        .spawn(move || drain_mlx_stderr(stderr))
    {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error).context("start MLX stderr drain");
    }
    let configured = (|| -> Result<(std::process::ChildStdin, std::process::ChildStdout)> {
        let mut stdin = child.stdin.take().context("open MLX sidecar stdin")?;
        writeln!(stdin, "{wire}").context("configure MLX sidecar")?;
        stdin.flush().context("configure MLX sidecar")?;
        let stdout = child.stdout.take().context("open MLX sidecar stdout")?;
        Ok((stdin, stdout))
    })();
    let (stdin, stdout) = match configured {
        Ok(configured) => configured,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    let (send, receive) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let result = (&mut reader)
            .take(65_537)
            .read_line(&mut line)
            .map(|_| line);
        let _ = send.send(result);
        // READY is the only protocol line today. Keep draining defensively so
        // future diagnostics cannot fill the child's stdout pipe and stall it.
        let _ = std::io::copy(&mut reader, &mut std::io::sink());
    });
    let line = match receive.recv_timeout(Duration::from_secs(180)) {
        Ok(Ok(line)) => line,
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            bail!("MLX sidecar did not become ready");
        }
    };
    let port = match parse_ready(&line, model) {
        Ok(port) => port,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    let endpoint = MlxEndpoint {
        base_url: format!("http://127.0.0.1:{port}/v1"),
        bearer: token,
        model: model.to_string(),
    };
    let probes_ready = probe(&endpoint, "/health", None).and_then(|healthy| {
        if healthy {
            probe(&endpoint, "/v1/models", Some(model))
        } else {
            Ok(false)
        }
    });
    if !matches!(probes_ready, Ok(true)) {
        let _ = child.kill();
        let _ = child.wait();
        bail!("MLX sidecar readiness check failed");
    }
    let mut state = state().lock();
    if state.generation != generation || crate::deep_lock::deep_lock_active() {
        drop(state);
        drop(stdin);
        let _ = child.kill();
        let _ = child.wait();
        bail!("MLX start was superseded");
    }
    state.last_owned_base_url = Some(endpoint.base_url.clone());
    state.endpoint = Some(endpoint.clone());
    state.last_load_ms = Some(load_started.elapsed().as_millis().min(u64::MAX as u128) as u64);
    state.stdin = Some(stdin);
    state.child = Some(child);
    Ok(endpoint)
}

#[cfg(target_os = "macos")]
fn probe(endpoint: &MlxEndpoint, path: &str, expected_model: Option<&str>) -> Result<bool> {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;
    let port = endpoint
        .base_url
        .strip_prefix("http://127.0.0.1:")
        .and_then(|rest| rest.strip_suffix("/v1"))
        .and_then(|port| port.parse::<u16>().ok())
        .context("invalid MLX endpoint")?;
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(3))
        .context("connect MLX readiness check")?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .context("configure MLX readiness check")?;
    write!(stream, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {}\r\nConnection: close\r\n\r\n", endpoint.bearer).context("send MLX readiness check")?;
    let mut response = Vec::new();
    stream
        .take(128 * 1024)
        .read_to_end(&mut response)
        .context("read MLX readiness check")?;
    let text = String::from_utf8(response).context("decode MLX readiness check")?;
    if !text.starts_with("HTTP/1.1 200 ") {
        return Ok(false);
    }
    let Some(model) = expected_model else {
        return Ok(true);
    };
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(body).context("parse MLX model check")?;
    Ok(json
        .pointer("/data/0/id")
        .and_then(serde_json::Value::as_str)
        == Some(model))
}

#[cfg(target_os = "macos")]
fn stop_child(
    owned: (
        Option<std::process::ChildStdin>,
        Option<std::process::Child>,
    ),
) {
    use std::time::{Duration, Instant};
    drop(owned.0);
    let Some(mut child) = owned.1 else {
        return;
    };
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if child.try_wait().is_ok_and(|status| status.is_some()) {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn ready_parser_is_bounded_exact_and_loopback_port_only() {
        assert_eq!(
            parse_ready(
                r#"{"event":"READY","version":1,"port":49152,"model":"m"}"#,
                "m"
            )
            .unwrap(),
            49152
        );
        for bad in [
            r#"{"event":"READY","version":2,"port":1,"model":"m"}"#,
            r#"{"event":"READY","version":1,"port":0,"model":"m"}"#,
            r#"{"event":"READY","version":1,"port":1,"model":"other"}"#,
            "not json",
        ] {
            assert!(parse_ready(bad, "m").is_err(), "{bad}");
        }
        assert!(parse_ready(&"x".repeat(64 * 1024 + 1), "m").is_err());
    }

    #[test]
    fn generation_fence_rejects_stale_intents() {
        let mut state = state().lock();
        state.generation = state.generation.wrapping_add(1);
        let first = state.generation;
        assert_eq!(state.generation, first);
        state.generation = state.generation.wrapping_add(1);
        let second = state.generation;
        assert_ne!(first, second);
        assert_eq!(state.generation, second);
    }

    #[test]
    fn endpoint_debug_redacts_session_token() {
        let endpoint = MlxEndpoint {
            base_url: "http://127.0.0.1:123/v1".into(),
            bearer: "secret-token".into(),
            model: "m".into(),
        };
        let debug = format!("{endpoint:?}");
        assert!(debug.contains("[redacted]"));
        assert!(!debug.contains("secret-token"));
    }

    #[test]
    fn ownership_matches_only_exact_current_or_last_endpoint() {
        let endpoint = MlxEndpoint {
            base_url: "http://127.0.0.1:123/v1".into(),
            bearer: "token".into(),
            model: "m".into(),
        };
        assert!(owned_endpoint_matches(
            Some(&endpoint),
            None,
            &endpoint.base_url
        ));
        assert!(owned_endpoint_matches(
            None,
            Some("http://127.0.0.1:456/v1"),
            "http://127.0.0.1:456/v1"
        ));
        assert!(!owned_endpoint_matches(
            Some(&endpoint),
            Some("http://127.0.0.1:456/v1"),
            "http://127.0.0.1:999/v1"
        ));
        assert_eq!(
            endpoint_matching_model(Some(&endpoint), "m"),
            Some(endpoint.clone())
        );
        assert!(endpoint_matching_model(Some(&endpoint), "other").is_none());
    }

    #[test]
    fn request_release_is_generation_fenced() {
        let mut state = RuntimeState {
            generation: 7,
            active_requests: 2,
            ..RuntimeState::default()
        };
        release_request(&mut state, 6);
        assert_eq!(state.active_requests, 2);
        release_request(&mut state, 7);
        assert_eq!(state.active_requests, 1);
        release_request(&mut state, 7);
        release_request(&mut state, 7);
        assert_eq!(state.active_requests, 0);
    }

    #[test]
    fn mlx_stderr_privacy_drain_accepts_exact_structured_shape() {
        let line = "MLX failure scope=model phase=load type=out_of_memory domain=gpu code=-1 sidecar=suflyor-mlx";
        assert_eq!(parse_mlx_stderr_line(line), Some(line));

        let line_pos_code = "MLX failure scope=engine phase=init type=bad_config domain=host code=42 sidecar=mlx.py";
        assert_eq!(parse_mlx_stderr_line(line_pos_code), Some(line_pos_code));
    }

    #[test]
    fn mlx_stderr_privacy_drain_rejects_malformed_path_and_unbounded_tokens() {
        assert_eq!(
            parse_mlx_stderr_line("MLX failure scope=/usr/local/model.bin phase=load type=err domain=gpu code=-1 sidecar=suflyor-mlx"),
            None
        );
        assert_eq!(
            parse_mlx_stderr_line("MLX failure scope=model phase=load type=err domain=gpu code=fail sidecar=suflyor-mlx"),
            None
        );
        let long_token = "a".repeat(97);
        let line_long = format!("MLX failure scope={long_token} phase=load type=err domain=gpu code=-1 sidecar=suflyor-mlx");
        assert_eq!(parse_mlx_stderr_line(&line_long), None);

        assert_eq!(
            parse_mlx_stderr_line(
                "MLX failure scope= phase=load type=err domain=gpu code=-1 sidecar=suflyor-mlx"
            ),
            None
        );
        assert_eq!(
            parse_mlx_stderr_line(
                "Traceback (most recent call last): File \"/app/main.py\", line 10"
            ),
            None
        );
        assert_eq!(
            parse_mlx_stderr_line("MLX failure scope=model phase=load type=err domain=gpu code=-1 sidecar=suflyor-mlx extra"),
            None
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn non_macos_runtime_fails_closed() {
        stop();
        assert!(start(crate::mlx_install::DEFAULT_TEXT_MODEL).is_err());
        assert!(active_endpoint_for_model(crate::mlx_install::DEFAULT_TEXT_MODEL).is_none());
    }
}
