//! Settings → Hermes tab (ТЗ 2026-07-09, `docs/goal-hermes-integration-2026-07-09.md`).
//!
//! Two directions:
//! - **Bridge** (Hermes → suflyor): a loopback `overlay_backend::bridge` server the
//!   local Hermes agent reads from. Toggle starts/stops it LIVE; port + token are
//!   editable; a button mints a fresh token. Off by default; no token ⇒ won't start.
//! - **Hermes API** (suflyor → Hermes): url + key for the agent's OpenAI-compatible
//!   server, a connection test, and «Подготовить профиль» — a slow agentic call that
//!   researches a seed line into a new active call-profile (never on the live path).
//!
//! The running bridge handle lives in a thread-local (the Settings window is
//! reused/dropped, the bridge must outlive it) — mirrors `transcript_player`'s
//! PLAYER slot. `apply_bridge_state` is the single start/stop entry: called at
//! boot (auto-start when enabled) and by the toggle.

use super::{ComponentHandle, SettingsWindow, SharedString};
use overlay_backend::bridge::{self, BridgeHandle};
use std::cell::RefCell;

thread_local! {
    /// The single live bridge server, `Some` while running. Started at boot if
    /// `hermes_bridge_enabled`, and toggled from Settings.
    static BRIDGE: RefCell<Option<BridgeHandle>> = const { RefCell::new(None) };
}

/// RU status line reflecting the LIVE bridge state (thread-local handle), for the
/// Settings label. Used by `populate_token_status` on every (re)open so the label
/// is never stale, and by the seed in `wire_hermes_settings`.
pub(crate) fn current_bridge_status(host: &str, port: u16) -> String {
    if BRIDGE.with(|b| b.borrow().is_some()) {
        let h = if host.trim().is_empty() {
            "127.0.0.1"
        } else {
            host.trim()
        };
        format!("включён · {h}:{port}")
    } else {
        "выключен".to_string()
    }
}

/// Start or stop the bridge to match `cfg.hermes_bridge_enabled`. Idempotent:
/// stops any running instance first, then (re)starts when enabled. Returns a
/// short RU status line for the Settings label (safe to show; no secrets).
/// Call at boot and whenever the toggle / port / token changes.
pub(crate) fn apply_bridge_state(cfg: &overlay_backend::config::SharedConfig) -> String {
    // Always stop the current one first (so a port/token edit re-binds cleanly).
    BRIDGE.with(|b| {
        if let Some(h) = b.borrow_mut().take() {
            h.stop();
        }
    });
    let enabled = cfg.read().hermes_bridge_enabled;
    if !enabled {
        return "выключен".to_string();
    }
    match bridge::start(cfg.clone()) {
        Ok(handle) => {
            let (host, port) = {
                let c = cfg.read();
                (c.hermes_bridge_host.clone(), c.hermes_bridge_port)
            };
            BRIDGE.with(|b| *b.borrow_mut() = Some(handle));
            current_bridge_status(&host, port)
        }
        Err(e) => format!("ошибка: {e}"),
    }
}

