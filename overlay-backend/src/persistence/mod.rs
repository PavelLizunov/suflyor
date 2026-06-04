//! Session catalog persistence — a REBUILDABLE SQLite index over the append-only
//! JSONL journals ("Phase 2 — SQLite catalog" of
//! `docs/personal-memory-and-session-store-architecture.md`).
//!
//! Two layers, by design:
//! - JSONL ([`crate::journal`]) stays the primary append-only event log — cheap,
//!   crash-proof, human-readable, the source of truth.
//! - SQLite ([`Store`]) is a queryable PROJECTION for the session archive +
//!   search. It can be deleted and rebuilt from the journals with no data loss,
//!   so the live audio / AI pipeline never depends on its speed.
//!
//! Callers see only owned row types ([`Session`] / [`Utterance`] / [`AiTurn`])
//! and [`Store`] — no `rusqlite` types or raw SQL leak out. The JSONL→SQLite
//! indexer + FTS search land in follow-up commits.

mod migrations;
pub mod models;
mod sqlite_store;

pub use models::{AiTurn, Session, Utterance};
pub use sqlite_store::Store;
