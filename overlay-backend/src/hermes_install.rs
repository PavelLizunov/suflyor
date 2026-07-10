//! In-app installer for the suflyor Hermes plugin (ТЗ 2026-07-10: установка
//! ТОЛЬКО из приложения — без zip/скриптов, как TTS/OCR-движки).
//!
//! The plugin sources are embedded at compile time; the Settings button calls
//! [`install_plugin`], which:
//! 1. writes `plugin.yaml` + `__init__.py` into `<hermes home>/plugins/suflyor/`
//!    (overwrite = upgrade; idempotent);
//! 2. line-merges `SUFLYOR_BRIDGE_URL` / `SUFLYOR_BRIDGE_TOKEN` into
//!    `<hermes home>/.env` (everything else preserved byte-for-byte);
//! 3. adds `suflyor` to `plugins.enabled` in `<hermes home>/config.yaml` via a
//!    conservative TEXT edit — never a YAML re-dump (the user's config is a
//!    73KB commented file; `hermes plugins enable` would strip every comment).
//!    Unrecognized shapes are left untouched with a manual hint instead.
//!
//! Hermes home resolution mirrors hermes-agent's `get_hermes_home()`:
//! `HERMES_HOME` env override, else the platform-native default —
//! `%LOCALAPPDATA%\hermes` on Windows, `~/.hermes` elsewhere.

use std::path::PathBuf;

const PLUGIN_YAML: &str = include_str!("../../integrations/hermes-plugin/suflyor/plugin.yaml");
const PLUGIN_INIT: &str = include_str!("../../integrations/hermes-plugin/suflyor/__init__.py");

/// Resolve the Hermes home directory (`HERMES_HOME` env override, else the
/// platform default). Returns `None` only when no base directory can be
/// derived at all (no LOCALAPPDATA/USERPROFILE/HOME).
pub fn hermes_home() -> Option<PathBuf> {
    if let Ok(h) = std::env::var("HERMES_HOME") {
        let t = h.trim();
        if !t.is_empty() {
            return Some(PathBuf::from(t));
        }
    }
    if cfg!(windows) {
        let base = std::env::var("LOCALAPPDATA")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("USERPROFILE")
                    .ok()
                    .map(|s| PathBuf::from(s).join("AppData").join("Local"))
            })?;
        Some(base.join("hermes"))
    } else {
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join(".hermes"))
    }
}

/// The bridge URL the LOCAL Hermes should call. A loopback or wildcard bind
/// is reachable via 127.0.0.1; a specific non-loopback bind (e.g. a Tailscale
/// IP) is only reachable via that address.
pub fn bridge_url_for_env(bind_host: &str, port: u16) -> String {
    let h = bind_host.trim();
    let host = if h.is_empty() || h == "0.0.0.0" || crate::bridge::is_loopback_host(h) {
        "127.0.0.1"
    } else {
        h
    };
    format!("http://{host}:{port}")
}

