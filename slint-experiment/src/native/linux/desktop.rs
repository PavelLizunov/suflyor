//! Native Hyprland transport and per-config command socket.

use sha2::{Digest, Sha256};
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
pub fn property(window:&str,name:&str)->Result<serde_json::Value,Error>{
    let value:serde_json::Value=serde_json::from_str(&hypr(&["-j","getprop",window,name])?)?;
    value.get(name).cloned().ok_or_else(||"compositor property unavailable".into())
}
pub fn set_property(window:&str,name:&str,value:&str)->Result<(),Error>{
    let lua=format!("hl.dsp.window.set_prop({{window={},prop={},value={}}})",serde_json::to_string(window)?,serde_json::to_string(name)?,serde_json::to_string(value)?);
    if hypr(&["dispatch",&lua])?.trim()!="ok"{return Err("compositor property rejected".into());}
    Ok(())
}
pub fn query(name: &str) -> Result<serde_json::Value, Error> {
    Ok(serde_json::from_str(&hypr(&["-j", name])?)?)
}
pub fn dispatch(action: &str, value: &str) -> Result<(), Error> {
    let lua = match action {
        "movewindowpixel" | "resizewindowpixel" => {
            let (coords,window)=value.rsplit_once(',').ok_or("missing window selector")?;
            let mut nums=coords.trim_start_matches("exact ").split_whitespace();
            let x=nums.next().ok_or("missing x")?.parse::<i32>()?;
            let y=nums.next().ok_or("missing y")?.parse::<i32>()?;
            let method=if action=="movewindowpixel"{"move"}else{"resize"};
            format!("hl.dsp.window.{method}({{window={},x={x},y={y}}})",serde_json::to_string(window)?)
        }
        "setfloating"=>format!("hl.dsp.window.float({{window={},action=\"on\"}})",serde_json::to_string(value)?),
        "focuswindow"=>format!("hl.dsp.focus({{window={}}})",serde_json::to_string(value)?),
        "pin"=>format!("hl.dsp.window.pin({{window={}}})",serde_json::to_string(value)?),
        _=>return Err("unsupported native compositor operation".into()),
    };
    let answer = hypr(&["dispatch", &lua])?;
    if answer.trim() != "ok" { return Err("Hyprland action was rejected".into()); }
    Ok(())
}

fn socket_path() -> Result<PathBuf, Error> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").ok_or("Wayland runtime directory unavailable")?;
    let config = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
        .filter(|p| p.is_absolute()).or_else(|| dirs::home_dir().map(|p|p.join(".config")))
        .ok_or("configuration directory unavailable")?;
    let hash = Sha256::digest(config.as_os_str().as_encoded_bytes());
    let suffix: String = hash[..8].iter().map(|b|format!("{b:02x}")).collect();
    Ok(PathBuf::from(runtime).join(format!("suflyor-{suffix}.sock")))
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

pub fn close() {
    if let Some(slot)=SOCKET.get(){let _=slot.lock().map(|mut slot|slot.take());}
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
    let quoted = |s: &str| format!("'{}'",s.replace('\'', "'\\''"));
    let config = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_|dirs::home_dir().unwrap_or_default().join(".config").to_string_lossy().into_owned());
    let command = format!("env XDG_CONFIG_HOME={} {} --command ",quoted(&config),quoted(&binary.to_string_lossy()));
    let existing = query("binds")?;
    let existing = existing.as_array().ok_or("invalid compositor bindings")?;
    let mut installed = Vec::new();
    for (keys, action) in bindings {
        let (mods, key) = binding_parts(keys);
        let conflict = existing.iter().any(|bind| {
            bind["key"].as_str().is_some_and(|k| k.eq_ignore_ascii_case(key))
                && bind["modmask"].as_u64() == Some(mods)
                && !bind["arg"].as_str().is_some_and(|s| s.contains(" --command ") && s.contains("suflyor"))
        });
        if conflict { continue; }
        let release = action.ends_with("-up");
        let keys = keys.strip_suffix("@release").unwrap_or(keys);
        if !release { let _=hypr(&["eval",&format!("hl.unbind({})",serde_json::to_string(keys)?)]); }
        let lua = format!("hl.bind({}, hl.dsp.exec_cmd({}), {{ release = {} }})", serde_json::to_string(keys)?, serde_json::to_string(&format!("{command}{action}"))?,release);
        let result = hypr(&["eval", &lua])?;
        if result.to_ascii_lowercase().contains("error") { continue; }
        installed.push(action.to_string());
    }
    Ok(installed)
}

fn binding_parts(keys: &str) -> (u64, &str) {
    let mut mods = 0;
    let mut key = "";
    for part in keys.trim_end_matches("@release").split(" + ") {
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
