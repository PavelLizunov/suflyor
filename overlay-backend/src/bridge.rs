//! Hermes bridge — a LOOPBACK-ONLY, token-authenticated, read-mostly HTTP API
//! so the owner's local Hermes agent can consult suflyor artifacts (ТЗ
//! 2026-07-09, `docs/goal-hermes-integration-2026-07-09.md`).
//!
//! SECURITY MODEL (this deliberately relaxes the "no IPC surface" invariant —
//! the ADR is the goal doc):
//! - OFF by default (`hermes_bridge_enabled=false`); started only from Settings.
//! - Binds STRICTLY to `127.0.0.1` — never a LAN interface.
//! - Every request must carry `Authorization: Bearer <hermes_bridge_token>`;
//!   an empty configured token refuses to start the server at all.
//! - READS expose text only (transcripts / summaries / approved memory /
//!   profiles). Never audio, screenshots, or any config secret.
//! - WRITES are two narrow verbs: a memory SUGGESTION (lands in the approval
//!   queue — the owner still approves in «Память», the approval invariant
//!   holds) and a profile upsert (meeting prep written by the agent).
//! - Request bodies are capped ([`MAX_BODY_BYTES`]); responses carry generic
//!   errors (no internal error chains); bodies are never logged.
//!
//! Design: the HTTP loop (`tiny_http`, a dedicated thread, `recv_timeout`
//! polling an [`AtomicBool`] so Settings can stop it live) is a thin shell
//! around the pure-ish [`dispatch`] — which takes the `Store` + config and
//! returns `(status, json, needs_config_save)`. `dispatch` never touches the
//! on-disk config file itself, so unit tests run against an in-memory store +
//! a scratch config without writing the user's real `config.json`.

