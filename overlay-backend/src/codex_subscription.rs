//! Experimental ChatGPT-subscription sign-in through the official Codex
//! `app-server` stdio protocol.
//!
//! Suflyor never reads or writes Codex tokens. The official child process owns
//! the device-code flow and stores credentials in Windows Credential Manager
//! (`cli_auth_credentials_store = "keyring"`) under an isolated `CODEX_HOME`.
//! Experimental live answers use only the official app-server protocol. Every
//! turn is ephemeral, model-pinned, environment-less, and confined to
//! Suflyor's empty workspace under an app-owned permission profile that denies
//! all model-command filesystem and network access. Unexpected tool activity
//! is interrupted and any protocol drift or added capability fails closed.

use crate::ai::{ChatMessage, ContentPart, MessageContent, TokenUsage};
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
const LOGIN_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const TURN_TIMEOUT: Duration = Duration::from_secs(3 * 60);
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const SECURE_PROFILE: &str = "suflyor-text-only";
const MAX_MODEL_PAGES: usize = 64;
const MAX_MODELS_PER_PAGE: usize = 100;
const MAX_MODELS_TOTAL: usize = 1_000;
const MAX_CURSOR_BYTES: usize = 4_096;
const MAX_DELTA_BYTES: usize = 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const APP_SERVER_ARGS: &[&str] = &[
    "app-server",
    "--stdio",
    "--strict-config",
    "-c",
    "cli_auth_credentials_store=\"keyring\"",
    "-c",
    "windows.sandbox=\"elevated\"",
    "-c",
    "default_permissions=\"suflyor-text-only\"",
    "-c",
    "permissions.suflyor-text-only.filesystem.:root=\"deny\"",
    "-c",
    "permissions.suflyor-text-only.network.enabled=false",
];

