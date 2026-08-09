//! Experimental ChatGPT-subscription sign-in through the official Codex
//! `app-server` stdio protocol.
//!
//! Suflyor never reads or writes Codex tokens. The official child process owns
//! the device-code flow and stores credentials in Windows Credential Manager
//! (`cli_auth_credentials_store = "keyring"`) under an isolated `CODEX_HOME`.
//! Live answers are intentionally not routed through app-server yet: the stable
//! protocol exposes an agent with filesystem/shell tools and does not currently
//! provide a documented, enforceable "no tools and no filesystem reads" mode.

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
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

static LOGIN_GENERATION: AtomicU64 = AtomicU64::new(0);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RpcFailure {
    NotInstalled,
    Io,
    Timeout,
    Protocol,
    Cancelled,
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
        Ok(Self {
            child,
            stdin,
            messages,
        })
    }

    fn initialize(&mut self) -> Result<(), RpcFailure> {
        self.request(
            1,
            "initialize",
            json!({
                "clientInfo": {
                    "name": "suflyor",
                    "title": "suflyor",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }),
        )?;
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

/// Run the official device-code flow. The caller decides how to present and
/// open the verification URL; this function never launches a browser.
pub fn device_login(mut notify: impl FnMut(LoginEvent)) {
    let generation = LOGIN_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let result = device_login_inner(generation, &mut notify);
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
    let mut server = AppServer::start()?;
    server.initialize()?;

    let current = server.request(2, "account/read", json!({"refreshToken":false}))?;
    if let AccountState::SignedIn { plan } = parse_account_state(&current)? {
        notify(LoginEvent::SignedIn { plan });
        return Ok(());
    }

    let started = server.request(
        3,
        "account/login/start",
        json!({"type":"chatgptDeviceCode"}),
    )?;
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
    notify(LoginEvent::AwaitingUser {
        verification_url,
        user_code,
    });

    let deadline = Instant::now() + LOGIN_TIMEOUT;
    loop {
        if LOGIN_GENERATION.load(Ordering::SeqCst) != generation {
            return Err(RpcFailure::Cancelled);
        }
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

/// Cancel a pending login and ask the official app-server to clear its stored
/// subscription credential. No credential bytes enter Suflyor memory.
#[must_use]
pub fn disconnect() -> AccountState {
    LOGIN_GENERATION.fetch_add(1, Ordering::SeqCst);
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
    let mut command = Command::new(executable);
    command
        .args([
            "app-server",
            "--stdio",
            "-c",
            "cli_auth_credentials_store=\"keyring\"",
        ])
        .env("CODEX_HOME", codex_home)
        .current_dir(workspace)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.spawn()
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
}