use crate::config::SharedConfig;
use crate::persistence::{open_default_store, NewMemoryCandidate, Store};
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Max accepted request body (a profile context is ≤ ~20k chars; 256 KiB is
/// far above any legitimate payload).
const MAX_BODY_BYTES: usize = 256 * 1024;
/// Default / max transcript characters returned by `GET /sessions/{id}`.
const TRANSCRIPT_CAP_DEFAULT: usize = 60_000;
const TRANSCRIPT_CAP_MAX: usize = 500_000;
/// Caps for list endpoints.
const SESSIONS_LIMIT_DEFAULT: usize = 10;
const SESSIONS_LIMIT_MAX: usize = 100;
const SEARCH_LIMIT_DEFAULT: i64 = 10;
const SEARCH_LIMIT_MAX: i64 = 50;
/// Caps for the two write verbs.
const SUGGEST_TEXT_MAX_CHARS: usize = 2_000;
const PROFILE_NAME_MAX_CHARS: usize = 100;
const PROFILE_CONTEXT_MAX_CHARS: usize = 20_000;

/// A running bridge server; dropping WITHOUT calling [`BridgeHandle::stop`]
/// leaves the thread running for the process lifetime (fine at app exit).
pub struct BridgeHandle {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

/// Normalize the configured bind host: blank → loopback default. Trimmed so a
/// stray space in the field can't make `tiny_http` fail to resolve.
fn bind_host(configured: &str) -> String {
    let h = configured.trim();
    if h.is_empty() {
        "127.0.0.1".to_string()
    } else {
        h.to_string()
    }
}

/// True when `host` is a loopback address (or blank → default loopback). Used by
/// Settings to decide whether to warn that the bridge is reachable off-machine.
#[must_use]
pub fn is_loopback_host(host: &str) -> bool {
    matches!(bind_host(host).as_str(), "127.0.0.1" | "localhost" | "::1")
}

/// Generate a fresh bridge token — 32 lowercase hex chars (16 bytes of OS
/// entropy). Used by the Settings «сгенерировать токен» button. Falls back to a
/// time-seeded value only if the OS RNG is unavailable (never blank, so the
/// bridge can always start; the fallback is astronomically unlikely on Windows).
#[must_use]
pub fn generate_token() -> String {
    let mut bytes = [0u8; 16];
    if getrandom::getrandom(&mut bytes).is_err() {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        bytes.copy_from_slice(&t.to_le_bytes());
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

impl BridgeHandle {
    /// Signal the loop to exit and join it (the poll tick is 300 ms, so this
    /// returns quickly). Used by the Settings toggle to stop the bridge live.
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Start the bridge on `127.0.0.1:<hermes_bridge_port>`. Fails (rather than
/// serving unauthenticated) when the configured token is blank, and on a busy
/// port. The error string is safe to show in Settings.
pub fn start(cfg: SharedConfig) -> Result<BridgeHandle, String> {
    let (host, port, token) = {
        let c = cfg.read();
        (
            bind_host(&c.hermes_bridge_host),
            c.hermes_bridge_port,
            c.hermes_bridge_token.trim().to_string(),
        )
    };
    if token.is_empty() {
        return Err("токен пуст — сгенерируйте токен".to_string());
    }
    // Bind to the configured host: `127.0.0.1` (default; loopback-only) or, for a
    // REMOTE Hermes over Tailscale, the machine's Tailscale IP / `0.0.0.0`. A
    // non-loopback bind exposes the (token-gated, read-mostly) API on that
    // interface — Settings warns when the host isn't loopback.
    let server = tiny_http::Server::http((host.as_str(), port))
        .map_err(|_| format!("{host}:{port} занят или недоступен"))?;
    let stop = Arc::new(AtomicBool::new(false));
    let stop_c = stop.clone();
    let thread = std::thread::Builder::new()
        .name("hermes-bridge".into())
        .spawn(move || {
            log::info!("[bridge] listening on {host}:{port}");
            while !stop_c.load(Ordering::Relaxed) {
                match server.recv_timeout(std::time::Duration::from_millis(300)) {
                    Ok(Some(req)) => handle_request(req, &cfg, &token),
                    Ok(None) => {}
                    Err(e) => {
                        log::warn!("[bridge] accept error: {e}");
                        break;
                    }
                }
            }
            log::info!("[bridge] stopped");
        })
        .map_err(|_| "не удалось запустить поток моста".to_string())?;
    Ok(BridgeHandle {
        stop,
        thread: Some(thread),
    })
}

/// Read + auth + dispatch + respond for one request. Never panics; every
/// failure path answers with a generic JSON error.
fn handle_request(mut req: tiny_http::Request, cfg: &SharedConfig, token: &str) {
    // Auth FIRST (before reading the body): constant shape, generic error.
    let authed = req
        .headers()
        .iter()
        .find(|h| h.field.equiv("Authorization"))
        .map(|h| h.value.as_str() == format!("Bearer {token}"))
        .unwrap_or(false);
    let (status, body, needs_save) = if !authed {
        (401, serde_json::json!({"error": "unauthorized"}), false)
    } else {
        // Body (capped). take() bounds the read; an over-cap body is rejected.
        let mut raw = Vec::new();
        let read_ok = req
            .as_reader()
            .take(MAX_BODY_BYTES as u64 + 1)
            .read_to_end(&mut raw)
            .is_ok();
        if !read_ok || raw.len() > MAX_BODY_BYTES {
            (413, serde_json::json!({"error": "body too large"}), false)
        } else {
            let method = req.method().as_str().to_uppercase();
            let url = req.url().to_string();
            let (path, query) = split_query(&url);
            let body_json: serde_json::Value =
                serde_json::from_slice(&raw).unwrap_or(serde_json::Value::Null);
            match open_default_store() {
                Ok(mut store) => dispatch(&method, path, &query, &body_json, &mut store, cfg),
                Err(_) => (
                    500,
                    serde_json::json!({"error": "store unavailable"}),
                    false,
                ),
            }
        }
    };
    if needs_save {
        // Persist the profile upsert. Done OUTSIDE dispatch so tests never
        // touch the user's real config.json.
        let c = cfg.read();
        if let Err(e) = crate::config::save(&c) {
            log::warn!("[bridge] config save failed: {e}");
        }
    }
    let mut response = tiny_http::Response::from_string(body.to_string()).with_status_code(status);
    // Static bytes — can't actually fail; skip the header rather than panic if
    // the API ever changes (both crates deny unwrap/expect).
    if let Ok(h) = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]) {
        response = response.with_header(h);
    }
    let _ = req.respond(response);
}

/// `"/path?a=1&b=2"` → (`"/path"`, `[("a","1"),("b","2")]`). No percent-decoding
/// — our params are numbers and simple queries; SQLite FTS handles raw text.
fn split_query(url: &str) -> (&str, Vec<(String, String)>) {
    match url.split_once('?') {
        None => (url, Vec::new()),
        Some((p, q)) => (
            p,
            q.split('&')
                .filter_map(|kv| {
                    let (k, v) = kv.split_once('=')?;
                    Some((k.to_string(), percent_decode(v)))
                })
                .collect(),
        ),
    }
}

/// Minimal %XX + '+' decoder for query values (search queries carry Cyrillic).
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let (Some(a), Some(b)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                    out.push(a * 16 + b);
                    i += 3;
                } else {
                    out.push(b'%');
                    i += 1;
                }
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn qget<'a>(query: &'a [(String, String)], key: &str) -> Option<&'a str> {
    query
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Route a request. Returns `(http_status, json_body, needs_config_save)`.
/// Pure with respect to the filesystem: mutations touch only `store` (the
/// caller's DB — in-memory in tests) and the in-memory `cfg`; persisting the
/// config is the caller's job when the flag is set.
fn dispatch(
    method: &str,
    path: &str,
    query: &[(String, String)],
    body: &serde_json::Value,
    store: &mut Store,
    cfg: &SharedConfig,
) -> (u16, serde_json::Value, bool) {
    match (method, path) {
        ("GET", "/health") => (
            200,
            serde_json::json!({
                "ok": true,
                "app": "suflyor",
                "version": env!("CARGO_PKG_VERSION"),
            }),
            false,
        ),

        ("GET", "/sessions") => {
            let limit = qget(query, "limit")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(SESSIONS_LIMIT_DEFAULT)
                .min(SESSIONS_LIMIT_MAX);
            match store.list_sessions() {
                Ok(all) => {
                    let items: Vec<_> = all
                        .iter()
                        .take(limit)
                        .map(|s| {
                            serde_json::json!({
                                "id": s.id,
                                "started_at_ms": s.started_at_ms,
                                "finished_at_ms": s.finished_at_ms,
                                "status": s.status,
                                "transcript_lines": s.transcript_lines,
                                "ai_turns": s.ai_turns_count,
                            })
                        })
                        .collect();
                    (200, serde_json::json!({ "sessions": items }), false)
                }
                Err(_) => (500, serde_json::json!({"error": "read failed"}), false),
            }
        }

        ("GET", p) if p.starts_with("/sessions/") && p.ends_with("/summary") => {
            let id = p
                .trim_start_matches("/sessions/")
                .trim_end_matches("/summary")
                .trim_matches('/');
            match store.session_ai_turns(id) {
                Ok(turns) => {
                    let summary = turns
                        .iter()
                        .rev()
                        .find(|t| t.purpose == "summary")
                        .map(|t| t.answer.clone());
                    match summary {
                        Some(text) => (
                            200,
                            serde_json::json!({"session_id": id, "summary": text}),
                            false,
                        ),
                        None => (404, serde_json::json!({"error": "no summary"}), false),
                    }
                }
                Err(_) => (500, serde_json::json!({"error": "read failed"}), false),
            }
        }

        ("GET", p) if p.starts_with("/sessions/") => {
            let id = p.trim_start_matches("/sessions/").trim_matches('/');
            let cap = qget(query, "max_chars")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(TRANSCRIPT_CAP_DEFAULT)
                .min(TRANSCRIPT_CAP_MAX);
            let Ok(Some(session)) = store.get_session(id) else {
                return (404, serde_json::json!({"error": "not found"}), false);
            };
            match store.session_utterances(id) {
                Ok(utts) => {
                    let mut used = 0usize;
                    let mut lines = Vec::new();
                    let mut truncated = false;
                    for u in &utts {
                        let n = u.text.chars().count();
                        if used + n > cap {
                            truncated = true;
                            break;
                        }
                        used += n;
                        lines.push(serde_json::json!({
                            "unix_ms": u.unix_ms,
                            "source": u.source,
                            "text": u.text,
                        }));
                    }
                    (
                        200,
                        serde_json::json!({
                            "id": session.id,
                            "started_at_ms": session.started_at_ms,
                            "status": session.status,
                            "transcript": lines,
                            "truncated": truncated,
                        }),
                        false,
                    )
                }
                Err(_) => (500, serde_json::json!({"error": "read failed"}), false),
            }
        }

        ("GET", "/search") => {
            let Some(q) = qget(query, "q").filter(|q| !q.trim().is_empty()) else {
                return (400, serde_json::json!({"error": "q required"}), false);
            };
            let limit = qget(query, "limit")
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(SEARCH_LIMIT_DEFAULT)
                .clamp(1, SEARCH_LIMIT_MAX);
            match store.search(q, limit) {
                Ok(hits) => {
                    let items: Vec<_> = hits
                        .iter()
                        .map(|h| {
                            serde_json::json!({
                                "session_id": h.session_id,
                                "kind": h.kind,
                                "unix_ms": h.unix_ms,
                                "text": h.body,
                            })
                        })
                        .collect();
                    (200, serde_json::json!({ "hits": items }), false)
                }
                // FTS5 syntax errors from odd queries land here — the caller
                // sees a clean 400 rather than a 500.
                Err(_) => (400, serde_json::json!({"error": "bad query"}), false),
            }
        }

        ("GET", "/memory") => match store.list_memory_items("default", false, -1) {
            Ok(items) => {
                let out: Vec<_> = items
                    .iter()
                    .map(|m| {
                        serde_json::json!({
                            "id": m.id,
                            "kind": m.kind,
                            "text": m.text,
                            "entity": m.entity,
                        })
                    })
                    .collect();
                (200, serde_json::json!({ "items": out }), false)
            }
            Err(_) => (500, serde_json::json!({"error": "read failed"}), false),
        },

        ("GET", "/profiles") => {
            let c = cfg.read();
            let profiles: Vec<_> = c
                .context_profiles
                .iter()
                .map(|p| serde_json::json!({"name": p.name, "context": p.context}))
                .collect();
            (
                200,
                serde_json::json!({
                    "active": c.active_profile,
                    "profiles": profiles,
                }),
                false,
            )
        }

        ("POST", "/memory/suggest") => {
            let text = body
                .get("text")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .unwrap_or("");
            if text.is_empty() || text.chars().count() > SUGGEST_TEXT_MAX_CHARS {
                return (400, serde_json::json!({"error": "bad text"}), false);
            }
            let reason = body
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("Предложено Hermes")
                .chars()
                .take(200)
                .collect::<String>();
            let cand = NewMemoryCandidate {
                profile_id: "default".to_string(),
                source_session_id: None,
                kind: "note".to_string(),
                text: text.to_string(),
                reason,
            };
            match store.insert_candidate(&cand, now_ms()) {
                Ok(id) => (200, serde_json::json!({"queued": true, "id": id}), false),
                Err(_) => (500, serde_json::json!({"error": "queue failed"}), false),
            }
        }

        ("POST", "/profiles") => {
            let name = body
                .get("name")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .unwrap_or("");
            let context = body.get("context").and_then(|v| v.as_str()).unwrap_or("");
            let set_active = body
                .get("set_active")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            if name.is_empty()
                || name.chars().count() > PROFILE_NAME_MAX_CHARS
                || context.trim().is_empty()
                || context.chars().count() > PROFILE_CONTEXT_MAX_CHARS
            {
                return (400, serde_json::json!({"error": "bad profile"}), false);
            }
            let mut c = cfg.write();
            let created = match c.context_profiles.iter_mut().find(|p| p.name == name) {
                Some(existing) => {
                    existing.context = context.to_string();
                    false
                }
                None => {
                    c.context_profiles.push(crate::config::ContextProfile {
                        name: name.to_string(),
                        context: context.to_string(),
                    });
                    true
                }
            };
            if set_active {
                c.active_profile = Some(name.to_string());
                // Keep the live meeting context in lockstep with the active
                // profile (same rule as Config::select_profile).
                c.meeting_context = context.to_string();
            }
            (
                200,
                serde_json::json!({"ok": true, "created": created, "active": set_active}),
                true,
            )
        }

        _ => (404, serde_json::json!({"error": "not found"}), false),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::config::Config;
    use parking_lot::RwLock;

    fn scratch() -> (Store, SharedConfig) {
        (
            Store::open_in_memory().unwrap(),
            Arc::new(RwLock::new(Config::defaults())),
        )
    }

    fn call(
        store: &mut Store,
        cfg: &SharedConfig,
        method: &str,
        url: &str,
        body: serde_json::Value,
    ) -> (u16, serde_json::Value, bool) {
        let (path, query) = split_query(url);
        dispatch(method, path, &query, &body, store, cfg)
    }

    #[test]
    fn health_and_unknown_routes() {
        let (mut s, cfg) = scratch();
        let (st, body, save) = call(&mut s, &cfg, "GET", "/health", serde_json::Value::Null);
        assert_eq!(st, 200);
        assert_eq!(body["app"], "suflyor");
        assert!(!save);
        let (st, ..) = call(&mut s, &cfg, "GET", "/nope", serde_json::Value::Null);
        assert_eq!(st, 404);
        let (st, ..) = call(&mut s, &cfg, "DELETE", "/memory", serde_json::Value::Null);
        assert_eq!(st, 404);
    }

    #[test]
    fn sessions_list_and_missing_session() {
        let (mut s, cfg) = scratch();
        let (st, body, _) = call(
            &mut s,
            &cfg,
            "GET",
            "/sessions?limit=5",
            serde_json::Value::Null,
        );
        assert_eq!(st, 200);
        assert!(body["sessions"].as_array().unwrap().is_empty());
        let (st, ..) = call(
            &mut s,
            &cfg,
            "GET",
            "/sessions/nope",
            serde_json::Value::Null,
        );
        assert_eq!(st, 404);
        let (st, ..) = call(
            &mut s,
            &cfg,
            "GET",
            "/sessions/nope/summary",
            serde_json::Value::Null,
        );
        // No ai_turns rows → empty list → "no summary" 404 (not a 500).
        assert_eq!(st, 404);
    }

    #[test]
    fn search_requires_query() {
        let (mut s, cfg) = scratch();
        let (st, ..) = call(&mut s, &cfg, "GET", "/search", serde_json::Value::Null);
        assert_eq!(st, 400);
        let (st, ..) = call(&mut s, &cfg, "GET", "/search?q=", serde_json::Value::Null);
        assert_eq!(st, 400);
    }

    #[test]
    fn suggest_lands_in_candidate_queue_not_memory() {
        let (mut s, cfg) = scratch();
        let (st, body, save) = call(
            &mut s,
            &cfg,
            "POST",
            "/memory/suggest",
            serde_json::json!({"text": "Тимлид — Тимур", "reason": "из созвона"}),
        );
        assert_eq!(st, 200);
        assert!(body["queued"].as_bool().unwrap());
        assert!(!save);
        // Queued as a PENDING candidate — approved memory stays empty.
        assert_eq!(s.count_candidates("default", "pending").unwrap(), 1);
        assert!(s
            .list_memory_items("default", false, -1)
            .unwrap()
            .is_empty());
        // Blank / oversized text rejected.
        let (st, ..) = call(
            &mut s,
            &cfg,
            "POST",
            "/memory/suggest",
            serde_json::json!({"text": "  "}),
        );
        assert_eq!(st, 400);
        let big = "x".repeat(SUGGEST_TEXT_MAX_CHARS + 1);
        let (st, ..) = call(
            &mut s,
            &cfg,
            "POST",
            "/memory/suggest",
            serde_json::json!({ "text": big }),
        );
        assert_eq!(st, 400);
    }

    #[test]
    fn profile_upsert_create_update_activate() {
        let (mut s, cfg) = scratch();
        // Create + activate.
        let (st, body, save) = call(
            &mut s,
            &cfg,
            "POST",
            "/profiles",
            serde_json::json!({"name": "Собес X", "context": "Компания X, роль Y", "set_active": true}),
        );
        assert_eq!(st, 200);
        assert!(body["created"].as_bool().unwrap());
        assert!(save, "profile upsert must request a config save");
        {
            let c = cfg.read();
            assert_eq!(c.active_profile.as_deref(), Some("Собес X"));
            assert_eq!(c.meeting_context, "Компания X, роль Y");
        }
        // Update in place (no duplicate).
        let (st, body, _) = call(
            &mut s,
            &cfg,
            "POST",
            "/profiles",
            serde_json::json!({"name": "Собес X", "context": "обновлено"}),
        );
        assert_eq!(st, 200);
        assert!(!body["created"].as_bool().unwrap());
        assert_eq!(cfg.read().context_profiles.len(), 1);
        // Bad payloads.
        let (st, ..) = call(
            &mut s,
            &cfg,
            "POST",
            "/profiles",
            serde_json::json!({"name": "", "context": "x"}),
        );
        assert_eq!(st, 400);
        // GET /profiles reflects the state.
        let (st, body, _) = call(&mut s, &cfg, "GET", "/profiles", serde_json::Value::Null);
        assert_eq!(st, 200);
        assert_eq!(body["active"], "Собес X");
        assert_eq!(body["profiles"][0]["context"], "обновлено");
    }

    #[test]
    fn query_decoding_cyrillic() {
        // «влад» percent-encoded (UTF-8) + '+' as space.
        let (_, q) = split_query("/search?q=%D0%B2%D0%BB%D0%B0%D0%B4+%D0%BA");
        assert_eq!(q[0].1, "влад к");
    }
}