/// Install the plugin into the local Hermes: files + `.env` + `config.yaml`.
/// Returns an RU status line for the Settings label (no secrets).
pub fn install_plugin(bridge_url: &str, token: &str) -> Result<String, String> {
    let home = hermes_home().ok_or_else(|| "не найден домашний каталог Hermes".to_string())?;
    if !home.is_dir() {
        return Err(format!(
            "Hermes не найден на этой машине (нет {})",
            home.display()
        ));
    }

    // 1. Plugin files (overwrite = upgrade path).
    let pdir = home.join("plugins").join("suflyor");
    std::fs::create_dir_all(&pdir).map_err(|e| format!("не создать {}: {e}", pdir.display()))?;
    std::fs::write(pdir.join("plugin.yaml"), PLUGIN_YAML)
        .map_err(|e| format!("запись plugin.yaml: {e}"))?;
    std::fs::write(pdir.join("__init__.py"), PLUGIN_INIT)
        .map_err(|e| format!("запись __init__.py: {e}"))?;

    // 2. .env merge (create if absent). Default to empty ONLY when the file
    // does not exist — any other read error (permissions, non-UTF-8) must
    // abort, or the merge would silently REPLACE the user's secrets file.
    let env_path = home.join(".env");
    let env_old = match std::fs::read_to_string(&env_path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("чтение .env: {e}")),
    };
    let env_new = merge_env_text(&env_old, bridge_url, token);
    if env_new != env_old {
        std::fs::write(&env_path, env_new).map_err(|e| format!("запись .env: {e}"))?;
    }

    // 3. config.yaml enable.
    let cfg_path = home.join("config.yaml");
    let cfg_old = if cfg_path.is_file() {
        Some(std::fs::read_to_string(&cfg_path).map_err(|e| format!("чтение config.yaml: {e}"))?)
    } else {
        None
    };
    let outcome = enable_in_config_text(cfg_old.as_deref().unwrap_or(""));
    match outcome {
        EnableEdit::Updated(new_text) => {
            std::fs::write(&cfg_path, new_text).map_err(|e| format!("запись config.yaml: {e}"))?;
            Ok("готово: плагин установлен и включён — перезапусти Hermes".to_string())
        }
        EnableEdit::AlreadyEnabled => {
            Ok("готово: плагин обновлён (уже включён) — перезапусти Hermes".to_string())
        }
        EnableEdit::Unsupported => Ok(
            "файлы установлены, но config.yaml нестандартный — выполни вручную: \
             hermes plugins enable suflyor"
                .to_string(),
        ),
    }
}

/// Ensure the local Hermes API server is enabled with a key, creating both
/// when missing (the «Настроить локальный Hermes» button — the tester should
/// never have to hand-edit YAML to learn what an API_SERVER_KEY is).
/// Returns `(key, changed)`; `changed` says config.yaml was rewritten (⇒ the
/// user must restart Hermes).
pub fn ensure_api_server() -> Result<(String, bool), String> {
    let home = hermes_home().ok_or_else(|| "не найден домашний каталог Hermes".to_string())?;
    if !home.is_dir() {
        return Err(format!(
            "Hermes не найден на этой машине (нет {})",
            home.display()
        ));
    }
    let cfg_path = home.join("config.yaml");
    let old = match std::fs::read_to_string(&cfg_path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("чтение config.yaml: {e}")),
    };
    match ensure_api_server_text(&old, crate::bridge::generate_token) {
        ApiEdit::Ready(key) => Ok((key, false)),
        ApiEdit::Updated(text, key) => {
            std::fs::write(&cfg_path, text).map_err(|e| format!("запись config.yaml: {e}"))?;
            Ok((key, true))
        }
        ApiEdit::Unsupported => Err(
            "config.yaml нестандартный — пропиши platforms.api_server.extra.key вручную"
                .to_string(),
        ),
    }
}

/// Result of the api_server config edit.
#[derive(Debug, PartialEq, Eq)]
pub enum ApiEdit {
    /// Server already enabled with this key — nothing written.
    Ready(String),
    /// New file content + the key now in effect.
    Updated(String, String),
    /// Shapes we refuse to touch (flow mappings, odd scalars).
    Unsupported,
}