/// Wire the Settings → Hermes tab. Mirrors `wire_ai_settings`: seed the fields
/// from config, and each callback writes config + saves (+ re-applies the bridge
/// where relevant). All heavy work (the profile-prep call) runs off-thread.
pub(crate) fn wire_hermes_settings(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::SharedConfig,
) {
    // -- seed fields from config --
    {
        let c = cfg.read();
        win.set_hermes_bridge_enabled(c.hermes_bridge_enabled);
        win.set_hermes_bridge_port(SharedString::from(c.hermes_bridge_port.to_string()));
        win.set_hermes_bridge_token(SharedString::from(c.hermes_bridge_token.clone()));
        win.set_hermes_bridge_host(SharedString::from(c.hermes_bridge_host.clone()));
        win.set_hermes_bridge_remote(!bridge::is_loopback_host(&c.hermes_bridge_host));
        win.set_hermes_api_url(SharedString::from(c.hermes_api_url.clone()));
        win.set_hermes_api_key(SharedString::from(c.hermes_api_key.clone()));
    }
    // Reflect the CURRENT live state (boot may have started it already).
    let (host, port) = {
        let c = cfg.read();
        (c.hermes_bridge_host.clone(), c.hermes_bridge_port)
    };
    win.set_hermes_bridge_status(SharedString::from(current_bridge_status(&host, port)));

    // -- toggle: persist + start/stop live --
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_hermes_bridge_toggled(move |on| {
            {
                let mut c = cfg_c.write();
                c.hermes_bridge_enabled = on;
                // Minting on first enable saves a click: a blank token would just
                // refuse to start.
                if on && c.hermes_bridge_token.trim().is_empty() {
                    c.hermes_bridge_token = bridge::generate_token();
                }
                let _ = overlay_backend::config::save(&c);
            }
            let status = apply_bridge_state(&cfg_c);
            if let Some(w) = weak.upgrade() {
                w.set_hermes_bridge_token(SharedString::from(
                    cfg_c.read().hermes_bridge_token.clone(),
                ));
                w.set_hermes_bridge_status(SharedString::from(status));
            }
        });
    }

    // -- port save (re-applies the bridge if running) --
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_hermes_bridge_port_save(move |txt| {
            let Ok(port) = txt.trim().parse::<u16>() else {
                if let Some(w) = weak.upgrade() {
                    w.set_hermes_bridge_status(SharedString::from("порт: нужно число 1–65535"));
                }
                return;
            };
            {
                let mut c = cfg_c.write();
                c.hermes_bridge_port = port;
                let _ = overlay_backend::config::save(&c);
            }
            let status = apply_bridge_state(&cfg_c);
            if let Some(w) = weak.upgrade() {
                w.set_hermes_bridge_status(SharedString::from(status));
            }
        });
    }

    // -- regenerate token (re-applies if running) --
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_hermes_bridge_regen_token(move || {
            let token = bridge::generate_token();
            {
                let mut c = cfg_c.write();
                c.hermes_bridge_token = token.clone();
                let _ = overlay_backend::config::save(&c);
            }
            let status = apply_bridge_state(&cfg_c);
            if let Some(w) = weak.upgrade() {
                w.set_hermes_bridge_token(SharedString::from(token));
                w.set_hermes_bridge_status(SharedString::from(status));
            }
        });
    }

    // -- «Установить плагин в Hermes» (ТЗ 2026-07-10: установка ТОЛЬКО из
    // приложения — файлы + .env + config.yaml, см. hermes_install.rs) --
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_hermes_plugin_install(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_hermes_plugin_install_status(SharedString::from("устанавливаю…"));
            // The .env write needs a real token — mint on first use, exactly
            // like the enable-toggle does.
            let (host, port, token, bridge_on) = {
                let mut c = cfg_c.write();
                if c.hermes_bridge_token.trim().is_empty() {
                    c.hermes_bridge_token = bridge::generate_token();
                    let _ = overlay_backend::config::save(&c);
                }
                (
                    c.hermes_bridge_host.clone(),
                    c.hermes_bridge_port,
                    c.hermes_bridge_token.clone(),
                    c.hermes_bridge_enabled,
                )
            };
            w.set_hermes_bridge_token(SharedString::from(token.clone()));
            let url = overlay_backend::hermes_install::bridge_url_for_env(&host, port);
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let msg = match overlay_backend::hermes_install::install_plugin(&url, &token) {
                    Ok(s) if bridge_on => s,
                    Ok(s) => format!("{s} · и включи «Мост для Hermes» выше"),
                    Err(e) => format!("ошибка: {e}"),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_res.upgrade() {
                        w.set_hermes_plugin_install_status(SharedString::from(msg));
                    }
                });
            });
        });
    }

    // -- bind host save (re-applies the bridge if running; updates the warning) --
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_hermes_bridge_host_save(move |txt| {
            let host = txt.trim().to_string();
            {
                let mut c = cfg_c.write();
                c.hermes_bridge_host = host.clone();
                let _ = overlay_backend::config::save(&c);
            }
            let status = apply_bridge_state(&cfg_c);
            if let Some(w) = weak.upgrade() {
                w.set_hermes_bridge_remote(!bridge::is_loopback_host(&host));
                w.set_hermes_bridge_status(SharedString::from(status));
            }
        });
    }

    // -- Hermes API url/key save --
    {
        let cfg_c = cfg.clone();
        win.on_hermes_api_url_save(move |txt| {
            let mut c = cfg_c.write();
            c.hermes_api_url = txt.trim().to_string();
            let _ = overlay_backend::config::save(&c);
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_hermes_api_key_save(move |txt| {
            let mut c = cfg_c.write();
            c.hermes_api_key = txt.trim().to_string();
            let _ = overlay_backend::config::save(&c);
        });
    }

    // -- «Взять ключ из локального Hermes» (тестер не обязан знать, что такое
    // API_SERVER_KEY): включает platforms.api_server в конфиге ЛОКАЛЬНОГО
    // Hermes, создаёт/читает ключ и вписывает его в suflyor. --
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_hermes_api_setup(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_hermes_api_setup_status(SharedString::from("настраиваю…"));
            let cfg_save = cfg_c.clone();
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let result = overlay_backend::hermes_install::ensure_api_server();
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(w) = weak_res.upgrade() else { return };
                    match result {
                        Ok((key, changed)) => {
                            {
                                let mut c = cfg_save.write();
                                c.hermes_api_key = key.clone();
                                let _ = overlay_backend::config::save(&c);
                            }
                            w.set_hermes_api_key(SharedString::from(key));
                            w.set_hermes_api_setup_status(SharedString::from(if changed {
                                "готово: API-сервер включён, ключ создан и вписан — перезапусти Hermes"
                            } else {
                                "готово: ключ взят из Hermes и вписан сюда"
                            }));
                        }
                        Err(e) => {
                            w.set_hermes_api_setup_status(SharedString::from(format!(
                                "ошибка: {e}"
                            )));
                        }
                    }
                });
            });
        });
    }

    // -- connection test. NOT ai::test_connection (10s timeout): the Hermes
    // gateway runs a FULL agentic turn even for a 1-token ping (huge system
    // prompt → slow prefill), so 10s false-negatives on a healthy setup.
    // ai::complete carries the 180s budget the agent path actually needs. --
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_hermes_api_test(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_hermes_api_test_result(SharedString::from(
                "проверка… (агентный вызов — до пары минут)",
            ));
            let (url, key) = {
                let c = cfg_c.read();
                (c.hermes_api_url.clone(), c.hermes_api_key.clone())
            };
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let messages = vec![overlay_backend::ai::ChatMessage {
                    role: "user".to_string(),
                    content: overlay_backend::ai::MessageContent::Text(
                        "Ответь одним словом: ok".to_string(),
                    ),
                }];
                let msg = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => match rt.block_on(overlay_backend::ai::complete(
                        &url,
                        &key,
                        "hermes-agent",
                        messages,
                        16,
                    )) {
                        Ok(_) => "[ok] агент ответил".to_string(),
                        Err(e) => format!("[err] {e:#}").chars().take(90).collect(),
                    },
                    Err(e) => format!("[err] runtime: {e}"),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_res.upgrade() {
                        w.set_hermes_api_test_result(SharedString::from(msg));
                    }
                });
            });
        });
    }

    // -- «Подготовить профиль» (P3): seed line → Hermes → new active profile --
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_hermes_prepare_profile(move |seed| {
            let seed = seed.trim().to_string();
            let Some(w) = weak.upgrade() else { return };
            if seed.is_empty() {
                w.set_hermes_profile_status(SharedString::from("впиши вводную (компания/роль)"));
                return;
            }
            w.set_hermes_profile_status(SharedString::from("Hermes готовит профиль… (это долго)"));
            let (url, key) = {
                let c = cfg_c.read();
                (c.hermes_api_url.clone(), c.hermes_api_key.clone())
            };
            let cfg_save = cfg_c.clone();
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let status = run_prepare_profile(&url, &key, &seed, &cfg_save);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_res.upgrade() {
                        // Refresh the profile list + status on success.
                        {
                            let snap = cfg_save.read();
                            super::refresh_profiles(&w, &snap);
                        }
                        w.set_hermes_profile_status(SharedString::from(status));
                    }
                });
            });
        });
    }
}

