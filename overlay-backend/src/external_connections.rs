//! Explicit import of global OpenCode/Pi Chat Completions profiles, not agent execution.
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::{
    io::Read,
    path::{Path, PathBuf},
};

const MAX_BYTES: u64 = 1024 * 1024;
const MAX_CONNECTIONS: usize = 128;

// Never expose credentials through Debug, serialization or the UI model.
pub struct Connection {
    pub label: String,
    pub base_url: String,
    pub model: String,
    bearer: String,
}

pub fn apply(connection: &Connection, current: &crate::config::Config) -> crate::config::Config {
    let mut next = current.clone();
    if loopback(&connection.base_url) {
        next.ai_provider = "local".into();
        next.ai_local_base_url = connection.base_url.clone();
        next.ai_local_bearer = connection.bearer.clone();
        next.ai_local_model = connection.model.clone();
        next.ai_local_prep_model.clear();
    } else {
        next.ai_provider = "cloud".into();
        next.ai_base_url = connection.base_url.clone();
        next.ai_bearer = connection.bearer.clone();
        next.ai_model = connection.model.clone();
        next.prep_model = connection.model.clone();
        next.ai_reasoning_effort.clear();
    }
    next
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

pub fn discover() -> Result<Vec<Connection>> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("home directory unavailable"))?;
    let pi = env_path("PI_CODING_AGENT_DIR").unwrap_or_else(|| home.join(".pi/agent"));
    let oc = env_path("XDG_CONFIG_HOME")
        .unwrap_or_else(|| home.join(".config"))
        .join("opencode");
    let config = env_path("OPENCODE_CONFIG").unwrap_or_else(|| {
        let jsonc = oc.join("opencode.jsonc");
        if jsonc.exists() {
            jsonc
        } else {
            oc.join("opencode.json")
        }
    });
    let auth = env_path("XDG_DATA_HOME")
        .unwrap_or_else(|| home.join(".local/share"))
        .join("opencode/auth.json");
    let env = |key: &str| std::env::var(key).ok();
    let mut found = Vec::new();
    if let Some(models) = load(&pi.join("models.json"), false)? {
        parse_pi(
            &models,
            &load(&pi.join("auth.json"), false)?.unwrap_or(Value::Null),
            &env,
            &mut found,
        );
    }
    if let Some(config) = load(&config, true)? {
        parse_opencode(
            &config,
            &load(&auth, false)?.unwrap_or(Value::Null),
            &env,
            &mut found,
        );
    }
    Ok(found)
}

fn load(path: &Path, jsonc: bool) -> Result<Option<Value>> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(anyhow!("cannot read client configuration")),
    };
    let mut data = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut data)
        .map_err(|_| anyhow!("cannot read client configuration"))?;
    if data.len() as u64 > MAX_BYTES {
        return Err(anyhow!("client configuration is too large"));
    }
    let text = std::str::from_utf8(&data)
        .map_err(|_| anyhow!("invalid client configuration"))?
        .trim_start_matches('\u{feff}');
    let text = if jsonc {
        strip_jsonc(text)?
    } else {
        text.to_string()
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|_| anyhow!("invalid client configuration"))
}

// Keep string contents intact, including escaped quotes, URLs and comment-looking keys.
fn strip_jsonc(text: &str) -> Result<String> {
    let mut bytes = text.as_bytes().to_vec();
    let (mut i, mut string, mut escaped) = (0, false, false);
    while i < bytes.len() {
        let b = bytes[i];
        if string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                string = false;
            }
        } else if b == b'"' {
            string = true;
        } else if bytes.get(i..i + 2) == Some(b"//") {
            while i < bytes.len() && bytes[i] != b'\n' {
                bytes[i] = b' ';
                i += 1;
            }
            continue;
        } else if bytes.get(i..i + 2) == Some(b"/*") {
            bytes[i] = b' ';
            bytes[i + 1] = b' ';
            i += 2;
            while i + 1 < bytes.len() && &bytes[i..i + 2] != b"*/" {
                bytes[i] = b' ';
                i += 1;
            }
            if i + 1 >= bytes.len() {
                return Err(anyhow!("invalid client configuration"));
            }
            bytes[i] = b' ';
            bytes[i + 1] = b' ';
            i += 2;
            continue;
        }
        i += 1;
    }
    string = false;
    escaped = false;
    for i in 0..bytes.len() {
        let b = bytes[i];
        if string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                string = false;
            }
        } else if b == b'"' {
            string = true;
        } else if b == b','
            && bytes[i + 1..]
                .iter()
                .find(|b| !b.is_ascii_whitespace())
                .is_some_and(|b| matches!(b, b'}' | b']'))
        {
            bytes[i] = b' ';
        }
    }
    String::from_utf8(bytes).map_err(|_| anyhow!("invalid client configuration"))
}