/// Enable `platforms.api_server` + ensure `extra.key` via a conservative
/// line edit (same philosophy as [`enable_in_config_text`]): only bare
/// block-style keys are edited; anything exotic → [`ApiEdit::Unsupported`].
pub fn ensure_api_server_text(existing: &str, gen_key: impl Fn() -> String) -> ApiEdit {
    let eol = eol_of(existing);
    let normalized = existing.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized.split('\n').collect();

    let bare_key_at = |l: &str, name: &str| -> bool {
        let no_comment = l.split('#').next().unwrap_or("");
        no_comment.trim_end().trim_start() == format!("{name}:")
    };
    // Top-level `platforms:` (column 0, no inline value, not a comment).
    let platforms_idx = lines
        .iter()
        .position(|l| indent_of(l) == 0 && bare_key_at(l, "platforms"));
    // A top-level `platforms: {...}` flow form → refuse.
    if platforms_idx.is_none()
        && lines.iter().any(|l| {
            indent_of(l) == 0
                && l.trim_start().starts_with("platforms:")
                && !l
                    .split('#')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_start_matches("platforms:")
                    .trim()
                    .is_empty()
        })
    {
        return ApiEdit::Unsupported;
    }

    let Some(pidx) = platforms_idx else {
        // No platforms block — append a fresh one (the stock hermes config).
        let key = gen_key();
        let mut out = normalized.trim_end_matches('\n').to_string();
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!(
            "platforms:\n  api_server:\n    enabled: true\n    extra:\n      key: \"{key}\"\n"
        ));
        return finish_api(out, eol, key);
    };

    // The platforms block: up to the next top-level key.
    let mut block_end = lines.len();
    for (i, l) in lines.iter().enumerate().skip(pidx + 1) {
        if !l.trim().is_empty() && indent_of(l) == 0 {
            block_end = i;
            break;
        }
    }
    let api_idx = lines
        .iter()
        .enumerate()
        .take(block_end)
        .skip(pidx + 1)
        .find(|(_, l)| bare_key_at(l, "api_server"))
        .map(|(i, _)| i);

    let mut out_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();

    let Some(aidx) = api_idx else {
        // platforms: exists, api_server doesn't — insert the whole sub-block.
        if lines
            .iter()
            .take(block_end)
            .skip(pidx + 1)
            .any(|l| l.trim_start().starts_with("api_server:"))
        {
            return ApiEdit::Unsupported; // flow / inline form
        }
        let key = gen_key();
        let pad = " ".repeat(indent_of(lines[pidx]) + 2);
        out_lines.insert(pidx + 1, format!("{pad}api_server:"));
        out_lines.insert(pidx + 2, format!("{pad}  enabled: true"));
        out_lines.insert(pidx + 3, format!("{pad}  extra:"));
        out_lines.insert(pidx + 4, format!("{pad}    key: \"{key}\""));
        return finish_api(out_lines.join("\n"), eol, key);
    };

    // api_server block bounds: deeper-indented lines only.
    let a_indent = indent_of(lines[aidx]);
    let mut a_end = block_end;
    for (i, l) in lines.iter().enumerate().take(block_end).skip(aidx + 1) {
        if !l.trim().is_empty() && indent_of(l) <= a_indent {
            a_end = i;
            break;
        }
    }

    // Locate enabled: / extra: inside it (original indices).
    let mut enabled_idx: Option<usize> = None;
    let mut extra_idx: Option<usize> = None;
    for (i, l) in lines.iter().enumerate().take(a_end).skip(aidx + 1) {
        let t = l.split('#').next().unwrap_or("").trim();
        if t.starts_with("enabled:") && enabled_idx.is_none() {
            enabled_idx = Some(i);
        }
        if t.starts_with("extra:") && extra_idx.is_none() {
            extra_idx = Some(i);
        }
    }

    // Plan edits on ORIGINAL indices; apply bottom-up so nothing shifts.
    let mut replaces: Vec<(usize, String)> = Vec::new();
    let mut inserts: Vec<(usize, Vec<String>)> = Vec::new();

    match enabled_idx {
        Some(ei) => {
            let val = lines[ei]
                .split('#')
                .next()
                .unwrap_or("")
                .trim()
                .trim_start_matches("enabled:")
                .trim()
                .to_string();
            match val.as_str() {
                "true" => {}
                "false" => {
                    let pad = " ".repeat(indent_of(lines[ei]));
                    replaces.push((ei, format!("{pad}enabled: true")));
                }
                _ => return ApiEdit::Unsupported,
            }
        }
        None => {
            let pad = " ".repeat(a_indent + 2);
            inserts.push((aidx, vec![format!("{pad}enabled: true")]));
        }
    }

    let key = match extra_idx {
        Some(xi) => {
            let after = lines[xi]
                .split('#')
                .next()
                .unwrap_or("")
                .trim()
                .trim_start_matches("extra:")
                .trim()
                .to_string();
            if !after.is_empty() {
                return ApiEdit::Unsupported; // extra: {...}
            }
            let x_indent = indent_of(lines[xi]);
            let mut x_end = a_end;
            for (i, l) in lines.iter().enumerate().take(a_end).skip(xi + 1) {
                if !l.trim().is_empty() && indent_of(l) <= x_indent {
                    x_end = i;
                    break;
                }
            }
            let key_idx = lines
                .iter()
                .enumerate()
                .take(x_end)
                .skip(xi + 1)
                .find(|(_, l)| l.split('#').next().unwrap_or("").trim().starts_with("key:"))
                .map(|(i, _)| i);
            match key_idx {
                Some(ki) => {
                    let val = lines[ki]
                        .split('#')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .trim_start_matches("key:")
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string();
                    if val.is_empty() {
                        let key = gen_key();
                        let pad = " ".repeat(indent_of(lines[ki]));
                        replaces.push((ki, format!("{pad}key: \"{key}\"")));
                        key
                    } else {
                        val
                    }
                }
                None => {
                    let key = gen_key();
                    let pad = " ".repeat(x_indent + 2);
                    inserts.push((xi, vec![format!("{pad}key: \"{key}\"")]));
                    key
                }
            }
        }
        None => {
            let key = gen_key();
            let pad = " ".repeat(a_indent + 2);
            inserts.push((
                aidx,
                vec![format!("{pad}extra:"), format!("{pad}  key: \"{key}\"")],
            ));
            key
        }
    };

    if replaces.is_empty() && inserts.is_empty() {
        return ApiEdit::Ready(key);
    }
    for (i, text) in replaces {
        out_lines[i] = text;
    }
    // Bottom-up so earlier insertion points stay valid.
    inserts.sort_by_key(|(i, _)| std::cmp::Reverse(*i));
    for (i, rows) in inserts {
        for (n, row) in rows.into_iter().enumerate() {
            out_lines.insert(i + 1 + n, row);
        }
    }
    finish_api(out_lines.join("\n"), eol, key)
}

