//! Curated personal memory (Phase 3b) — the LOGIC layer over the `persistence`
//! memory tables. The raw SQL/CRUD for `memory_candidates` / `memory_items`
//! lives on [`crate::persistence::Store`]; THIS module turns a session into
//! candidate suggestions (candidate EXTRACTION) and — later — assembles the
//! user's APPROVED memory into a bounded context for a new AI request
//! (`context_builder`).
//!
//! The pipeline (per docs/personal-memory-and-session-store-architecture.md,
//! Phase 3): session → [`extract_heuristic`] (free, local, deterministic) +,
//! on demand, an AI extractor (richer) → candidates → the user approves/rejects
//! → approved items → context_builder mixes ONLY approved memory into an ask.
//!
//! Extraction is HYBRID: the heuristic here always runs (no cost, no egress);
//! a separate AI extractor (a later commit) is an opt-in "deep extract".

mod candidates;
mod context_builder;
mod normalize;
mod summary_ref;

pub use candidates::extract_heuristic;
pub use context_builder::{context_for_meeting, format_memory_block, merge_context};
pub use normalize::{heuristic_clean, is_grounded};
pub use summary_ref::{
    format_summary_reference, key_terms, relevant_items, summary_reference_for_transcript,
};
