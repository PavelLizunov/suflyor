//! AI provider client for legacy OpenAI-compatible endpoints plus native
//! OpenAI Responses and Anthropic Messages APIs. Emits AiEvent chunks downstream.

pub mod completion;
pub(crate) mod control;
pub mod inspect;
pub mod pricing;
pub mod prompt;
pub(crate) mod provider;
pub mod stream;
pub mod tps;
pub mod types;

#[cfg(test)]
mod tests;

pub use self::completion::*;
pub use self::control::{set_local_no_think, set_prompt_cache};
pub use self::inspect::*;
pub use self::pricing::*;
pub use self::prompt::*;
pub use self::stream::*;
pub use self::tps::{avg_tps, clear_request_perf, latest_request_perf, record_tps, RequestPerf};
pub use self::types::*;