/// Re-apply EOL style and wrap as `Updated(text, key)`.
fn finish_api(text: String, eol: &str, key: String) -> ApiEdit {
    let mut t = text;
    if !t.ends_with('\n') {
        t.push('\n');
    }
    if eol == "\r\n" {
        t = t.replace('\n', "\r\n");
    }
    ApiEdit::Updated(t, key)
}

/// Result of the conservative `config.yaml` text edit.
#[derive(Debug, PartialEq, Eq)]
pub enum EnableEdit {
    /// New file content to write (plugin appended to `plugins.enabled`).
    Updated(String),
    /// `suflyor` is already in the enabled list — nothing to write.
    AlreadyEnabled,
    /// The `plugins:` block has a shape we refuse to touch (flow mapping,
    /// non-list `enabled:` …) — the caller shows a manual hint instead.
    Unsupported,
}

/// Detect the file's dominant line ending so the edit doesn't churn it.
fn eol_of(text: &str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// Merge `SUFLYOR_BRIDGE_URL` / `SUFLYOR_BRIDGE_TOKEN` into dotenv text:
/// replace existing assignments in place, append missing ones at the end.
/// Every other line is preserved verbatim. Idempotent.
pub fn merge_env_text(existing: &str, url: &str, token: &str) -> String {
    let eol = eol_of(existing);
    let normalized = existing.replace("\r\n", "\n");
    let url_line = format!("SUFLYOR_BRIDGE_URL={url}");
    let token_line = format!("SUFLYOR_BRIDGE_TOKEN={token}");
    let mut saw_url = false;
    let mut saw_token = false;
    let mut out: Vec<String> = Vec::new();
    for line in normalized.split('\n') {
        let t = line.trim_start();
        if t.starts_with("SUFLYOR_BRIDGE_URL=") {
            out.push(url_line.clone());
            saw_url = true;
        } else if t.starts_with("SUFLYOR_BRIDGE_TOKEN=") {
            out.push(token_line.clone());
            saw_token = true;
        } else {
            out.push(line.to_string());
        }
    }
    // Drop a single trailing empty segment (split artifact of a trailing \n)
    // so appends don't create blank-line drift; re-added by join+push below.
    if out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    if !saw_url || !saw_token {
        out.push(String::new());
        out.push("# suflyor bridge (added by the suflyor app)".to_string());
        if !saw_url {
            out.push(url_line);
        }
        if !saw_token {
            out.push(token_line);
        }
    }
    let mut joined = out.join("\n");
    joined.push('\n');
    if eol == "\r\n" {
        joined = joined.replace('\n', "\r\n");
    }
    joined
}

/// Indent width (spaces) of a line. Tabs are treated as unsupported YAML
/// indentation by the callers (they only compare equality/ordering, and a
/// tab-indented hermes config never ships), so counting spaces suffices.
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

/// Add `- suflyor` to `plugins.enabled` via a conservative line edit.
/// Handles: no file / no `plugins:` key (append block), block-style
/// `enabled:` lists (insert item), `enabled: []` (convert to block),
/// already-listed (no-op). Anything else → [`EnableEdit::Unsupported`].
pub fn enable_in_config_text(existing: &str) -> EnableEdit {
    let eol = eol_of(existing);
    let normalized = existing.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized.split('\n').collect();

    // Locate a TOP-LEVEL `plugins:` key (column 0; ignore comments).
    let plugins_idx = lines.iter().position(|l| {
        let no_comment = l.split('#').next().unwrap_or("");
        indent_of(l) == 0 && no_comment.trim_end() == "plugins:"
    });

    // Top-level `plugins:` with inline content (flow mapping) → refuse.
    let has_flow_plugins = lines.iter().any(|l| {
        indent_of(l) == 0
            && l.trim_start().starts_with("plugins:")
            && !l
                .split('#')
                .next()
                .unwrap_or("")
                .trim_end()
                .trim_start_matches("plugins:")
                .trim()
                .is_empty()
    });
    if plugins_idx.is_none() && has_flow_plugins {
        return EnableEdit::Unsupported;
    }

    let Some(pidx) = plugins_idx else {
        // No plugins block at all — append one (the common fresh-install case).
        let mut out = normalized.trim_end_matches('\n').to_string();
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("plugins:\n  enabled:\n    - suflyor\n");
        return finish(out, eol);
    };

    // Scan the plugins block: lines after `plugins:` until the next
    // top-level key (indent 0 with content).
    let mut block_end = lines.len();
    for (i, l) in lines.iter().enumerate().skip(pidx + 1) {
        if !l.trim().is_empty() && indent_of(l) == 0 {
            block_end = i;
            break;
        }
    }

    // Find `enabled:` inside the block — the SHALLOWEST match, so a nested
    // `plugins.entries.<x>.enabled:` never shadows the real allow-list.
    let mut enabled_idx: Option<usize> = None;
    for (i, l) in lines.iter().enumerate().take(block_end).skip(pidx + 1) {
        let no_comment = l.split('#').next().unwrap_or("");
        if no_comment.trim().starts_with("enabled:") {
            let shallower = match enabled_idx {
                Some(prev) => indent_of(l) < indent_of(lines[prev]),
                None => true,
            };
            if shallower {
                enabled_idx = Some(i);
            }
        }
    }

    let mut out_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();

    match enabled_idx {
        None => {
            // `plugins:` exists but no `enabled:` — insert both lines right
            // after the `plugins:` line.
            out_lines.insert(pidx + 1, "  enabled:".to_string());
            out_lines.insert(pidx + 2, "    - suflyor".to_string());
        }
        Some(eidx) => {
            let eline = &lines[eidx];
            let e_indent = indent_of(eline);
            let after_colon = eline
                .split('#')
                .next()
                .unwrap_or("")
                .trim()
                .trim_start_matches("enabled:")
                .trim()
                .to_string();
            if after_colon == "[]" {
                // Empty flow list → convert to a block list with our item.
                let pad = " ".repeat(e_indent);
                out_lines[eidx] = format!("{pad}enabled:");
                out_lines.insert(eidx + 1, format!("{pad}  - suflyor"));
                return finish(out_lines.join("\n"), eol);
            }
            if !after_colon.is_empty() {
                // Non-empty flow list / scalar — refuse to guess.
                return EnableEdit::Unsupported;
            }
            // Block list: walk items (deeper-indented `- …` lines).
            let mut item_indent: Option<usize> = None;
            let mut last_item = eidx;
            for (i, l) in lines.iter().enumerate().take(block_end).skip(eidx + 1) {
                if l.trim().is_empty() {
                    continue;
                }
                let ind = indent_of(l);
                if ind <= e_indent {
                    break;
                }
                let t = l.trim_start();
                if let Some(item) = t.strip_prefix("- ").or_else(|| t.strip_prefix("-")) {
                    let name = item.trim().trim_matches('"').trim_matches('\'');
                    if name == "suflyor" {
                        return EnableEdit::AlreadyEnabled;
                    }
                    item_indent.get_or_insert(ind);
                    last_item = i;
                }
            }
            let pad = " ".repeat(item_indent.unwrap_or(e_indent + 2));
            out_lines.insert(last_item + 1, format!("{pad}- suflyor"));
        }
    }
    finish(out_lines.join("\n"), eol)
}

/// Re-apply the original EOL style and wrap as `Updated`.
fn finish(text: String, eol: &str) -> EnableEdit {
    let mut t = text;
    if !t.ends_with('\n') {
        t.push('\n');
    }
    if eol == "\r\n" {
        t = t.replace('\n', "\r\n");
    }
    EnableEdit::Updated(t)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]
    use super::*;

    #[test]
    fn embedded_plugin_files_nonempty() {
        assert!(PLUGIN_YAML.contains("name: suflyor"));
        assert!(PLUGIN_INIT.contains("def register(ctx)"));
    }

    #[test]
    fn env_merge_appends_when_missing() {
        let out = merge_env_text("FOO=1\n", "http://127.0.0.1:8654", "tok123");
        assert!(out.contains("FOO=1\n"));
        assert!(out.contains("SUFLYOR_BRIDGE_URL=http://127.0.0.1:8654\n"));
        assert!(out.contains("SUFLYOR_BRIDGE_TOKEN=tok123\n"));
    }

    #[test]
    fn env_merge_replaces_in_place_and_is_idempotent() {
        let old = "A=1\nSUFLYOR_BRIDGE_URL=http://old:1\nB=2\nSUFLYOR_BRIDGE_TOKEN=oldtok\n";
        let out = merge_env_text(old, "http://127.0.0.1:9000", "newtok");
        assert_eq!(
            out,
            "A=1\nSUFLYOR_BRIDGE_URL=http://127.0.0.1:9000\nB=2\nSUFLYOR_BRIDGE_TOKEN=newtok\n"
        );
        let again = merge_env_text(&out, "http://127.0.0.1:9000", "newtok");
        assert_eq!(again, out);
    }

    #[test]
    fn env_merge_preserves_crlf() {
        let old = "A=1\r\n";
        let out = merge_env_text(old, "http://127.0.0.1:8654", "t");
        assert!(out.contains("\r\n"));
        assert!(!out.replace("\r\n", "").contains('\r'));
        assert!(out.contains("SUFLYOR_BRIDGE_TOKEN=t\r\n"));
    }

    #[test]
    fn config_append_when_no_plugins_key() {
        let out = enable_in_config_text("agent:\n  model: x\n");
        let EnableEdit::Updated(t) = out else {
            panic!("expected Updated")
        };
        assert_eq!(
            t,
            "agent:\n  model: x\nplugins:\n  enabled:\n    - suflyor\n"
        );
    }

    #[test]
    fn config_create_when_empty() {
        let EnableEdit::Updated(t) = enable_in_config_text("") else {
            panic!("expected Updated")
        };
        assert_eq!(t, "plugins:\n  enabled:\n    - suflyor\n");
    }

    #[test]
    fn config_inserts_into_existing_enabled_list() {
        let src = "plugins:\n  enabled:\n    - other\n  disabled: []\ntop: 1\n";
        let EnableEdit::Updated(t) = enable_in_config_text(src) else {
            panic!("expected Updated")
        };
        assert_eq!(
            t,
            "plugins:\n  enabled:\n    - other\n    - suflyor\n  disabled: []\ntop: 1\n"
        );
    }

    #[test]
    fn config_adds_enabled_under_bare_plugins() {
        let src = "plugins:\n  disabled:\n    - x\n";
        let EnableEdit::Updated(t) = enable_in_config_text(src) else {
            panic!("expected Updated")
        };
        assert_eq!(
            t,
            "plugins:\n  enabled:\n    - suflyor\n  disabled:\n    - x\n"
        );
    }

    #[test]
    fn config_already_enabled_detected() {
        let src = "plugins:\n  enabled:\n    - suflyor\n";
        assert_eq!(enable_in_config_text(src), EnableEdit::AlreadyEnabled);
        // Quoted form too.
        let src2 = "plugins:\n  enabled:\n    - \"suflyor\"\n";
        assert_eq!(enable_in_config_text(src2), EnableEdit::AlreadyEnabled);
    }

    #[test]
    fn config_empty_flow_list_converted() {
        let src = "plugins:\n  enabled: []\n";
        let EnableEdit::Updated(t) = enable_in_config_text(src) else {
            panic!("expected Updated")
        };
        assert_eq!(t, "plugins:\n  enabled:\n    - suflyor\n");
    }

    #[test]
    fn config_flow_forms_unsupported() {
        assert_eq!(
            enable_in_config_text("plugins: {enabled: [a]}\n"),
            EnableEdit::Unsupported
        );
        assert_eq!(
            enable_in_config_text("plugins:\n  enabled: [a, b]\n"),
            EnableEdit::Unsupported
        );
    }

    #[test]
    fn config_crlf_preserved() {
        let src = "top: 1\r\n";
        let EnableEdit::Updated(t) = enable_in_config_text(src) else {
            panic!("expected Updated")
        };
        assert_eq!(t, "top: 1\r\nplugins:\r\n  enabled:\r\n    - suflyor\r\n");
    }

    #[test]
    fn config_block_ends_at_next_top_level_key() {
        // `enabled:` belongs to ANOTHER top-level key after plugins — the
        // scan must not cross into it.
        let src = "plugins:\n  disabled: []\nother:\n  enabled:\n    - x\n";
        let EnableEdit::Updated(t) = enable_in_config_text(src) else {
            panic!("expected Updated")
        };
        assert_eq!(
            t,
            "plugins:\n  enabled:\n    - suflyor\n  disabled: []\nother:\n  enabled:\n    - x\n"
        );
    }

    fn genkey() -> String {
        "GENKEY".to_string()
    }

    #[test]
    fn api_appends_block_to_stock_config() {
        // The stock hermes config has NO uncommented `platforms:` — only the
        // commented example block (mirrors the real 73KB template).
        let src = "agent:\n  model: x\n#   platforms:\n#     api_server:\n#       enabled: true\n";
        let ApiEdit::Updated(t, key) = ensure_api_server_text(src, genkey) else {
            panic!("expected Updated")
        };
        assert_eq!(key, "GENKEY");
        assert!(t.ends_with(
            "platforms:\n  api_server:\n    enabled: true\n    extra:\n      key: \"GENKEY\"\n"
        ));
        assert!(t.contains("#   platforms:")); // comments untouched
    }

    #[test]
    fn api_inserts_under_existing_platforms() {
        let src = "platforms:\n  telegram:\n    enabled: true\ntop: 1\n";
        let ApiEdit::Updated(t, _) = ensure_api_server_text(src, genkey) else {
            panic!("expected Updated")
        };
        assert_eq!(
            t,
            "platforms:\n  api_server:\n    enabled: true\n    extra:\n      key: \"GENKEY\"\n  telegram:\n    enabled: true\ntop: 1\n"
        );
    }

    #[test]
    fn api_flips_enabled_false_keeps_key() {
        let src =
            "platforms:\n  api_server:\n    enabled: false\n    extra:\n      key: \"OLDKEY\"\n";
        let ApiEdit::Updated(t, key) = ensure_api_server_text(src, genkey) else {
            panic!("expected Updated")
        };
        assert_eq!(key, "OLDKEY");
        assert!(t.contains("    enabled: true\n"));
        assert!(t.contains("key: \"OLDKEY\""));
    }

    #[test]
    fn api_ready_when_all_set() {
        let src = "platforms:\n  api_server:\n    enabled: true\n    extra:\n      key: mykey\n";
        assert_eq!(
            ensure_api_server_text(src, genkey),
            ApiEdit::Ready("mykey".to_string())
        );
    }

    #[test]
    fn api_inserts_missing_enabled_keeps_key() {
        let src = "platforms:\n  api_server:\n    extra:\n      key: \"K1\"\n";
        let ApiEdit::Updated(t, key) = ensure_api_server_text(src, genkey) else {
            panic!("expected Updated")
        };
        assert_eq!(key, "K1");
        assert!(t.contains("    enabled: true\n"));
    }

    #[test]
    fn api_fills_empty_key() {
        let src = "platforms:\n  api_server:\n    enabled: true\n    extra:\n      key: \"\"\n";
        let ApiEdit::Updated(t, key) = ensure_api_server_text(src, genkey) else {
            panic!("expected Updated")
        };
        assert_eq!(key, "GENKEY");
        assert!(t.contains("      key: \"GENKEY\"\n"));
    }

    #[test]
    fn api_missing_extra_added() {
        let src = "platforms:\n  api_server:\n    enabled: true\n";
        let ApiEdit::Updated(t, _) = ensure_api_server_text(src, genkey) else {
            panic!("expected Updated")
        };
        assert_eq!(
            t,
            "platforms:\n  api_server:\n    extra:\n      key: \"GENKEY\"\n    enabled: true\n"
        );
    }

    #[test]
    fn api_flow_forms_unsupported() {
        assert_eq!(
            ensure_api_server_text("platforms: {api_server: {enabled: true}}\n", genkey),
            ApiEdit::Unsupported
        );
        assert_eq!(
            ensure_api_server_text(
                "platforms:\n  api_server:\n    enabled: true\n    extra: {key: x}\n",
                genkey
            ),
            ApiEdit::Unsupported
        );
        assert_eq!(
            ensure_api_server_text("platforms:\n  api_server:\n    enabled: maybe\n", genkey),
            ApiEdit::Unsupported
        );
    }

    #[test]
    fn api_crlf_preserved() {
        let src = "top: 1\r\n";
        let ApiEdit::Updated(t, _) = ensure_api_server_text(src, genkey) else {
            panic!("expected Updated")
        };
        assert!(t.contains("platforms:\r\n  api_server:\r\n"));
    }

    #[test]
    fn bridge_url_host_selection() {
        assert_eq!(bridge_url_for_env("", 8654), "http://127.0.0.1:8654");
        assert_eq!(
            bridge_url_for_env("127.0.0.1", 8654),
            "http://127.0.0.1:8654"
        );
        assert_eq!(bridge_url_for_env("0.0.0.0", 8654), "http://127.0.0.1:8654");
        assert_eq!(
            bridge_url_for_env("100.64.0.5", 9000),
            "http://100.64.0.5:9000"
        );
    }
}