fn loopback(url: &str) -> bool {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .is_some_and(|h| {
            h == "localhost"
                || h.trim_matches(['[', ']'])
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
        })
}

fn unsupported(v: &Value) -> bool {
    [
        "headers",
        "compat",
        "oauth",
        "samplingParams",
        "samplingParamsByThinkingLevel",
    ]
    .iter()
    .any(|key| {
        v.get(key)
            .is_some_and(|v| !v.is_null() && v.as_object().is_none_or(|m| !m.is_empty()))
    })
}

fn token(raw: &str, env: &impl Fn(&str) -> Option<String>) -> Option<String> {
    let raw = raw.trim();
    let value = if let Some(name) = raw.strip_prefix("{env:").and_then(|v| v.strip_suffix('}')) {
        env(name.trim())?
    } else if let Some(name) = raw.strip_prefix("${").and_then(|v| v.strip_suffix('}')) {
        env(name)?
    } else if let Some(name) = raw.strip_prefix('$') {
        env(name)?
    } else {
        if raw.starts_with('!') || raw.contains("{file:") || raw.contains("{command:") {
            return None;
        }
        raw.to_string()
    };
    (!value.trim().is_empty()).then(|| value.trim().to_string())
}

fn credential(
    key: Option<&Value>,
    auth: &Value,
    kind: &str,
    url: &str,
    env: &impl Fn(&str) -> Option<String>,
) -> Option<String> {
    // Pi's stored login takes precedence over models.json, including OAuth.
    if kind != "api_key" || auth.is_null() {
        if let Some(key) = key {
            return token(key.as_str()?, env);
        }
    }
    if !auth.is_null() {
        if auth["type"].as_str()? != kind {
            return None;
        }
        return token(auth["key"].as_str()?, env);
    }
    loopback(url).then(String::new)
}

fn push(
    out: &mut Vec<Connection>,
    source: &str,
    provider: &str,
    model: &str,
    url: &str,
    bearer: String,
) {
    if out.len() >= MAX_CONNECTIONS || model.trim().is_empty() {
        return;
    }
    if let Ok(url) = crate::ai::normalize_compatible_base_url(url) {
        out.push(Connection {
            label: format!("{source}: {provider}/{model}"),
            base_url: url,
            model: model.into(),
            bearer,
        });
    }
}

fn parse_pi(
    config: &Value,
    auth: &Value,
    env: &impl Fn(&str) -> Option<String>,
    out: &mut Vec<Connection>,
) {
    let Some(providers) = config["providers"].as_object() else {
        return;
    };
    for (id, p) in providers {
        if unsupported(p)
            || p.get("modelOverrides")
                .is_some_and(|v| v.as_object().is_none_or(|m| !m.is_empty()))
        {
            continue;
        }
        let Some(models) = p["models"].as_array() else {
            continue;
        };
        for m in models {
            if unsupported(m)
                || m.get("api")
                    .or_else(|| p.get("api"))
                    .and_then(Value::as_str)
                    != Some("openai-completions")
            {
                continue;
            }
            let Some(model) = m["id"].as_str() else {
                continue;
            };
            let Some(url) = m
                .get("baseUrl")
                .or_else(|| p.get("baseUrl"))
                .and_then(Value::as_str)
            else {
                continue;
            };
            let Some(bearer) = credential(p.get("apiKey"), &auth[id], "api_key", url, env) else {
                continue;
            };
            push(out, "Pi", id, model, url, bearer);
        }
    }
}

