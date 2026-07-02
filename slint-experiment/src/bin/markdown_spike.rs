//! Phase 0.5 spike 3 — markdown adapter.
//!
//! Validates the migration plan's #1 risk: building a
//! `pulldown-cmark → Slint Text/StyledText` walker in Rust. This spike
//! covers the bulk of CommonMark features the tile-content surface
//! uses (headings 1-3, paragraphs, bullet lists, fenced code blocks,
//! horizontal rules). Inline emphasis (bold/italic/inline-code) is
//! intentionally NOT exercised — that requires Slint's StyledText
//! widget with per-run formatting, which is Phase 4 proper work.
//!
//! syntect (code-block syntax highlighting) is also deferred to
//! Phase 4; this spike renders code blocks as monospace plaintext.
//!
//! Reads the first ~80 lines of src-tauri/knowledge/glossary.md
//! (a realistic input sample) + parses + populates a MarkdownSpike
//! window. The window auto-closes after 8 s so smoke scripts can
//! screenshot it.
//!
//! Run from slint-experiment/:
//!   cargo run --bin markdown-spike

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::time::Duration;

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::pedantic,
    clippy::nursery,
    clippy::all
)]
mod ui {
    slint::include_modules!();
}

use ui::{MarkdownBlock, MarkdownSpike};

/// Block discriminant — keep in sync with markdown_spike.slint comment.
mod kind {
    pub const PARAGRAPH: i32 = 0;
    pub const H1: i32 = 1;
    pub const H2: i32 = 2;
    pub const H3: i32 = 3;
    pub const BULLET: i32 = 4;
    pub const CODE: i32 = 5;
    pub const HR: i32 = 6;
}

fn block(kind: i32, text: String, lang: String) -> MarkdownBlock {
    MarkdownBlock {
        kind,
        text: SharedString::from(text),
        lang: SharedString::from(lang),
    }
}

/// Convert a CommonMark source string to a Vec<MarkdownBlock>.
///
/// Walks pulldown_cmark events. Accumulates text within each block
/// until the matching End event, then emits one MarkdownBlock per
/// top-level construct. Nested lists are flattened to bullets with
/// a leading "  " indent per level for the spike (Phase 4 would
/// emit a proper hierarchy + custom layout).
fn parse_markdown(source: &str) -> Vec<MarkdownBlock> {
    let mut out: Vec<MarkdownBlock> = Vec::new();
    let mut current_text = String::new();
    let mut current_kind: Option<i32> = None;
    let mut current_lang = String::new();
    let mut list_depth: usize = 0;

    for event in Parser::new(source) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                flush(
                    &mut out,
                    &mut current_text,
                    &mut current_kind,
                    &mut current_lang,
                );
                current_kind = Some(match level {
                    HeadingLevel::H1 => kind::H1,
                    HeadingLevel::H2 => kind::H2,
                    HeadingLevel::H3 => kind::H3,
                    _ => kind::H3, // fold H4-H6 into H3 for the spike
                });
            }
            Event::Start(Tag::Paragraph) if list_depth == 0 => {
                flush(
                    &mut out,
                    &mut current_text,
                    &mut current_kind,
                    &mut current_lang,
                );
                current_kind = Some(kind::PARAGRAPH);
            }
            Event::Start(Tag::Paragraph) => {
                // Inside a list item — the paragraph is part of the
                // bullet; don't start a new block.
            }
            Event::Start(Tag::CodeBlock(cb)) => {
                flush(
                    &mut out,
                    &mut current_text,
                    &mut current_kind,
                    &mut current_lang,
                );
                current_kind = Some(kind::CODE);
                current_lang = match cb {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
            }
            Event::Start(Tag::List(_)) => {
                list_depth += 1;
            }
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
            }
            Event::Start(Tag::Item) => {
                flush(
                    &mut out,
                    &mut current_text,
                    &mut current_kind,
                    &mut current_lang,
                );
                current_kind = Some(kind::BULLET);
                // Indent bullets by 2 spaces per nesting level beyond 1.
                if list_depth > 1 {
                    current_text.push_str(&"  ".repeat(list_depth - 1));
                }
            }
            Event::Text(t) => {
                current_text.push_str(&t);
            }
            Event::Code(t) => {
                // B-inline (ТЗ 2026-07-02): inline code renders plain (backticks
                // stripped) — kept in sync with markdown.rs::parse.
                current_text.push_str(&t);
            }
            Event::SoftBreak | Event::HardBreak => {
                current_text.push(' ');
            }
            Event::End(TagEnd::Heading(_))
            | Event::End(TagEnd::Paragraph)
            | Event::End(TagEnd::Item)
            | Event::End(TagEnd::CodeBlock) => {
                flush(
                    &mut out,
                    &mut current_text,
                    &mut current_kind,
                    &mut current_lang,
                );
            }
            Event::Rule => {
                flush(
                    &mut out,
                    &mut current_text,
                    &mut current_kind,
                    &mut current_lang,
                );
                out.push(block(kind::HR, String::new(), String::new()));
            }
            _ => {
                // Tables, footnotes, images, html, links — Phase 4 work.
                // Spike silently ignores them.
            }
        }
    }
    // Flush any trailing block.
    flush(
        &mut out,
        &mut current_text,
        &mut current_kind,
        &mut current_lang,
    );
    out
}

fn flush(
    out: &mut Vec<MarkdownBlock>,
    text: &mut String,
    kind: &mut Option<i32>,
    lang: &mut String,
) {
    if let Some(k) = kind.take() {
        if !text.is_empty() || k == self::kind::HR {
            out.push(block(k, std::mem::take(text), std::mem::take(lang)));
        } else {
            text.clear();
            lang.clear();
        }
    }
    text.clear();
    lang.clear();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Locate the canonical glossary input. The repo root is the
    // current dir's parent when running via `cargo run` from
    // slint-experiment/, so try both relative paths.
    let candidates = [
        "../overlay-backend/knowledge/glossary.md",
        "overlay-backend/knowledge/glossary.md",
    ];
    let (path, raw) = candidates
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok().map(|s| (*p, s)))
        .ok_or("glossary.md not found in either candidate path")?;

    // Take only the first ~80 lines so the spike window isn't endless.
    let excerpt: String = raw.lines().take(80).collect::<Vec<_>>().join("\n");
    let blocks = parse_markdown(&excerpt);
    eprintln!(
        "[markdown-spike] parsed {} blocks from {} (excerpt of {} chars)",
        blocks.len(),
        path,
        excerpt.len()
    );
    for (i, b) in blocks.iter().enumerate().take(8) {
        eprintln!(
            "[markdown-spike]   block {i}: kind={} text={:?}",
            b.kind,
            b.text.chars().take(60).collect::<String>()
        );
    }

    let window = MarkdownSpike::new()?;
    window.set_blocks(ModelRc::new(VecModel::from(blocks)));
    window.set_source_name(SharedString::from(path));

    // Auto-close after 8 s for the smoke script.
    let weak = window.as_weak();
    slint::Timer::single_shot(Duration::from_secs(8), move || {
        if let Some(w) = weak.upgrade() {
            eprintln!("[markdown-spike] 8 s elapsed, hiding window.");
            let _ = w.hide();
        }
    });

    window.run()?;
    Ok(())
}
