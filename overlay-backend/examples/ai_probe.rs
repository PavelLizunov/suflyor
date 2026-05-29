//! Manual AI-request probe — fire a real request through the SAME path the app
//! uses (`ai::build_request` -> `ai::complete`) against whatever endpoint
//! `config.json` points at (local llama or cloud bridge), and print BOTH the
//! assembled system prompt (so you can see KB / RAG injection) and the model's
//! answer. A repeatable harness for checking hypotheses about prompts/answers.
//!
//! Run:
//!   cargo run -p overlay-backend --example ai_probe -- "Что такое Exasol?"
//!   cargo run -p overlay-backend --example ai_probe -- "что такое kubernetes"
//!
//! To A/B the KB grounding, ask about a term that IS a KB key (e.g. "Exasol",
//! "kubernetes") vs one that isn't — the printed system prompt shows whether a
//! "Справка из базы знаний" block was injected, and the answer shows the effect.

use overlay_backend::ai::{self, MessageContent};
use overlay_backend::config;

#[tokio::main]
async fn main() {
    let question = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    if question.trim().is_empty() {
        eprintln!("usage: ai_probe <question...>");
        std::process::exit(2);
    }

    let cfg = config::load();
    let ep = cfg.ai_endpoint(false);
    // Mirror the app: disable hidden "thinking" for local models (else a small
    // model can burn the whole token budget on reasoning and return empty).
    ai::set_local_no_think(ep.is_local && !cfg.ai_local_thinking);

    // Exact app path: build the request (incl. KB/RAG injection) then send it.
    let messages = ai::build_request("", &cfg.response_language, &[], None, Some(&question));

    println!("===== SYSTEM PROMPT (what the model receives) =====");
    if let Some(sys) = messages.iter().find(|m| m.role == "system") {
        match &sys.content {
            MessageContent::Text(t) => println!("{t}"),
            MessageContent::Parts(_) => println!("(multi-part system message)"),
        }
    }

    println!("\n===== ENDPOINT =====");
    println!(
        "provider: {}   model: {}   url: {}",
        if ep.is_local { "local" } else { "cloud" },
        ep.model,
        ep.base_url
    );

    println!("\n===== QUESTION =====\n{question}");

    println!("\n===== ANSWER =====");
    match ai::complete(&ep.base_url, &ep.bearer, &ep.model, messages, 400).await {
        Ok(answer) => println!("{answer}"),
        Err(e) => eprintln!("AI error: {e:#}"),
    }
}