static LOGIN_GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct LoginAttempt(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountState {
    NotInstalled,
    SignedOut,
    SignInRequired,
    SignedIn { plan: Option<String> },
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginEvent {
    AwaitingUser {
        verification_url: String,
        user_code: String,
    },
    SignedIn {
        plan: Option<String>,
    },
    SignInRequired,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexModel {
    pub id: String,
    pub display_name: String,
    pub is_default: bool,
    pub default_reasoning_effort: Option<String>,
    pub reasoning_efforts: Vec<String>,
    pub input_modalities: Vec<String>,
}

impl CodexModel {
    #[must_use]
    pub fn picker_label(&self) -> String {
        let mut details = Vec::new();
        if self.is_default {
            details.push("default".to_string());
        }
        if !self.reasoning_efforts.is_empty() {
            let efforts = self.reasoning_efforts.join("/");
            details.push(self.default_reasoning_effort.as_ref().map_or_else(
                || format!("reasoning: {efforts}"),
                |default| format!("reasoning: {efforts}; default {default}"),
            ));
        } else if let Some(effort) = &self.default_reasoning_effort {
            details.push(format!("reasoning: {effort}"));
        }
        if !self.input_modalities.is_empty() {
            details.push(self.input_modalities.join("+"));
        }
        if details.is_empty() {
            self.display_name.clone()
        } else {
            format!("{} ({})", self.display_name, details.join(", "))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSnapshot {
    pub account: AccountState,
    pub models: Vec<CodexModel>,
    /// Sanitized account-window summary. It never contains account ids, URLs,
    /// credentials, or raw server error text.
    pub rate_limits: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnEvent {
    Start { id: String },
    Delta { text: String },
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnFailure {
    NotInstalled,
    SignedOut,
    InvalidModel,
    UnsupportedSecurityProfile,
    SecurityViolation,
    ModelMismatch,
    Cancelled,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TurnNotification {
    Delta(String),
    Usage { input: u64, output: u64 },
    Completed,
    SafeItemLifecycle,
    SafeReasoningUpdate,
    RateLimitsUpdated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RpcFailure {
    NotInstalled,
    Io,
    Timeout,
    Protocol,
    Cancelled,
    Security,
    ModelMismatch,
    SignedOut,
    InvalidModel,
    UnsupportedSecurity,
}

struct AppServer {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<Result<Value, RpcFailure>>,
}

impl Drop for AppServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl AppServer {
    fn start() -> Result<Self, RpcFailure> {
        let (codex_home, workspace) = isolated_paths()?;
        fs::create_dir_all(&codex_home).map_err(|_| RpcFailure::Io)?;
        fs::create_dir_all(&workspace).map_err(|_| RpcFailure::Io)?;
        let mut child = codex_executable_candidates()
            .into_iter()
            .find_map(|executable| spawn_app_server(&executable, &codex_home, &workspace).ok())
            .ok_or(RpcFailure::NotInstalled)?;
        let stdin = child.stdin.take().ok_or(RpcFailure::Io)?;
        let stdout = child.stdout.take().ok_or(RpcFailure::Io)?;
        let stderr = child.stderr.take().ok_or(RpcFailure::Io)?;
        let (sender, messages) = mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let value = line
                    .map_err(|_| RpcFailure::Io)
                    .and_then(|line| serde_json::from_str(&line).map_err(|_| RpcFailure::Protocol));
                if sender.send(value).is_err() {
                    break;
                }
            }
        });
        std::thread::spawn(move || {
            let mut seen = std::collections::HashSet::new();
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if let Some(class) = classify_app_server_stderr(&line) {
                    if seen.insert(class) {
                        eprintln!("[suflyor-codex] app-server diagnostic={class}");
                    }
                }
            }
        });
        Ok(Self {
            child,
            stdin,
            messages,
        })
    }

    fn initialize(&mut self) -> Result<(), RpcFailure> {
        self.initialize_with_experimental(false)
    }

    fn initialize_experimental(&mut self) -> Result<(), RpcFailure> {
        self.initialize_with_experimental(true)
    }

    fn initialize_with_experimental(&mut self, experimental: bool) -> Result<(), RpcFailure> {
        self.request(1, "initialize", initialize_params(experimental))?;
        self.write(json!({"method":"initialized","params":{}}))?;
        Ok(())
    }

    fn write(&mut self, value: Value) -> Result<(), RpcFailure> {
        serde_json::to_writer(&mut self.stdin, &value).map_err(|_| RpcFailure::Io)?;
        self.stdin.write_all(b"\n").map_err(|_| RpcFailure::Io)?;
        self.stdin.flush().map_err(|_| RpcFailure::Io)
    }

    fn request(&mut self, id: u64, method: &str, params: Value) -> Result<Value, RpcFailure> {
        self.write(rpc_request(id, method, params))?;
        self.wait_for_response(id, REQUEST_TIMEOUT)
    }

    fn request_secure(
        &mut self,
        id: u64,
        method: &str,
        params: Value,
    ) -> Result<Value, RpcFailure> {
        self.write(rpc_request(id, method, params))?;
        let deadline = Instant::now() + REQUEST_TIMEOUT;
        let message = recv_until(&self.messages, deadline)?;
        if message.get("method").is_some() || message.get("id").and_then(Value::as_u64) != Some(id)
        {
            return Err(RpcFailure::Security);
        }
        if message.get("error").is_some() {
            return Err(RpcFailure::Protocol);
        }
        message.get("result").cloned().ok_or(RpcFailure::Protocol)
    }

    fn wait_for_response(&self, id: u64, timeout: Duration) -> Result<Value, RpcFailure> {
        let deadline = Instant::now() + timeout;
        loop {
            let message = recv_until(&self.messages, deadline)?;
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if message.get("error").is_some() {
                return Err(RpcFailure::Protocol);
            }
            return message.get("result").cloned().ok_or(RpcFailure::Protocol);
        }
    }
}

fn initialize_params(experimental: bool) -> Value {
    json!({
        "clientInfo": {
            "name": "suflyor",
            "title": "suflyor",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "capabilities": {
            "experimentalApi": experimental,
            "mcpServerOpenaiFormElicitation": false,
            "requestAttestation": false,
            "optOutNotificationMethods": [
                "thread/started",
                "turn/started"
            ]
        }
    })
}

/// Read account state, the complete account-aware model catalog, and the
/// official rate-limit snapshot in a single isolated app-server session.
#[must_use]
pub fn provider_snapshot() -> ProviderSnapshot {
    match provider_snapshot_inner() {
        Ok(snapshot) => snapshot,
        Err(RpcFailure::NotInstalled) => ProviderSnapshot {
            account: AccountState::NotInstalled,
            models: Vec::new(),
            rate_limits: None,
        },
        Err(_) => ProviderSnapshot {
            account: AccountState::Error,
            models: Vec::new(),
            rate_limits: None,
        },
    }
}

fn provider_snapshot_inner() -> Result<ProviderSnapshot, RpcFailure> {
    let mut server = AppServer::start()?;
    server.initialize_experimental()?;
    let account =
        parse_account_state(&server.request(2, "account/read", json!({"refreshToken":false}))?)?;
    if !matches!(account, AccountState::SignedIn { .. }) {
        return Ok(ProviderSnapshot {
            account,
            models: Vec::new(),
            rate_limits: None,
        });
    }
    let models = list_models_paginated(&mut server, 10)?;
    let rate_limits = server
        .request(100, "account/rateLimits/read", Value::Null)
        .ok()
        .and_then(|value| parse_rate_limits(&value));
    Ok(ProviderSnapshot {
        account,
        models,
        rate_limits,
    })
}

fn list_models_paginated(
    server: &mut AppServer,
    first_id: u64,
) -> Result<Vec<CodexModel>, RpcFailure> {
    let mut models = Vec::new();
    let mut cursor: Option<String> = None;
    for request_id in (first_id..).take(MAX_MODEL_PAGES) {
        let page = server.request(
            request_id,
            "model/list",
            json!({"cursor":cursor,"includeHidden":false,"limit":100}),
        )?;
        cursor = append_model_page(&mut models, &page)?;
        if cursor.is_none() {
            return Ok(models);
        }
    }
    Err(RpcFailure::Protocol)
}

fn append_model_page(
    models: &mut Vec<CodexModel>,
    page: &Value,
) -> Result<Option<String>, RpcFailure> {
    let data = page
        .get("data")
        .and_then(Value::as_array)
        .ok_or(RpcFailure::Protocol)?;
    if data.len() > MAX_MODELS_PER_PAGE
        || models.len().saturating_add(data.len()) > MAX_MODELS_TOTAL
    {
        return Err(RpcFailure::Protocol);
    }
    for value in data {
        let model = parse_model(value)?;
        if models
            .iter()
            .any(|existing: &CodexModel| existing.id == model.id)
        {
            return Err(RpcFailure::Protocol);
        }
        models.push(model);
    }
    match page.get("nextCursor") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(cursor)) if !cursor.is_empty() && cursor.len() <= MAX_CURSOR_BYTES => {
            Ok(Some(cursor.clone()))
        }
        _ => Err(RpcFailure::Protocol),
    }
}

fn parse_model(value: &Value) -> Result<CodexModel, RpcFailure> {
    let id = value
        .get("model")
        .and_then(Value::as_str)
        .and_then(safe_model_id)
        .ok_or(RpcFailure::Protocol)?;
    let display_name = value
        .get("displayName")
        .and_then(Value::as_str)
        .and_then(safe_display_label)
        .unwrap_or_else(|| id.clone());
    let reasoning_efforts = reasoning_effort_array(value.get("supportedReasoningEfforts"))?;
    let input_modalities = string_array(value.get("inputModalities"), 8, 24)?;
    let default_reasoning_effort = value
        .get("defaultReasoningEffort")
        .and_then(Value::as_str)
        .and_then(safe_short_label);
    Ok(CodexModel {
        id,
        display_name,
        is_default: value
            .get("isDefault")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        default_reasoning_effort,
        reasoning_efforts,
        input_modalities,
    })
}

fn string_array(
    value: Option<&Value>,
    max_items: usize,
    max_len: usize,
) -> Result<Vec<String>, RpcFailure> {
    let Some(array) = value.and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    if array.len() > max_items {
        return Err(RpcFailure::Protocol);
    }
    array
        .iter()
        .map(|item| {
            item.as_str()
                .filter(|text| !text.is_empty() && text.len() <= max_len && text.is_ascii())
                .map(str::to_string)
                .ok_or(RpcFailure::Protocol)
        })
        .collect()
}

fn reasoning_effort_array(value: Option<&Value>) -> Result<Vec<String>, RpcFailure> {
    let Some(array) = value.and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    if array.len() > 16 {
        return Err(RpcFailure::Protocol);
    }
    array
        .iter()
        .map(|item| {
            item.get("reasoningEffort")
                .and_then(Value::as_str)
                .and_then(safe_short_label)
                .ok_or(RpcFailure::Protocol)
        })
        .collect()
}

fn parse_rate_limits(value: &Value) -> Option<String> {
    let snapshot = value.get("rateLimits").unwrap_or(value);
    let primary = snapshot.get("primary")?;
    let used = primary.get("usedPercent")?.as_i64()?.clamp(0, 100);
    let window = primary.get("windowDurationMins").and_then(Value::as_i64);
    Some(window.map_or_else(
        || format!("{used}% used"),
        |minutes| format!("{used}% used / {minutes} min"),
    ))
}

fn safe_model_id(value: &str) -> Option<String> {
    (!value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/' | b':')
        }))
    .then(|| value.to_string())
}

fn safe_short_label(value: &str) -> Option<String> {
    (!value.is_empty() && value.len() <= 32 && value.is_ascii()).then(|| value.to_string())
}

fn safe_display_label(value: &str) -> Option<String> {
    (!value.is_empty()
        && value.len() <= 96
        && value.is_ascii()
        && !value.chars().any(char::is_control))
    .then(|| value.to_string())
}

/// Read current subscription-account state without exposing email or tokens.
#[must_use]
pub fn account_state() -> AccountState {
    match account_state_inner() {
        Ok(state) => state,
        Err(RpcFailure::NotInstalled) => AccountState::NotInstalled,
        Err(_) => AccountState::Error,
    }
}

fn account_state_inner() -> Result<AccountState, RpcFailure> {
    let mut server = AppServer::start()?;
    server.initialize()?;
    let result = server.request(2, "account/read", json!({"refreshToken":false}))?;
    parse_account_state(&result)
}

/// Reserve a device-login attempt before its worker thread starts. Starting a
/// new attempt invalidates any older one, including workers not scheduled yet.
#[must_use]
pub fn begin_device_login() -> LoginAttempt {
    LoginAttempt(LOGIN_GENERATION.fetch_add(1, Ordering::SeqCst) + 1)
}

/// Cancel a pending device login without changing the stored account.
pub fn cancel_pending_login() {
    LOGIN_GENERATION.fetch_add(1, Ordering::SeqCst);
}

/// Run the official device-code flow. The caller decides how to present and
/// open the verification URL; this function never launches a browser.
pub fn device_login(attempt: LoginAttempt, mut notify: impl FnMut(LoginEvent)) {
    let result = device_login_inner(attempt.0, &mut notify);
    match result {
        Ok(()) | Err(RpcFailure::Cancelled) => {}
        Err(RpcFailure::NotInstalled) => notify(LoginEvent::Error),
        Err(_) => notify(LoginEvent::Error),
    }
}

fn device_login_inner(
    generation: u64,
    notify: &mut impl FnMut(LoginEvent),
) -> Result<(), RpcFailure> {
    ensure_current_login(generation)?;
    let mut server = AppServer::start()?;
    server.initialize()?;
    ensure_current_login(generation)?;

    let current = server.request(2, "account/read", json!({"refreshToken":false}))?;
    ensure_current_login(generation)?;
    if let AccountState::SignedIn { plan } = parse_account_state(&current)? {
        notify(LoginEvent::SignedIn { plan });
        return Ok(());
    }

    let started = server.request(
        3,
        "account/login/start",
        json!({"type":"chatgptDeviceCode"}),
    )?;
    ensure_current_login(generation)?;
    let login_id = started
        .get("loginId")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(RpcFailure::Protocol)?
        .to_string();
    let verification_url = started
        .get("verificationUrl")
        .and_then(Value::as_str)
        .filter(|value| allowed_signin_url(value))
        .ok_or(RpcFailure::Protocol)?
        .to_string();
    let user_code = started
        .get("userCode")
        .and_then(Value::as_str)
        .filter(|value| valid_user_code(value))
        .ok_or(RpcFailure::Protocol)?
        .to_string();
    ensure_current_login(generation)?;
    notify(LoginEvent::AwaitingUser {
        verification_url,
        user_code,
    });

    let deadline = Instant::now() + LOGIN_TIMEOUT;
    loop {
        ensure_current_login(generation)?;
        let slice_deadline = deadline.min(Instant::now() + Duration::from_secs(1));
        let message = match recv_until(&server.messages, slice_deadline) {
            Ok(message) => message,
            Err(RpcFailure::Timeout) if Instant::now() < deadline => continue,
            Err(error) => return Err(error),
        };
        match parse_login_notification(&message, &login_id)? {
            LoginProgress::Ignore => {}
            LoginProgress::Failed => {
                notify(LoginEvent::SignInRequired);
                return Ok(());
            }
            LoginProgress::Completed => {
                let state = server.request(4, "account/read", json!({"refreshToken":false}))?;
                match parse_account_state(&state)? {
                    AccountState::SignedIn { plan } => notify(LoginEvent::SignedIn { plan }),
                    _ => notify(LoginEvent::SignInRequired),
                }
                return Ok(());
            }
        }
    }
}

fn ensure_current_login(generation: u64) -> Result<(), RpcFailure> {
    (LOGIN_GENERATION.load(Ordering::SeqCst) == generation)
        .then_some(())
        .ok_or(RpcFailure::Cancelled)
}

/// Cancel a pending login and ask the official app-server to clear its stored
/// subscription credential. No credential bytes enter Suflyor memory.
#[must_use]
pub fn disconnect() -> AccountState {
    cancel_pending_login();
    match disconnect_inner() {
        Ok(()) => AccountState::SignedOut,
        Err(RpcFailure::NotInstalled) => AccountState::NotInstalled,
        Err(_) => AccountState::Error,
    }
}

fn disconnect_inner() -> Result<(), RpcFailure> {
    let mut server = AppServer::start()?;
    server.initialize()?;
    let _ = server.request(2, "account/logout", json!({}))?;
    Ok(())
}

/// Execute one model-pinned, text-only turn. Returning `false` from `notify`
/// cancels the turn and tears down the short-lived app-server child.
pub fn run_turn(
    model: &str,
    messages: &[ChatMessage],
    mut notify: impl FnMut(TurnEvent) -> bool,
) -> Result<TokenUsage, TurnFailure> {
    run_turn_inner(model, messages, &mut notify).map_err(map_turn_failure)
}

fn map_turn_failure(failure: RpcFailure) -> TurnFailure {
    match failure {
        RpcFailure::NotInstalled => TurnFailure::NotInstalled,
        RpcFailure::Cancelled => TurnFailure::Cancelled,
        RpcFailure::Security => TurnFailure::SecurityViolation,
        RpcFailure::ModelMismatch => TurnFailure::ModelMismatch,
        RpcFailure::SignedOut => TurnFailure::SignedOut,
        RpcFailure::InvalidModel => TurnFailure::InvalidModel,
        RpcFailure::UnsupportedSecurity => TurnFailure::UnsupportedSecurityProfile,
        RpcFailure::Protocol | RpcFailure::Io | RpcFailure::Timeout => TurnFailure::Unavailable,
    }
}

fn run_turn_inner(
    model: &str,
    messages: &[ChatMessage],
    notify: &mut impl FnMut(TurnEvent) -> bool,
) -> Result<TokenUsage, RpcFailure> {
    let selected_model = safe_model_id(model).ok_or(RpcFailure::InvalidModel)?;
    let prompt = text_only_prompt(messages)?;
    let (_, workspace) = isolated_paths()?;
    fs::create_dir_all(&workspace).map_err(|_| RpcFailure::Io)?;
    ensure_workspace_empty(&workspace)?;
    let workspace_text = workspace.to_string_lossy().to_string();

    let mut server = AppServer::start()?;
    server.initialize_experimental()?;
    let account = server.request(2, "account/read", json!({"refreshToken":false}))?;
    if !matches!(
        parse_account_state(&account)?,
        AccountState::SignedIn { .. }
    ) {
        return Err(RpcFailure::SignedOut);
    }
    require_secure_profile(&mut server, &workspace_text)?;

    let thread_result = server.request_secure(
        20,
        "thread/start",
        secure_thread_params(&selected_model, &workspace_text),
    )?;
    let thread_id = validate_thread_contract(&thread_result, &selected_model, &workspace)?;
    let turn_request = secure_turn_params(&thread_id, &selected_model, &workspace_text, &prompt);
    server.write(rpc_request(21, "turn/start", turn_request))?;

    let mut turn_id: Option<String> = None;
    let result = receive_turn(&mut server, &thread_id, &mut turn_id, notify);
    if result.is_err() {
        interrupt_turn(&mut server, &thread_id, turn_id.as_deref());
    }
    result
}

fn receive_turn(
    server: &mut AppServer,
    thread_id: &str,
    turn_id: &mut Option<String>,
    notify: &mut impl FnMut(TurnEvent) -> bool,
) -> Result<TokenUsage, RpcFailure> {
    let mut usage = TokenUsage {
        finish_reason: "stop".into(),
        ..TokenUsage::default()
    };
    let mut output_bytes = 0_usize;
    let deadline = Instant::now() + TURN_TIMEOUT;
    loop {
        let message = recv_until(&server.messages, deadline)?;
        if message.get("method").is_some() && message.get("id").is_some() {
            return Err(RpcFailure::Security);
        }
        if message.get("id").and_then(Value::as_u64) == Some(21) {
            if turn_id.is_some() {
                return Err(RpcFailure::Protocol);
            }
            if message.get("error").is_some() {
                return Err(RpcFailure::Protocol);
            }
            let result = message.get("result").ok_or(RpcFailure::Protocol)?;
            let turn = result.get("turn").ok_or(RpcFailure::Protocol)?;
            validate_safe_items(turn.get("items"))?;
            let id = turn
                .get("id")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or(RpcFailure::Protocol)?
                .to_string();
            *turn_id = Some(id.clone());
            emit_turn_event(notify, TurnEvent::Start { id })?;
            continue;
        }
        let active_turn = turn_id.as_deref().ok_or(RpcFailure::Protocol)?;
        let parsed = parse_turn_notification(&message, thread_id, active_turn)?;
        match parsed {
            TurnNotification::Delta(text) => {
                output_bytes = output_bytes.saturating_add(text.len());
                if output_bytes > MAX_OUTPUT_BYTES {
                    return Err(RpcFailure::Protocol);
                }
                emit_turn_event(notify, TurnEvent::Delta { text })?;
            }
            TurnNotification::Usage { input, output } => {
                usage.input = input;
                usage.output = output;
            }
            TurnNotification::Completed => {
                emit_turn_event(notify, TurnEvent::Done)?;
                return Ok(usage);
            }
            TurnNotification::SafeItemLifecycle
            | TurnNotification::SafeReasoningUpdate
            | TurnNotification::RateLimitsUpdated => {}
        }
    }
}

fn emit_turn_event(
    notify: &mut impl FnMut(TurnEvent) -> bool,
    event: TurnEvent,
) -> Result<(), RpcFailure> {
    notify(event).then_some(()).ok_or(RpcFailure::Cancelled)
}

fn parse_turn_notification(
    message: &Value,
    thread_id: &str,
    turn_id: &str,
) -> Result<TurnNotification, RpcFailure> {
    match message.get("method").and_then(Value::as_str) {
        Some("item/agentMessage/delta") => {
            let params = matching_turn_params(message, thread_id, Some(turn_id))?;
            let delta = params
                .get("delta")
                .and_then(Value::as_str)
                .filter(|text| text.len() <= MAX_DELTA_BYTES)
                .ok_or(RpcFailure::Protocol)?;
            Ok(TurnNotification::Delta(delta.to_string()))
        }
        Some("thread/tokenUsage/updated") => {
            let params = matching_turn_params(message, thread_id, Some(turn_id))?;
            let last = params
                .get("tokenUsage")
                .and_then(|value| value.get("last"))
                .ok_or(RpcFailure::Protocol)?;
            Ok(TurnNotification::Usage {
                input: nonnegative_u64(last.get("inputTokens")),
                output: nonnegative_u64(last.get("outputTokens")),
            })
        }
        Some("turn/completed") => {
            let turn = matching_completed_turn(message, thread_id, turn_id)?;
            if turn.get("status").and_then(Value::as_str) != Some("completed") {
                return Err(RpcFailure::Protocol);
            }
            validate_safe_items(turn.get("items"))?;
            Ok(TurnNotification::Completed)
        }
        Some("model/rerouted") => Err(RpcFailure::ModelMismatch),
        Some("account/rateLimits/updated") => message
            .get("params")
            .and_then(|params| params.get("rateLimits"))
            .filter(|rate_limits| rate_limits.is_object())
            .map(|_| TurnNotification::RateLimitsUpdated)
            .ok_or(RpcFailure::Protocol),
        Some("item/started") | Some("item/completed") => {
            let params = matching_turn_params(message, thread_id, Some(turn_id))?;
            matches!(
                params
                    .get("item")
                    .and_then(|value| value.get("type"))
                    .and_then(Value::as_str),
                Some("agentMessage" | "reasoning")
            )
            .then_some(TurnNotification::SafeItemLifecycle)
            .ok_or(RpcFailure::Security)
        }
        Some(
            "item/reasoning/summaryTextDelta"
            | "item/reasoning/summaryPartAdded"
            | "item/reasoning/textDelta",
        ) => {
            let _ = matching_turn_params(message, thread_id, Some(turn_id))?;
            Ok(TurnNotification::SafeReasoningUpdate)
        }
        _ => Err(RpcFailure::Security),
    }
}

fn validate_safe_items(items: Option<&Value>) -> Result<(), RpcFailure> {
    let items = items
        .and_then(Value::as_array)
        .ok_or(RpcFailure::Protocol)?;
    if items.len() > 128 {
        return Err(RpcFailure::Protocol);
    }
    items
        .iter()
        .all(|item| {
            matches!(
                item.get("type").and_then(Value::as_str),
                Some("agentMessage" | "reasoning")
            )
        })
        .then_some(())
        .ok_or(RpcFailure::Security)
}

fn text_only_prompt(messages: &[ChatMessage]) -> Result<String, RpcFailure> {
    if messages.is_empty() {
        return Err(RpcFailure::Protocol);
    }
    let mut prompt = String::new();
    for message in messages {
        let role = match message.role.as_str() {
            "system" | "user" | "assistant" => message.role.as_str(),
            _ => return Err(RpcFailure::Protocol),
        };
        let text = match &message.content {
            MessageContent::Text(text) => text.clone(),
            MessageContent::Parts(parts) => {
                if parts
                    .iter()
                    .any(|part| !matches!(part, ContentPart::Text { .. }))
                {
                    return Err(RpcFailure::Security);
                }
                parts
                    .iter()
                    .filter_map(|part| match part {
                        ContentPart::Text { text } => Some(text.as_str()),
                        ContentPart::ImageUrl { .. } => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        };
        if prompt.len().saturating_add(text.len()) > 2 * 1024 * 1024 {
            return Err(RpcFailure::Protocol);
        }
        prompt.push_str(role);
        prompt.push_str(":\n");
        prompt.push_str(&text);
        prompt.push_str("\n\n");
    }
    Ok(prompt)
}

fn secure_thread_params(model: &str, workspace: &str) -> Value {
    json!({
        "model": model,
        "cwd": workspace,
        "ephemeral": true,
        "approvalPolicy": "never",
        "permissions": SECURE_PROFILE,
        "runtimeWorkspaceRoots": [workspace],
        "selectedCapabilityRoots": [],
        "environments": [],
        "dynamicTools": [],
        "allowProviderModelFallback": false,
        "experimentalRawEvents": false,
        "baseInstructions": "Return only a text answer. Do not use tools, files, web, apps, skills, plugins, hooks, MCP, collaboration, or environments.",
        "config": {
            "web_search": "disabled",
            "mcp_servers": {},
            "shell_environment_policy": {"inherit": "none"}
        }
    })
}

fn secure_turn_params(thread_id: &str, model: &str, workspace: &str, prompt: &str) -> Value {
    json!({
        "threadId": thread_id,
        "model": model,
        "cwd": workspace,
        "approvalPolicy": "never",
        "permissions": SECURE_PROFILE,
        "runtimeWorkspaceRoots": [workspace],
        "environments": [],
        "input": [{"type":"text","text":prompt,"text_elements":[]}]
    })
}

fn require_secure_profile(server: &mut AppServer, workspace: &str) -> Result<(), RpcFailure> {
    let result = server.request_secure(
        3,
        "permissionProfile/list",
        json!({"cwd":workspace,"cursor":null,"limit":100}),
    )?;
    let data = result
        .get("data")
        .and_then(Value::as_array)
        .ok_or(RpcFailure::Protocol)?;
    data.iter()
        .any(|profile| {
            profile.get("id").and_then(Value::as_str) == Some(SECURE_PROFILE)
                && profile.get("allowed").and_then(Value::as_bool) == Some(true)
        })
        .then_some(())
        .ok_or(RpcFailure::UnsupportedSecurity)
}

fn validate_thread_contract(
    result: &Value,
    model: &str,
    workspace: &Path,
) -> Result<String, RpcFailure> {
    if result.get("model").and_then(Value::as_str) != Some(model) {
        return Err(RpcFailure::ModelMismatch);
    }
    if result.get("approvalPolicy").and_then(Value::as_str) != Some("never")
        || result
            .get("activePermissionProfile")
            .and_then(|value| value.get("id"))
            .and_then(Value::as_str)
            != Some(SECURE_PROFILE)
        || result
            .get("sandbox")
            .and_then(|value| value.get("type"))
            .and_then(Value::as_str)
            != Some("readOnly")
        || result
            .get("sandbox")
            .and_then(|value| value.get("networkAccess"))
            .and_then(Value::as_bool)
            != Some(false)
        || result
            .get("instructionSources")
            .and_then(Value::as_array)
            .is_none_or(|sources| !sources.is_empty())
    {
        return Err(RpcFailure::Security);
    }
    let cwd = result
        .get("cwd")
        .and_then(Value::as_str)
        .ok_or(RpcFailure::Protocol)?;
    if !paths_match(Path::new(cwd), workspace) {
        return Err(RpcFailure::Security);
    }
    let roots = result
        .get("runtimeWorkspaceRoots")
        .and_then(Value::as_array)
        .ok_or(RpcFailure::Security)?;
    if roots.len() != 1
        || !roots[0]
            .as_str()
            .is_some_and(|root| paths_match(Path::new(root), workspace))
    {
        return Err(RpcFailure::Security);
    }
    result
        .get("thread")
        .and_then(|value| value.get("id"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or(RpcFailure::Protocol)
}

fn paths_match(actual: &Path, expected: &Path) -> bool {
    match (fs::canonicalize(actual), fs::canonicalize(expected)) {
        (Ok(actual), Ok(expected)) => actual == expected,
        _ => actual == expected,
    }
}

fn matching_turn_params<'a>(
    message: &'a Value,
    thread_id: &str,
    turn_id: Option<&str>,
) -> Result<&'a Value, RpcFailure> {
    let params = message.get("params").ok_or(RpcFailure::Protocol)?;
    if params.get("threadId").and_then(Value::as_str) != Some(thread_id) {
        return Err(RpcFailure::Security);
    }
    if let Some(expected) = turn_id {
        if params.get("turnId").and_then(Value::as_str) != Some(expected) {
            return Err(RpcFailure::Security);
        }
    }
    Ok(params)
}

fn matching_completed_turn<'a>(
    message: &'a Value,
    thread_id: &str,
    turn_id: &str,
) -> Result<&'a Value, RpcFailure> {
    let params = message.get("params").ok_or(RpcFailure::Protocol)?;
    if params.get("threadId").and_then(Value::as_str) != Some(thread_id) {
        return Err(RpcFailure::Security);
    }
    let turn = params.get("turn").ok_or(RpcFailure::Protocol)?;
    if turn.get("id").and_then(Value::as_str) != Some(turn_id) {
        return Err(RpcFailure::Security);
    }
    Ok(turn)
}

fn interrupt_turn(server: &mut AppServer, thread_id: &str, turn_id: Option<&str>) {
    if let Some(turn_id) = turn_id {
        let _ = server.write(rpc_request(
            99,
            "turn/interrupt",
            json!({"threadId":thread_id,"turnId":turn_id}),
        ));
    }
}

fn nonnegative_u64(value: Option<&Value>) -> u64 {
    value
        .and_then(Value::as_i64)
        .and_then(|number| u64::try_from(number).ok())
        .unwrap_or(0)
}

fn ensure_workspace_empty(workspace: &Path) -> Result<(), RpcFailure> {
    let mut entries = fs::read_dir(workspace).map_err(|_| RpcFailure::Io)?;
    if entries.next().is_some() {
        return Err(RpcFailure::Security);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoginProgress {
    Ignore,
    Completed,
    Failed,
}

fn parse_login_notification(value: &Value, login_id: &str) -> Result<LoginProgress, RpcFailure> {
    if value.get("method").and_then(Value::as_str) != Some("account/login/completed") {
        return Ok(LoginProgress::Ignore);
    }
    let params = value.get("params").ok_or(RpcFailure::Protocol)?;
    if params.get("loginId").and_then(Value::as_str) != Some(login_id) {
        return Ok(LoginProgress::Ignore);
    }
    Ok(
        if params.get("success").and_then(Value::as_bool) == Some(true) {
            LoginProgress::Completed
        } else {
            LoginProgress::Failed
        },
    )
}

fn parse_account_state(result: &Value) -> Result<AccountState, RpcFailure> {
    let Some(account) = result.get("account") else {
        return Err(RpcFailure::Protocol);
    };
    if account.is_null() {
        return Ok(AccountState::SignedOut);
    }
    let account_type = account
        .get("type")
        .and_then(Value::as_str)
        .ok_or(RpcFailure::Protocol)?;
    if account_type != "chatgpt" {
        return Ok(AccountState::SignInRequired);
    }
    let plan = account
        .get("planType")
        .and_then(Value::as_str)
        .and_then(safe_plan_label);
    Ok(AccountState::SignedIn { plan })
}

fn safe_plan_label(value: &str) -> Option<String> {
    (!value.is_empty()
        && value.len() <= 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')))
    .then(|| value.to_string())
}

fn recv_until(
    receiver: &Receiver<Result<Value, RpcFailure>>,
    deadline: Instant,
) -> Result<Value, RpcFailure> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .ok_or(RpcFailure::Timeout)?;
    match receiver.recv_timeout(remaining) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => Err(RpcFailure::Timeout),
        Err(RecvTimeoutError::Disconnected) => Err(RpcFailure::Io),
    }
}

#[must_use]
pub fn allowed_signin_url(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    matches!(host, "auth.openai.com" | "chatgpt.com")
}

fn valid_user_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn isolated_paths() -> Result<(PathBuf, PathBuf), RpcFailure> {
    let root = dirs::data_local_dir()
        .ok_or(RpcFailure::Io)?
        .join("suflyor")
        .join("codex-provider");
    Ok((root.join("home"), root.join("empty-workspace")))
}

fn spawn_app_server(
    executable: &Path,
    codex_home: &Path,
    workspace: &Path,
) -> std::io::Result<Child> {
    // Windows `creation_flags(CREATE_NO_WINDOW)` is applied below after the
    // environment allowlist is configured.
    let mut command = Command::new(executable);
    command
        .env_clear()
        .args(APP_SERVER_ARGS)
        .current_dir(workspace)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for name in [
        "SystemRoot",
        "WINDIR",
        "TEMP",
        "TMP",
        "LOCALAPPDATA",
        "APPDATA",
        "USERPROFILE",
    ] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command.env("CODEX_HOME", codex_home);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.spawn()
}

/// Map app-server stderr to bounded, non-secret diagnostics. Raw child text
/// is never forwarded because it may gain account or endpoint details in a
/// future Codex version.
fn classify_app_server_stderr(line: &str) -> Option<&'static str> {
    let lower = line.to_ascii_lowercase();
    if lower.contains("filesystem path") || lower.contains("permission profile") {
        Some("security-config-rejected")
    } else if lower.contains("unknown configuration") || lower.contains("unrecognized") {
        Some("config-version-mismatch")
    } else if lower.contains("access is denied") || lower.contains("permission denied") {
        Some("executable-access-denied")
    } else if lower.contains("credential") || lower.contains("keyring") {
        Some("credential-store-error")
    } else {
        None
    }
}

fn rpc_request(id: u64, method: &str, params: Value) -> Value {
    json!({"method":method,"id":id,"params":params})
}

fn codex_executable_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    // Prefer the real Codex Desktop bundle over WindowsApps execution aliases,
    // which can exist on PATH but reject CreateProcess with access denied.
    if let Some(local) = dirs::data_local_dir() {
        let bundled = local.join("OpenAI").join("Codex").join("bin");
        if let Ok(entries) = fs::read_dir(bundled) {
            candidates.extend(
                entries
                    .flatten()
                    .map(|entry| entry.path().join("codex.exe")),
            );
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        candidates.extend(std::env::split_paths(&path).map(|dir| dir.join("codex.exe")));
    }
    let mut seen = std::collections::HashSet::new();
    candidates.retain(|path| path.is_file() && seen.insert(path.clone()));
    candidates
}

#[cfg(test)]
fn find_existing_executable<'a>(paths: impl IntoIterator<Item = &'a Path>) -> Option<PathBuf> {
    paths
        .into_iter()
        .find(|path| path.is_file())
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parses_non_secret_account_states() {
        assert_eq!(
            parse_account_state(&json!({"account":null})).unwrap(),
            AccountState::SignedOut
        );
        assert_eq!(
            parse_account_state(&json!({"account":{"type":"chatgpt","email":"private@example.test","planType":"plus"}})).unwrap(),
            AccountState::SignedIn { plan: Some("plus".into()) }
        );
        assert_eq!(
            parse_account_state(&json!({"account":{"type":"apiKey"}})).unwrap(),
            AccountState::SignInRequired
        );
    }

    #[test]
    fn login_state_machine_ignores_other_ids_and_never_surfaces_error_text() {
        let secret = "sk-sensitive-refresh-token";
        assert_eq!(
            parse_login_notification(
                &json!({"method":"account/login/completed","params":{"loginId":"other","success":false,"error":secret}}),
                "ours"
            )
            .unwrap(),
            LoginProgress::Ignore
        );
        assert_eq!(
            parse_login_notification(
                &json!({"method":"account/login/completed","params":{"loginId":"ours","success":false,"error":secret}}),
                "ours"
            )
            .unwrap(),
            LoginProgress::Failed
        );
        assert!(!format!("{:?}", LoginProgress::Failed).contains(secret));
    }

    #[test]
    fn malformed_and_reauth_responses_fail_closed() {
        assert_eq!(parse_account_state(&json!({})), Err(RpcFailure::Protocol));
        assert_eq!(
            parse_account_state(&json!({"account":{"type":"chatgpt","planType":"<token>"}}))
                .unwrap(),
            AccountState::SignedIn { plan: None }
        );
        assert_eq!(
            parse_login_notification(
                &json!({"method":"account/login/completed","params":{}}),
                "ours"
            ),
            Ok(LoginProgress::Ignore)
        );
    }

    #[test]
    fn timeout_is_deterministic() {
        let (_sender, receiver) = mpsc::channel();
        assert_eq!(
            recv_until(&receiver, Instant::now() + Duration::from_millis(1)),
            Err(RpcFailure::Timeout)
        );
    }

    #[test]
    fn receiver_drop_on_terminal_done_is_still_a_cancellation() {
        let mut dropped = |_event| false;
        assert_eq!(
            emit_turn_event(&mut dropped, TurnEvent::Done),
            Err(RpcFailure::Cancelled)
        );
    }

    #[test]
    fn executable_detection_reports_absent_and_present() {
        let temp = TempDir::new().unwrap();
        let absent = temp.path().join("missing.exe");
        assert!(find_existing_executable([absent.as_path()]).is_none());
        let present = temp.path().join("codex.exe");
        fs::write(&present, b"mock").unwrap();
        assert_eq!(
            find_existing_executable([absent.as_path(), present.as_path()]),
            Some(present)
        );
    }

    #[test]
    fn app_server_stderr_is_classified_without_forwarding_raw_details() {
        assert_eq!(
            classify_app_server_stderr("filesystem path x must be absolute"),
            Some("security-config-rejected")
        );
        assert_eq!(
            classify_app_server_stderr("unknown configuration key private.value"),
            Some("config-version-mismatch")
        );
        assert_eq!(classify_app_server_stderr("token=secret"), None);
    }

    #[test]
    fn only_official_https_login_hosts_are_accepted() {
        assert!(allowed_signin_url("https://auth.openai.com/codex/device"));
        assert!(allowed_signin_url("https://chatgpt.com/auth"));
        assert!(!allowed_signin_url("http://auth.openai.com/codex/device"));
        assert!(!allowed_signin_url(
            "https://auth.openai.com.evil.test/device"
        ));
    }

    #[test]
    fn user_codes_are_bounded_ascii() {
        assert!(valid_user_code("ABCD-1234"));
        assert!(!valid_user_code(""));
        assert!(!valid_user_code("ABCD 1234"));
        assert!(!valid_user_code(&"A".repeat(33)));
    }

    #[test]
    fn stable_json_rpc_requests_match_official_wire_shape() {
        assert_eq!(
            rpc_request(
                3,
                "account/login/start",
                json!({"type":"chatgptDeviceCode"})
            ),
            json!({
                "method":"account/login/start",
                "id":3,
                "params":{"type":"chatgptDeviceCode"}
            })
        );
        let account = rpc_request(2, "account/read", json!({"refreshToken":false}));
        assert!(account.get("jsonrpc").is_none());
        assert_eq!(account["params"]["refreshToken"], false);
    }

    #[test]
    fn experimental_security_contract_is_explicit_and_model_pinned_twice() {
        assert!(APP_SERVER_ARGS.contains(&"--strict-config"));
        assert!(APP_SERVER_ARGS.contains(&"windows.sandbox=\"elevated\""));
        assert!(APP_SERVER_ARGS.contains(&"default_permissions=\"suflyor-text-only\""));
        assert!(
            APP_SERVER_ARGS.contains(&"permissions.suflyor-text-only.filesystem.:root=\"deny\"")
        );
        assert!(APP_SERVER_ARGS.contains(&"permissions.suflyor-text-only.network.enabled=false"));
        let initialize = initialize_params(true);
        assert_eq!(initialize["capabilities"]["experimentalApi"], true);
        assert_eq!(
            initialize["capabilities"]["mcpServerOpenaiFormElicitation"],
            false
        );
        assert_eq!(initialize["capabilities"]["requestAttestation"], false);
        assert_eq!(
            initialize["capabilities"]["optOutNotificationMethods"],
            json!(["thread/started", "turn/started"])
        );

        let thread = secure_thread_params("gpt-5.4", r"C:\safe\empty");
        assert_eq!(thread["model"], "gpt-5.4");
        assert_eq!(thread["ephemeral"], true);
        assert_eq!(thread["approvalPolicy"], "never");
        assert_eq!(thread["permissions"], SECURE_PROFILE);
        assert_eq!(thread["runtimeWorkspaceRoots"], json!([r"C:\safe\empty"]));
        assert_eq!(thread["selectedCapabilityRoots"], json!([]));
        assert_eq!(thread["environments"], json!([]));
        assert_eq!(thread["dynamicTools"], json!([]));
        assert_eq!(thread["allowProviderModelFallback"], false);
        assert_eq!(thread["experimentalRawEvents"], false);
        assert_eq!(thread["config"]["web_search"], "disabled");
        assert_eq!(thread["config"]["mcp_servers"], json!({}));
        assert_eq!(
            thread["config"]["shell_environment_policy"]["inherit"],
            "none"
        );

        let turn = secure_turn_params("thread-1", "gpt-5.4", r"C:\safe\empty", "hello");
        assert_eq!(turn["model"], "gpt-5.4");
        assert_eq!(turn["approvalPolicy"], "never");
        assert_eq!(turn["permissions"], SECURE_PROFILE);
        assert_eq!(turn["environments"], json!([]));
        assert_eq!(turn["input"].as_array().unwrap().len(), 1);
        assert_eq!(turn["input"][0]["type"], "text");
        let wire = format!("{thread}{turn}");
        assert!(!wire.contains("shellCommand"));
        assert!(!wire.contains("dynamicToolCall"));
        assert!(!wire.contains("process/"));
    }

    #[test]
    fn model_catalog_preserves_account_metadata_and_rejects_unsafe_ids() {
        let model = parse_model(&json!({
            "model":"gpt-5.4-codex",
            "displayName":"GPT-5.4 Codex",
            "isDefault":true,
            "defaultReasoningEffort":"high",
            "supportedReasoningEfforts":[
                {"reasoningEffort":"medium","description":"Balanced"},
                {"reasoningEffort":"high","description":"Deeper reasoning"}
            ],
            "inputModalities":["text","image"]
        }))
        .unwrap();
        assert_eq!(model.id, "gpt-5.4-codex");
        assert!(model.is_default);
        assert!(model
            .picker_label()
            .contains("reasoning: medium/high; default high"));
        assert!(model.picker_label().contains("text+image"));
        assert_eq!(
            parse_model(&json!({"model":"gpt 5 <token>","displayName":"bad"})),
            Err(RpcFailure::Protocol)
        );
        assert_eq!(
            parse_model(&json!({
                "model":"gpt-safe",
                "supportedReasoningEfforts":["high"]
            })),
            Err(RpcFailure::Protocol)
        );
    }

    #[test]
    fn paginated_catalog_cursor_and_duplicate_exact_model_fail_closed() {
        let first = json!({"data":[{"model":"gpt-a","displayName":"A"}],"nextCursor":"next"});
        let second = json!({"data":[{"model":"gpt-b","displayName":"B"}],"nextCursor":null});
        let mut models = Vec::new();
        assert_eq!(
            append_model_page(&mut models, &first).unwrap(),
            Some("next".into())
        );
        assert_eq!(append_model_page(&mut models, &second).unwrap(), None);
        assert_eq!(
            models
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            ["gpt-a", "gpt-b"]
        );
        let duplicate = json!({"data":[{"model":"gpt-a","displayName":"Again"}],"nextCursor":null});
        assert_eq!(
            append_model_page(&mut models, &duplicate),
            Err(RpcFailure::Protocol)
        );
        let oversized_page = json!({
            "data": (0..=MAX_MODELS_PER_PAGE)
                .map(|index| json!({"model":format!("gpt-{index}")}))
                .collect::<Vec<_>>(),
            "nextCursor": null
        });
        assert_eq!(
            append_model_page(&mut Vec::new(), &oversized_page),
            Err(RpcFailure::Protocol)
        );
        let oversized_cursor = json!({
            "data": [],
            "nextCursor": "x".repeat(MAX_CURSOR_BYTES + 1)
        });
        assert_eq!(
            append_model_page(&mut Vec::new(), &oversized_cursor),
            Err(RpcFailure::Protocol)
        );
    }

    #[test]
    fn thread_contract_requires_exact_model_profile_workspace_and_no_network() {
        let workspace = Path::new(r"C:\safe\empty");
        let response = json!({
            "model":"gpt-5.4",
            "approvalPolicy":"never",
            "activePermissionProfile":{"id":SECURE_PROFILE},
            "sandbox":{"type":"readOnly","networkAccess":false},
            "cwd":r"C:\safe\empty",
            "runtimeWorkspaceRoots":[r"C:\safe\empty"],
            "instructionSources":[],
            "thread":{"id":"thread-1"}
        });
        assert_eq!(
            validate_thread_contract(&response, "gpt-5.4", workspace).unwrap(),
            "thread-1"
        );
        let mut rerouted = response.clone();
        rerouted["model"] = json!("gpt-other");
        assert_eq!(
            validate_thread_contract(&rerouted, "gpt-5.4", workspace),
            Err(RpcFailure::ModelMismatch)
        );
        let mut networked = response.clone();
        networked["sandbox"]["networkAccess"] = json!(true);
        assert_eq!(
            validate_thread_contract(&networked, "gpt-5.4", workspace),
            Err(RpcFailure::Security)
        );
        let mut missing_network = response.clone();
        missing_network["sandbox"]
            .as_object_mut()
            .unwrap()
            .remove("networkAccess");
        assert_eq!(
            validate_thread_contract(&missing_network, "gpt-5.4", workspace),
            Err(RpcFailure::Security)
        );
        let mut missing_sources = response;
        missing_sources
            .as_object_mut()
            .unwrap()
            .remove("instructionSources");
        assert_eq!(
            validate_thread_contract(&missing_sources, "gpt-5.4", workspace),
            Err(RpcFailure::Security)
        );
    }

    #[test]
    fn streaming_ids_are_exact_and_tool_items_are_denied() {
        let delta = json!({"method":"item/agentMessage/delta","params":{
            "threadId":"thread-1","turnId":"turn-1","delta":"hello"
        }});
        assert_eq!(
            parse_turn_notification(&delta, "thread-1", "turn-1"),
            Ok(TurnNotification::Delta("hello".into()))
        );
        assert_eq!(
            parse_turn_notification(&delta, "other", "turn-1"),
            Err(RpcFailure::Security)
        );
        let usage = json!({"method":"thread/tokenUsage/updated","params":{
            "threadId":"thread-1","turnId":"turn-1",
            "tokenUsage":{"last":{"inputTokens":17,"outputTokens":9}}
        }});
        assert_eq!(
            parse_turn_notification(&usage, "thread-1", "turn-1"),
            Ok(TurnNotification::Usage {
                input: 17,
                output: 9
            })
        );
        let completed = json!({"method":"turn/completed","params":{
            "threadId":"thread-1",
            "turn":{"id":"turn-1","status":"completed","items":[]}
        }});
        assert_eq!(
            parse_turn_notification(&completed, "thread-1", "turn-1"),
            Ok(TurnNotification::Completed)
        );
        assert_eq!(
            parse_turn_notification(&completed, "thread-1", "other"),
            Err(RpcFailure::Security)
        );
        let unsafe_completed = json!({"method":"turn/completed","params":{
            "threadId":"thread-1",
            "turn":{"id":"turn-1","status":"completed","items":[{"type":"commandExecution"}]}
        }});
        assert_eq!(
            parse_turn_notification(&unsafe_completed, "thread-1", "turn-1"),
            Err(RpcFailure::Security)
        );
        let rate_limits = json!({"method":"account/rateLimits/updated","params":{
            "rateLimits":{"primary":{"usedPercent":13}}
        }});
        assert_eq!(
            parse_turn_notification(&rate_limits, "thread-1", "turn-1"),
            Ok(TurnNotification::RateLimitsUpdated)
        );
        assert_eq!(
            parse_turn_notification(
                &json!({"method":"account/rateLimits/updated","params":{}}),
                "thread-1",
                "turn-1"
            ),
            Err(RpcFailure::Protocol)
        );
        let lifecycle = json!({"method":"item/started","params":{
            "threadId":"thread-1","turnId":"turn-1","item":{"type":"agentMessage"}
        }});
        assert_eq!(
            parse_turn_notification(&lifecycle, "thread-1", "turn-1"),
            Ok(TurnNotification::SafeItemLifecycle)
        );
        let reasoning = json!({"method":"item/reasoning/summaryTextDelta","params":{
            "threadId":"thread-1","turnId":"turn-1","delta":"private reasoning"
        }});
        assert_eq!(
            parse_turn_notification(&reasoning, "thread-1", "turn-1"),
            Ok(TurnNotification::SafeReasoningUpdate)
        );
        for denied in [
            "commandExecution",
            "fileChange",
            "mcpToolCall",
            "dynamicToolCall",
            "webSearch",
            "imageGeneration",
        ] {
            let item = json!({"method":"item/started","params":{
                "threadId":"thread-1","turnId":"turn-1","item":{"type":denied}
            }});
            assert_eq!(
                parse_turn_notification(&item, "thread-1", "turn-1"),
                Err(RpcFailure::Security)
            );
        }
        assert_eq!(
            parse_turn_notification(
                &json!({"method":"permissions/requestApproval","params":{}}),
                "thread-1",
                "turn-1"
            ),
            Err(RpcFailure::Security)
        );
        assert_eq!(
            parse_turn_notification(
                &json!({"method":"model/rerouted","params":{}}),
                "thread-1",
                "turn-1"
            ),
            Err(RpcFailure::ModelMismatch)
        );
    }

    #[test]
    fn image_input_and_nonempty_workspace_fail_closed() {
        let messages = vec![ChatMessage {
            role: "user".into(),
            content: MessageContent::Parts(vec![ContentPart::ImageUrl {
                image_url: crate::ai::ImageUrl {
                    url: "data:image/png;base64,secret".into(),
                },
            }]),
        }];
        assert_eq!(text_only_prompt(&messages), Err(RpcFailure::Security));
        let temp = TempDir::new().unwrap();
        assert!(ensure_workspace_empty(temp.path()).is_ok());
        fs::write(temp.path().join("unexpected.txt"), b"x").unwrap();
        assert_eq!(
            ensure_workspace_empty(temp.path()),
            Err(RpcFailure::Security)
        );
    }

    #[test]
    fn rate_limit_and_failures_never_echo_server_secrets() {
        assert_eq!(
            parse_rate_limits(
                &json!({"rateLimits":{"primary":{"usedPercent":42,"windowDurationMins":300},"token":"sk-secret"}})
            ),
            Some("42% used / 300 min".into())
        );
        let rendered = format!("{:?}", TurnFailure::SecurityViolation);
        assert!(!rendered.contains("sk-secret"));
        assert!(!rendered.contains("https://"));
    }
}
