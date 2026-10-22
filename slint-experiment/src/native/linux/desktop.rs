//! Native Hyprland transport and per-config command socket.

use sha2::{Digest, Sha256};
use std::io::{self, Read};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixDatagram, UnixStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

type Error = Box<dyn std::error::Error>;

pub fn hypr(args: &[&str]) -> Result<String, Error> {
    let output = Command::new("timeout")
        .args(["--signal=KILL", "2s", "hyprctl"])
        .args(args)
        .stderr(Stdio::null())
        .output()?;
    if !output.status.success() {
        return Err("Hyprland request failed".into());
    }
    Ok(String::from_utf8(output.stdout)?)
}
pub fn property(window: &str, name: &str) -> Result<serde_json::Value, Error> {
    let value: serde_json::Value = serde_json::from_str(&hypr(&["-j", "getprop", window, name])?)?;
    value
        .get(name)
        .cloned()
        .ok_or_else(|| "compositor property unavailable".into())
}
pub fn set_property(window: &str, name: &str, value: &str) -> Result<(), Error> {
    let lua = format!(
        "hl.dsp.window.set_prop({{window={},prop={},value={}}})",
        serde_json::to_string(window)?,
        serde_json::to_string(name)?,
        serde_json::to_string(value)?
    );
    if hypr(&["dispatch", &lua])?.trim() != "ok" {
        return Err("compositor property rejected".into());
    }
    Ok(())
}
pub fn query(name: &str) -> Result<serde_json::Value, Error> {
    Ok(serde_json::from_str(&hypr(&["-j", name])?)?)
}
pub fn dispatch(action: &str, value: &str) -> Result<(), Error> {
    let lua = match action {
        "movewindowpixel" | "resizewindowpixel" => {
            let (coords, window) = value.rsplit_once(',').ok_or("missing window selector")?;
            let mut nums = coords.trim_start_matches("exact ").split_whitespace();
            let x = nums.next().ok_or("missing x")?.parse::<i32>()?;
            let y = nums.next().ok_or("missing y")?.parse::<i32>()?;
            let method = if action == "movewindowpixel" {
                "move"
            } else {
                "resize"
            };
            format!(
                "hl.dsp.window.{method}({{window={},x={x},y={y}}})",
                serde_json::to_string(window)?
            )
        }
        "setfloating" => format!(
            "hl.dsp.window.float({{window={},action=\"on\"}})",
            serde_json::to_string(value)?
        ),
        "focuswindow" => format!("hl.dsp.focus({{window={}}})", serde_json::to_string(value)?),
        "pin" => format!(
            "hl.dsp.window.pin({{window={}}})",
            serde_json::to_string(value)?
        ),
        _ => return Err("unsupported native compositor operation".into()),
    };
    let answer = hypr(&["dispatch", &lua])?;
    if answer.trim() != "ok" {
        return Err("Hyprland action was rejected".into());
    }
    Ok(())
}

fn socket_path() -> Result<PathBuf, Error> {
    let runtime =
        std::env::var_os("XDG_RUNTIME_DIR").ok_or("Wayland runtime directory unavailable")?;
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| dirs::home_dir().map(|p| p.join(".config")))
        .ok_or("configuration directory unavailable")?;
    let hash = Sha256::digest(config.as_os_str().as_encoded_bytes());
    let suffix: String = hash[..8].iter().map(|b| format!("{b:02x}")).collect();
    Ok(PathBuf::from(runtime).join(format!("suflyor-{suffix}.sock")))
}

pub fn send_command(value: &str) -> Result<(), Error> {
    if value.len() > 80
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err("invalid Suflyor command".into());
    }
    UnixDatagram::unbound()?.send_to(value.as_bytes(), socket_path()?)?;
    Ok(())
}

struct CommandSocket {
    socket: UnixDatagram,
    path: PathBuf,
    events: UnixStream,
    pending: Vec<u8>,
}

fn take_config_reload(pending: &mut Vec<u8>) -> bool {
    let mut reloaded = false;
    while let Some(end) = pending.iter().position(|&b| b == b'\n') {
        reloaded |= &pending[..end] == b"configreloaded>>";
        pending.drain(..=end);
    }
    // An incomplete unrelated event must not grow the input buffer indefinitely.
    if pending.len() > 8192 {
        pending.clear();
    }
    reloaded
}
impl Drop for CommandSocket {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
static SOCKET: OnceLock<Mutex<Option<CommandSocket>>> = OnceLock::new();

pub fn listen() -> Result<(), Error> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").ok_or("Wayland runtime unavailable")?;
    let signature =
        std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").ok_or("Hyprland instance unavailable")?;
    let events = UnixStream::connect(
        PathBuf::from(runtime)
            .join("hypr")
            .join(signature)
            .join(".socket2.sock"),
    )?;
    events.set_nonblocking(true)?;
    let path = socket_path()?;
    if path.exists() {
        if UnixDatagram::unbound()?.connect(&path).is_ok() {
            return Err("Suflyor command socket is busy".into());
        }
        std::fs::remove_file(&path)?;
    }
    let socket = UnixDatagram::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    socket.set_nonblocking(true)?;
    let mut slot = SOCKET
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "command socket lock failed")?;
    *slot = Some(CommandSocket {
        socket,
        path,
        events,
        pending: Vec::new(),
    });
    Ok(())
}

