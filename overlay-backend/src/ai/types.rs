use serde::{Deserialize, Serialize};

/// Wire protocol used by a resolved AI endpoint. Existing bridge, local and
/// Hermes routes stay on OpenAI Chat Completions compatibility; direct cloud
/// providers use their native, documented APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiProtocol {
    OpenAiCompatible,
    OpenAiResponses,
    AnthropicMessages,
    /// Official Codex app-server account integration using the experimental,
    /// fail-closed no-tools permission contract.
    CodexSubscription,
}

impl AiProtocol {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::OpenAiCompatible => "openai-compatible",
            Self::OpenAiResponses => "openai-responses",
            Self::AnthropicMessages => "anthropic-messages",
            Self::CodexSubscription => "codex-subscription",
        }
    }

    #[must_use]
    pub const fn supports_model_listing(self) -> bool {
        matches!(
            self,
            Self::OpenAiCompatible | Self::OpenAiResponses | Self::CodexSubscription
        )
    }

    #[must_use]
    pub const fn supports_prompt_cache_control(self) -> bool {
        matches!(self, Self::OpenAiCompatible | Self::AnthropicMessages)
    }

    #[must_use]
    pub const fn supports_live_answers(self) -> bool {
        true
    }
}

/// Fully resolved target for one request. The credential is held only in
/// memory; direct-provider keys are loaded from Windows Credential Manager.
#[derive(Clone)]
pub struct AiEndpoint {
    pub protocol: AiProtocol,
    pub base_url: String,
    pub bearer: String,
    pub model: String,
    /// Optional reasoning effort for the official Codex app-server. Other
    /// protocols ignore it.
    pub reasoning_effort: Option<String>,
    pub is_local: bool,
}

impl AiEndpoint {
    #[must_use]
    pub const fn requires_bearer(&self) -> bool {
        !self.is_local && !matches!(self.protocol, AiProtocol::CodexSubscription)
    }

    #[must_use]
    pub const fn is_unmetered(&self) -> bool {
        self.is_local || matches!(self.protocol, AiProtocol::CodexSubscription)
    }

    #[must_use]
    pub const fn accepts_images(&self) -> bool {
        true
    }
}

impl std::fmt::Debug for AiEndpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AiEndpoint")
            .field("protocol", &self.protocol)
            .field("model", &self.model)
            .field("has_reasoning_effort", &self.reasoning_effort.is_some())
            .field("is_local", &self.is_local)
            .field("has_base_url", &!self.base_url.trim().is_empty())
            .field("has_credential", &!self.bearer.trim().is_empty())
            .finish()
    }
}

/// Frontend-visible event stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AiEvent {
    Start { id: String },
    Delta { text: String },
    Done { reason: String },
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: MessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String, // "data:image/jpeg;base64,..."
}
