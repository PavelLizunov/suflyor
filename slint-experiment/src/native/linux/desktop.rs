//! Native Hyprland transport and per-config command socket.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixDatagram;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

type Error = Box<dyn std::error::Error>;

pub fn hypr(args: &[&str]) -> Result<String, Error> {
    let output = Command::new("timeout").args(["--signal=KILL", "2s", "hyprctl"])
        .args(args).stderr(Stdio::null()).output()?;
    if !output.status.success() { return Err("Hyprland request failed".into()); }
    Ok(String::from_utf8(output.stdout)?)
}
pub fn query(name: &str) -> Result<serde_json::Value, Error> {
    Ok(serde_json::from_str(&hypr(&["-j", name])?)?)
}
pub fn dispatch(action: &str, value: &str) -> Result<(), Error> {
    let answer = hypr(&["dispatch", action, value])?;
    if answer.trim() != "ok" { return Err("Hyprland action was rejected".into()); }
    Ok(())
}

fn socket_path() -> Result<PathBuf, Error> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").ok_or("Wayland runtime directory unavailable")?;
    let mut hash = DefaultHasher::new();
    overlay_backend::paths::data_root().hash(&mut hash);
    Ok(PathBuf::from(runtime).join(format!("suflyor-{:016x}.sock", hash.finish())))
}

pub fn send_command(value: &str) -> Result<(), Error> {
    if value.len() > 80 || !value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return Err("invalid Suflyor command".into());
    }
    UnixDatagram::unbound()?.send_to(value.as_bytes(), socket_path()?)?;
    Ok(())
}

struct CommandSocket { socket: UnixDatagram, path: PathBuf }
impl Drop for CommandSocket {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.path); }
}
static SOCKET: OnceLock<Mutex<Option<CommandSocket>>> = OnceLock::new();

pub fn listen() -> Result<(), Error> {
    let path = socket_path()?;
    if path.exists() {
        if UnixDatagram::unbound()?.connect(&path).is_ok() { return Err("Suflyor command socket is busy".into()); }
        std::fs::remove_file(&path)?;
    }
    let socket = UnixDatagram::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    socket.set_nonblocking(true)?;
    let mut slot = SOCKET.get_or_init(|| Mutex::new(None)).lock().map_err(|_| "command socket lock failed")?;
    *slot = Some(CommandSocket { socket, path });
    Ok(())
}

pub fn receive() -> Option<String> {
    let slot = SOCKET.get()?.lock().ok()?;
    let mut buffer = [0u8; 81];
    let n = slot.as_ref()?.socket.recv(&mut buffer).ok()?;
    if n > 80 { return None; }
    std::str::from_utf8(&buffer[..n]).ok().map(str::to_owned)
}

pub fn install_bindings(bindings: &[(&str, &str)]) -> Result<Vec<String>, Error> {
    let binary = std::env::current_exe()?;
    let command = format!("'{}' --command ", binary.to_string_lossy().replace('\'', "'\\''"));
    let existing = query("binds")?;
    let existing = existing.as_array().ok_or("invalid compositor bindings")?;
    let mut installed = Vec::new();
    for (keys, action) in bindings {
        let (mods, key) = binding_parts(keys);
        let conflict = existing.iter().any(|bind| {
            bind["key"].as_str().is_some_and(|k| k.eq_ignore_ascii_case(key))
                && bind["modmask"].as_u64() == Some(mods)
                && !bind["arg"].as_str().is_some_and(|s| s == format!("{command}{action}"))
        });
        if conflict { continue; }
        let lua = format!("hl.bind({}, hl.dsp.exec_cmd({}))", serde_json::to_string(keys)?, serde_json::to_string(&format!("{command}{action}"))?);
        let result = hypr(&["eval", &lua])?;
        if result.to_ascii_lowercase().contains("error") { continue; }
        installed.push(action.to_string());
    }
    Ok(installed)
}

fn binding_parts(keys: &str) -> (u64, &str) {
    let mut mods = 0;
    let mut key = "";
    for part in keys.split(" + ") {
        match part { "SHIFT" => mods |= 1, "CTRL" => mods |= 4, "ALT" => mods |= 8, "SUPER" => mods |= 64, _ => key = part }
    }
    (mods, key)
}

pub fn open(target: &std::ffi::OsStr) -> io::Result<()> {
    let mut child = Command::new("xdg-open").arg(target).stdin(Stdio::null())
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
    std::thread::spawn(move || { let _ = child.wait(); });
    Ok(())
}