pub fn close() {
    if let Ok(owner) = binding_owner() {
        if let Ok(existing) = query("binds") {
            if let Some(bindings) = existing.as_array() {
                for bind in bindings
                    .iter()
                    .filter(|b| b["description"].as_str() == Some(&owner))
                {
                    if let Some(key) = bind["key"].as_str() {
                        let mask = bind["modmask"].as_u64().unwrap_or(0);
                        let mut parts = Vec::new();
                        for (bit, name) in [(64, "SUPER"), (1, "SHIFT"), (4, "CTRL"), (8, "ALT")] {
                            if mask & bit != 0 {
                                parts.push(name);
                            }
                        }
                        parts.push(key);
                        if let Ok(key) = serde_json::to_string(&parts.join(" + ")) {
                            let _ = hypr(&["eval", &format!("hl.unbind({key})")]);
                        }
                    }
                }
            }
        }
    }
    if let Some(slot) = SOCKET.get() {
        let _ = slot.lock().map(|mut slot| slot.take());
    }
}

pub fn receive() -> Option<String> {
    let mut slot = SOCKET.get()?.lock().ok()?;
    let owner = slot.as_mut()?;
    let mut command = [0u8; 81];
    if let Ok(n) = owner.socket.recv(&mut command) {
        return (n <= 80)
            .then(|| std::str::from_utf8(&command[..n]).ok().map(str::to_owned))
            .flatten();
    }
    let mut events = [0u8; 4096];
    if let Ok(n) = owner.events.read(&mut events) {
        owner.pending.extend_from_slice(&events[..n]);
    }
    take_config_reload(&mut owner.pending).then(|| "config-reloaded".to_string())
}

fn binding_owner() -> Result<String, Error> {
    let path = socket_path()?;
    Ok(path
        .file_stem()
        .ok_or("command identity unavailable")?
        .to_string_lossy()
        .into_owned())
}

pub fn install_bindings(bindings: &[(&str, &str)]) -> Result<Vec<String>, Error> {
    let owner = binding_owner()?;
    let binary = std::env::current_exe()?;
    let quoted = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
    let config = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".config")
            .to_string_lossy()
            .into_owned()
    });
    let command = format!(
        "env XDG_CONFIG_HOME={} {} --command ",
        quoted(&config),
        quoted(&binary.to_string_lossy())
    );
    let existing = query("binds")?;
    let existing = existing.as_array().ok_or("invalid compositor bindings")?;
    let mut installed = Vec::new();
    let mut script = String::new();
    for (keys, action) in bindings {
        let (mods, key) = binding_parts(keys);
        let conflict = existing.iter().any(|bind| {
            bind["key"]
                .as_str()
                .is_some_and(|k| k.eq_ignore_ascii_case(key))
                && bind["modmask"].as_u64() == Some(mods)
                && bind["description"].as_str() != Some(&owner)
        });
        if conflict {
            continue;
        }
        let release = action.ends_with("-up");
        let keys = keys.strip_suffix("@release").unwrap_or(keys);
        if !release {
            script.push_str(&format!("hl.unbind({});", serde_json::to_string(keys)?));
        }
        let lua = format!(
            "hl.bind({}, hl.dsp.exec_cmd({}), {{ description = {}, release = {} }})",
            serde_json::to_string(keys)?,
            serde_json::to_string(&format!("{command}{action}"))?,
            serde_json::to_string(&owner)?,
            release
        );
        script.push_str(&lua);
        script.push(';');
        installed.push(action.to_string());
    }
    if !script.is_empty()
        && hypr(&["eval", &script])?
            .to_ascii_lowercase()
            .contains("error")
    {
        return Err("compositor bindings were rejected".into());
    }
    Ok(installed)
}

fn binding_parts(keys: &str) -> (u64, &str) {
    let mut mods = 0;
    let mut key = "";
    for part in keys.trim_end_matches("@release").split(" + ") {
        match part {
            "SHIFT" => mods |= 1,
            "CTRL" => mods |= 4,
            "ALT" => mods |= 8,
            "SUPER" => mods |= 64,
            _ => key = part,
        }
    }
    (mods, key)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::take_config_reload;

    #[test]
    fn reload_events_preserve_partial_frames_and_ignore_other_events() {
        let mut pending = b"windowtitle>>configreloaded\nconfigrelo".to_vec();
        assert!(!take_config_reload(&mut pending));
        assert_eq!(pending, b"configrelo");
        pending.extend_from_slice(b"aded>>\nworkspace>>2\nconfigreloaded>>\n");
        assert!(take_config_reload(&mut pending));
        assert!(pending.is_empty());
        assert!(!take_config_reload(&mut pending));
        pending.resize(8193, b'x');
        assert!(!take_config_reload(&mut pending));
        assert!(pending.is_empty());
    }
}

pub fn open(target: &std::ffi::OsStr) -> io::Result<()> {
    let mut child = Command::new("xdg-open")
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
