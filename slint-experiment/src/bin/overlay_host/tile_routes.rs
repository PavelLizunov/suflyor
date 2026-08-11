//! The per-tile ask ROUTE model carved out of `tile_ask.rs` (the `tile_ask`
//! split — see `docs/overlay-host-modular-structure-current.md`, "P1: разрезать
//! tile_ask.rs"). `AskRoute` (Text / Vision / Cloud) + its endpoint/token
//! resolution, the per-tile MUTABLE `LiveRoute` (sticky-cloud after 🧠), and
//! `live_route`. Pure config-driven routing — no UI, no network. Reached from
//! `tile_ask.rs` + the other tile modules through the `use tile_routes::*;`
//! re-export.
//!
//! NOTE (§7): only the crate-root symbols used are imported below; the `impl`
//! methods are `pub(crate)` (were private) so the ask entrypoints — now in a
//! SIBLING module — can still call `route.endpoint()` / `.max_tokens()`.
use super::{vision, Rc, AI_STREAM_MAX_TOKENS};

/// V0.8.0 (Поток D) — which AI endpoint an ask/follow-up/regenerate routes to.
/// Replaces the old `use_vision: bool` so the three routes are explicit and the
/// compiler enforces exhaustive handling (no silent bool transposition across
/// the ~9 call sites of the central ask fns).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AskRoute {
    /// Default text model (local or cloud per `ai_provider`).
    Text,
    /// Vision endpoint — the stored conversation carries the screenshot (F8).
    Vision,
    /// One-shot CLOUD escalation: the smart `prep_model` on the cloud bridge,
    /// IGNORING `ai_provider`. For a single hard question without flipping the
    /// persistent provider. Stronger reasoning, NOT live web.
    Cloud,
}

impl AskRoute {
    /// Every remote endpoint needs its resolved credential. Local endpoints may
    /// intentionally run without one.
    pub(crate) fn has_required_auth(self, c: &overlay_backend::config::Config) -> bool {
        let endpoint = self.endpoint(c);
        (!endpoint.requires_bearer() || !endpoint.bearer.trim().is_empty())
            && (!matches!(
                endpoint.protocol,
                overlay_backend::ai::AiProtocol::CodexSubscription
            ) || !endpoint.model.trim().is_empty())
    }

    /// Resolve the endpoint for this route from config.
    pub(crate) fn endpoint(
        self,
        c: &overlay_backend::config::Config,
    ) -> overlay_backend::config::AiEndpoint {
        match self {
            AskRoute::Text => c.ai_endpoint(false),
            AskRoute::Vision => c.vision_endpoint().unwrap_or_else(|| {
                // A Vision tile must never drift onto a text-only model when
                // its configured route becomes unavailable between turns.
                overlay_backend::config::AiEndpoint {
                    protocol: overlay_backend::ai::AiProtocol::OpenAiCompatible,
                    base_url: String::new(),
                    bearer: String::new(),
                    model: String::new(),
                    reasoning_effort: None,
                    is_local: false,
                }
            }),
            AskRoute::Cloud => c.ai_endpoint_cloud(),
        }
    }
    /// Max output tokens for this route (vision is capped tighter).
    pub(crate) fn max_tokens(self) -> u32 {
        match self {
            AskRoute::Vision => vision::VISION_MAX_TOKENS,
            AskRoute::Text | AskRoute::Cloud => AI_STREAM_MAX_TOKENS,
        }
    }
    /// True when the request carries a screenshot (journal flag).
    pub(crate) fn attaches_screenshot(self) -> bool {
        matches!(self, AskRoute::Vision)
    }
}

/// V0.8.1 — a per-tile MUTABLE route, shared by a tile's continuation surfaces
/// (text follow-up, 🔄 regenerate, 🎤 voice). They read it at CLICK time (not at
/// wire time), so when the 🧠 escalate button flips it to Cloud the rest of that
/// tile's conversation stays in the cloud — matching the sticky-cloud behaviour
/// Shift+F9 already has. UI-thread-only, so a Cell (no lock) is sufficient.
pub(crate) type LiveRoute = Rc<std::cell::Cell<AskRoute>>;

pub(crate) fn live_route(initial: AskRoute) -> LiveRoute {
    Rc::new(std::cell::Cell::new(initial))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::AskRoute;

    #[test]
    fn remote_routes_require_bearer_and_local_does_not() {
        let mut config = overlay_backend::config::Config {
            ai_bearer: " \t".into(),
            ..Default::default()
        };

        assert!(!AskRoute::Cloud.has_required_auth(&config));
        assert!(!AskRoute::Text.has_required_auth(&config));

        config.ai_bearer = "token".into();
        assert!(AskRoute::Cloud.has_required_auth(&config));
        assert!(AskRoute::Text.has_required_auth(&config));

        config.ai_provider = "local".into();
        config.ai_local_bearer.clear();
        assert!(AskRoute::Text.has_required_auth(&config));

        config.ai_provider = "codex".into();
        config.codex_model.clear();
        assert!(!AskRoute::Text.has_required_auth(&config));
        assert!(!AskRoute::Cloud.has_required_auth(&config));
        config.codex_model = "gpt-safe".into();
        assert!(AskRoute::Text.has_required_auth(&config));
        assert!(AskRoute::Cloud.has_required_auth(&config));
        for route in [AskRoute::Text, AskRoute::Cloud] {
            let endpoint = route.endpoint(&config);
            assert_eq!(
                endpoint.protocol,
                overlay_backend::ai::AiProtocol::CodexSubscription
            );
            assert_eq!(endpoint.model, "gpt-safe");
            assert!(endpoint.base_url.is_empty());
            assert!(endpoint.bearer.is_empty());
        }
    }

    #[test]
    fn missing_vision_route_never_falls_back_to_the_text_model() {
        let mut config = overlay_backend::config::Config::defaults();
        config.ai_provider = "codex".into();
        config.codex_model = "text-only".into();
        config.vision_provider = "same".into();

        let endpoint = AskRoute::Vision.endpoint(&config);
        assert!(endpoint.model.is_empty());
        assert!(!AskRoute::Vision.has_required_auth(&config));
    }
}