/// Off-thread: ask Hermes to research `seed` into a call-profile context, then
/// upsert it as a new ACTIVE profile in config. Returns an RU status line.
/// `profile_name` is derived from the seed's first line (capped), so re-prepping
/// the same seed updates the same profile instead of piling up duplicates.
fn run_prepare_profile(
    url: &str,
    key: &str,
    seed: &str,
    cfg: &overlay_backend::config::SharedConfig,
) -> String {
    let prompt = format!(
        "Ты помогаешь подготовиться к рабочему созвону/собеседованию. По вводной ниже \
         собери КОМПАКТНУЮ справку-контекст (на русском), которую ассистент будет \
         использовать как фон при ответах: о компании/продукте, вероятных темах и \
         терминах, на что обратить внимание. Без воды, маркдаун-списками. Вводная:\n\n{seed}"
    );
    let messages = vec![overlay_backend::ai::ChatMessage {
        role: "user".to_string(),
        content: overlay_backend::ai::MessageContent::Text(prompt),
    }];
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => return format!("ошибка runtime: {e}"),
    };
    // Non-streaming collect; big token budget (the qwen-kit handoff: a thinking
    // model needs ≥3000 or content comes back empty). Generous — this is slow.
    let answer = rt.block_on(async {
        overlay_backend::ai::complete(url, key, "hermes-agent", messages, 8000).await
    });
    match answer {
        Ok(text) if !text.trim().is_empty() => {
            let name = profile_name_from_seed(seed);
            let mut c = cfg.write();
            match c.context_profiles.iter_mut().find(|p| p.name == name) {
                Some(p) => p.context = text.clone(),
                None => c
                    .context_profiles
                    .push(overlay_backend::config::ContextProfile {
                        name: name.clone(),
                        context: text.clone(),
                    }),
            }
            c.active_profile = Some(name.clone());
            c.meeting_context = text;
            let _ = overlay_backend::config::save(&c);
            format!("готово: профиль «{name}» создан и активен")
        }
        Ok(_) => "Hermes вернул пустой ответ (модель не дала контент)".to_string(),
        Err(e) => format!("ошибка Hermes: {e:#}").chars().take(120).collect(),
    }
}

/// A profile name from the seed's first line: trimmed, ≤ 48 chars, never blank.
fn profile_name_from_seed(seed: &str) -> String {
    let first = seed.lines().next().unwrap_or(seed).trim();
    let name: String = first.chars().take(48).collect();
    if name.is_empty() {
        "Созвон (Hermes)".to_string()
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::profile_name_from_seed;

    #[test]
    fn profile_name_derivation() {
        assert_eq!(
            profile_name_from_seed("Собес в Яндекс, SRE"),
            "Собес в Яндекс, SRE"
        );
        assert_eq!(
            profile_name_from_seed("  Первая строка\nвторая\nтретья  "),
            "Первая строка"
        );
        assert_eq!(profile_name_from_seed("   "), "Созвон (Hermes)");
        // Long line is capped to 48 chars.
        assert_eq!(profile_name_from_seed(&"я".repeat(100)).chars().count(), 48);
    }
}