fn parse_opencode(
    config: &Value,
    auth: &Value,
    env: &impl Fn(&str) -> Option<String>,
    out: &mut Vec<Connection>,
) {
    let Some(providers) = config["provider"].as_object() else {
        return;
    };
    for (id, p) in providers {
        if config["disabled_providers"]
            .as_array()
            .is_some_and(|v| v.iter().any(|v| v == id))
            || config["enabled_providers"]
                .as_array()
                .is_some_and(|v| !v.iter().any(|v| v == id))
        {
            continue;
        }
        let options = &p["options"];
        if p["npm"] != "@ai-sdk/openai-compatible" || unsupported(options) || unsupported(p) {
            continue;
        }
        let Some(url) = options["baseURL"].as_str() else {
            continue;
        };
        let Some(models) = p["models"].as_object() else {
            continue;
        };
        for (model, m) in models {
            if unsupported(m)
                || m.get("options")
                    .is_some_and(|o| o.as_object().is_none_or(|o| !o.is_empty()))
            {
                continue;
            }
            if p["blacklist"]
                .as_array()
                .is_some_and(|list| list.iter().any(|v| v == model))
                || p["whitelist"]
                    .as_array()
                    .is_some_and(|list| !list.iter().any(|v| v == model))
            {
                continue;
            }
            let Some(bearer) = credential(options.get("apiKey"), &auth[id], "api", url, env) else {
                continue;
            };
            let request_model = m.get("id").and_then(Value::as_str).unwrap_or(model);
            push(out, "OpenCode", id, request_model, url, bearer);
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    #[test]
    fn opencode_documented_options_and_auth_are_imported() {
        let config = json!({"provider":{"demo":{"npm":"@ai-sdk/openai-compatible","options":{"baseURL":"https://example.invalid/v1"},"models":{"answer":{}}}}});
        let auth = json!({"demo":{"type":"api","key":"synthetic-token"}});
        let mut out = Vec::new();
        parse_opencode(&config, &auth, &|_| None, &mut out);
        assert_eq!(out.len(), 1);
        let current = crate::config::Config::defaults();
        let next = apply(&out[0], &current);
        assert_eq!(next.ai_endpoint(false).model, "answer");
        assert_eq!(next.ai_endpoint(true).model, "answer");
        assert_eq!(next.ai_bearer, "synthetic-token");
        assert_eq!(next.ai_local_model, current.ai_local_model);
        assert_eq!(next.stt_provider, current.stt_provider);
        assert_eq!(next.vision_provider, current.vision_provider);
        assert_eq!(next.meeting_context, current.meeting_context);
    }

    #[test]
    fn pi_model_overrides_and_environment_tokens() {
        let config = json!({"providers":{"demo":{"api":"anthropic-messages","baseUrl":"https://example.invalid/v1","apiKey":"$TOKEN","models":[{"id":"answer","api":"openai-completions","baseUrl":"http://localhost:9191/v1"},{"id":"unsupported"}]}}});
        let mut out = Vec::new();
        parse_pi(
            &config,
            &Value::Null,
            &|k| (k == "TOKEN").then(|| "synthetic".into()),
            &mut out,
        );
        assert_eq!(out.len(), 1);
        let next = apply(&out[0], &crate::config::Config::defaults());
        assert_eq!(next.ai_provider, "local");
        assert_eq!(next.ai_endpoint(true).model, "answer");
        assert_eq!(next.ai_endpoint(false).base_url, "http://localhost:9191/v1");
    }

    #[test]
    fn unsupported_credentials_are_not_used_as_bearers() {
        for raw in [
            "!command",
            "{file:/not-read}",
            "$MISSING",
            "{env:MISSING}",
            "",
        ] {
            assert!(token(raw, &|_| None).is_none());
        }
        assert_eq!(
            token("{env:TOKEN}", &|_| Some("synthetic".into())),
            Some("synthetic".into())
        );
        assert!(credential(
            None,
            &json!({"type":"oauth","access":"not-an-api-key"}),
            "api",
            "http://localhost/v1",
            &|_| None
        )
        .is_none());
        assert!(credential(
            None,
            &Value::Null,
            "api",
            "https://example.invalid/v1",
            &|_| None
        )
        .is_none());
        assert!(unsupported(
            &json!({"headers":{"Authorization":"synthetic"}})
        ));
        assert!(unsupported(&json!({"oauth":"radius"})));
    }

    #[test]
    fn pi_stored_login_precedes_config_and_disabled_opencode_providers_are_skipped() {
        let key = json!("config-token");
        assert_eq!(
            credential(
                Some(&key),
                &json!({"type":"api_key","key":"stored-token"}),
                "api_key",
                "https://example.invalid/v1",
                &|_| None
            ),
            Some("stored-token".into())
        );
        assert!(credential(
            Some(&key),
            &json!({"type":"oauth","access":"not-an-api-key"}),
            "api_key",
            "https://example.invalid/v1",
            &|_| None
        )
        .is_none());
        let mut config = json!({"provider":{"demo":{"npm":"@ai-sdk/openai-compatible",
            "options":{"baseURL":"http://localhost:9191"}, "models":{"alias":{"id":"actual-model"}}}}});
        let mut out = Vec::new();
        parse_opencode(&config, &Value::Null, &|_| None, &mut out);
        assert_eq!(out[0].model, "actual-model");
        assert_eq!(out[0].base_url, "http://localhost:9191");
        config["disabled_providers"] = json!(["demo"]);
        out.clear();
        parse_opencode(&config, &Value::Null, &|_| None, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn jsonc_preserves_strings_and_rejects_broken_comments() {
        let text =
            r#"{/* comment */ "url":"http://localhost/v1", "key":"a\\\"//b", "items":[1,],}"#;
        let value: Value = serde_json::from_str(&strip_jsonc(text).unwrap()).unwrap();
        assert_eq!(value["url"], "http://localhost/v1");
        assert!(strip_jsonc("{} /* unfinished").is_err());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models.json");
        std::fs::write(&path, vec![b' '; MAX_BYTES as usize + 1]).unwrap();
        assert!(load(&path, false).is_err());
    }
}
