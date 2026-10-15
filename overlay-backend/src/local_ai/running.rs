//! Model servers that already run on this PC, so Settings can connect to one
//! in one click: Ollama, LM Studio, Jan, KoboldCpp, a Strata the user started
//! and other OpenAI-compatible servers on their usual loopback ports.

use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

use super::{curl_success_body, LLAMA_PORT};

/// A server that answers `/v1/models` on a loopback port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunningServer {
    /// "Ollama", "LM Studio", "Strata"…, or empty when the port says nothing.
    pub name: String,
    pub port: u16,
    pub base_url: String,
    /// Chat models it serves; embedding models are left out.
    pub models: Vec<String>,
    /// Its `/health` says it is Strata with the model loaded.
    pub strata_ready: bool,
}

impl RunningServer {
    /// The local provider already points at this server and at a model it
    /// serves.
    #[must_use]
    pub fn is_current(&self, base_url: &str, model: &str) -> bool {
        super::is_loopback_v1_on_port(base_url, &self.port.to_string())
            && self.models.iter().any(|m| m == model)
    }

    /// The model to use: the saved one when this server has it, else its first.
    #[must_use]
    pub fn pick_model(&self, saved: &str) -> String {
        self.models
            .iter()
            .find(|m| m.as_str() == saved)
            .or_else(|| self.models.first())
            .cloned()
            .unwrap_or_default()
    }
}

/// The usual ports of local model servers. :8080 is the built-in engine's
/// port and is not listed.
const KNOWN_PORTS: &[(u16, &str)] = &[
    (11434, "Ollama"),
    (1234, "LM Studio"),
    (1337, "Jan"),
    (5001, "KoboldCpp"),
    (4891, "GPT4All"),
    (5000, "text-generation-webui"),
    (8000, ""),
    (8090, ""),
];

fn chat_models(body: &str) -> Vec<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|json| json.get("data")?.as_array().cloned())
        .into_iter()
        .flatten()
        .filter_map(|m| m.get("id")?.as_str().map(str::to_owned))
        .filter(|id| !id.trim().is_empty() && !id.to_ascii_lowercase().contains("embed"))
        .collect()
}

fn probe(port: u16, name: &str) -> Option<RunningServer> {
    // A closed port refuses at once; curl only runs for an open one.
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).ok()?;
    let base_url = format!("http://127.0.0.1:{port}/v1");
    let body = curl_success_body(&[
        "-f",
        "-sS",
        "--max-time",
        "2",
        &format!("{base_url}/models"),
    ])?;
    let models = chat_models(&body);
    if models.is_empty() {
        return None;
    }
    // Strata's /health carries `loaded`, whatever port it runs on.
    let health = curl_success_body(&[
        "-f",
        "-sS",
        "--max-time",
        "2",
        &format!("http://127.0.0.1:{port}/health"),
    ])
    .and_then(|b| serde_json::from_str::<serde_json::Value>(&b).ok());
    let loaded = health
        .as_ref()
        .and_then(|h| h.get("loaded"))
        .map(|l| l.as_bool() == Some(true));
    // :8090 is the Strata engine's port: the app treats it as Strata, so a
    // server there is listed only when it is a Strata with its model loaded.
    if port == 8090 && loaded != Some(true) {
        return None;
    }
    Some(RunningServer {
        name: if loaded.is_some() { "Strata" } else { name }.to_string(),
        port,
        base_url,
        models,
        strata_ready: loaded == Some(true),
    })
}

/// Servers answering on the usual ports, probed in parallel (about two
/// seconds at most). Callers keep it off the UI thread.
#[must_use]
pub fn find_running_servers() -> Vec<RunningServer> {
    let handles: Vec<_> = KNOWN_PORTS
        .iter()
        .filter(|(port, _)| port.to_string() != LLAMA_PORT)
        .map(|&(port, name)| std::thread::spawn(move || probe(port, name)))
        .collect();
    handles
        .into_iter()
        .filter_map(|h| h.join().ok().flatten())
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn chat_models_skip_embeddings_and_blanks() {
        let body = r#"{"data":[{"id":"qwen3:8b"},{"id":"nomic-embed-text:latest"},{"id":""},{"id":"llama3.2"}]}"#;
        assert_eq!(chat_models(body), vec!["qwen3:8b", "llama3.2"]);
        assert!(chat_models("not json").is_empty());
        assert!(chat_models(r#"{"data":[]}"#).is_empty());
    }

    #[test]
    fn current_server_and_model_choice() {
        let server = RunningServer {
            name: "Ollama".into(),
            port: 11434,
            base_url: "http://127.0.0.1:11434/v1".into(),
            models: vec!["qwen3:8b".into(), "llama3.2".into()],
            strata_ready: false,
        };
        assert!(server.is_current("http://localhost:11434/v1/", "llama3.2"));
        // The saved model is gone from the server: not in use.
        assert!(!server.is_current("http://127.0.0.1:11434/v1", "gemma"));
        assert!(!server.is_current("http://127.0.0.1:8080/v1", "llama3.2"));
        assert_eq!(server.pick_model("llama3.2"), "llama3.2");
        assert_eq!(server.pick_model("gemma"), "qwen3:8b");
    }

    #[test]
    fn the_built_in_engine_port_is_not_probed() {
        assert!(KNOWN_PORTS
            .iter()
            .all(|(port, _)| port.to_string() != LLAMA_PORT));
    }

    #[test]
    fn a_server_with_models_is_found() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming().take(3) {
                let mut stream = stream.unwrap();
                let mut buf = [0u8; 1024];
                let n = stream.read(&mut buf).unwrap();
                let req = String::from_utf8_lossy(&buf[..n]);
                let body = if req.starts_with("GET /v1/models") {
                    r#"{"data":[{"id":"qwen3:8b"}]}"#
                } else {
                    r#"{"status":"ok"}"#
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        let found = probe(port, "Ollama").unwrap();
        assert_eq!(found.name, "Ollama");
        assert_eq!(found.base_url, format!("http://127.0.0.1:{port}/v1"));
        assert_eq!(found.models, vec!["qwen3:8b"]);
        assert!(!found.strata_ready);
    }
}
