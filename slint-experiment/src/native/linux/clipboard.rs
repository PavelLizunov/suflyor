//! Wayland clipboard transport via the worker's native wl-clipboard tools.

use std::io::Write;
use std::process::{Command, Stdio};

/// Read text without appending a newline; empty clipboard means no selection.
pub fn read_text() -> Option<String> {
    let output = Command::new("timeout")
        .args(["2s", "wl-paste", "--no-newline", "--type", "text"])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .filter(|text| !text.is_empty())
}

/// Read the actual selected text without synthesizing input into another app.
pub fn read_primary() -> Option<String> {
    let output = Command::new("timeout")
        .args([
            "--signal=KILL",
            "2s",
            "wl-paste",
            "--primary",
            "--no-newline",
            "--type",
            "text",
        ])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .filter(|text| !text.is_empty())
}

/// Replace the Wayland selection without passing its contents in process args.
pub fn set_text(text: &str) -> Result<(), String> {
    let mut child = Command::new("timeout")
        .args(["2s", "wl-copy", "--type", "text/plain;charset=utf-8"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Wayland clipboard tools are unavailable".to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        if stdin.write_all(text.as_bytes()).is_err() {
            drop(stdin);
            let _ = child.kill();
            let _ = child.wait();
            return Err("Wayland clipboard write failed".into());
        }
    }
    if child
        .wait()
        .map_err(|_| "Wayland clipboard write failed")?
        .success()
    {
        Ok(())
    } else {
        Err("Wayland clipboard write failed".into())
    }
}

pub fn write_text(text: &str) {
    let _ = set_text(text);
}

pub fn clear() {
    let _ = set_text("");
}
