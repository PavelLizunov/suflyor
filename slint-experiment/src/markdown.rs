//! Markdown adapter — pulldown-cmark events → block list.
//!
//! Extracted from Phase 0.5 spike 3 (`bin/markdown_spike.rs`) into a
//! reusable lib module. Returns a `Vec<MarkdownBlock>` shaped as
//! `{ kind: i32, text: String, lang: String }` per row — matches the
//! Slint-side struct of the same name. The binary calls this and pipes
//! the result into a `ModelRc<MarkdownBlock>`.
//!
//! Phase 4 scope: H1-H3, paragraphs, bullet lists, code blocks (no
//! syntect colors), horizontal rules. Inline emphasis renders as
//! plaintext (bold/italic dropped; inline code wrapped in literal
//! backticks). Tables, links, images, footnotes, HTML are silently
//! dropped — Phase 4.x follow-up.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};

/// Block discriminant values — keep in sync with
/// `ui/markdown_spike.slint` and `ui/tile.slint`.
pub mod kind {
    pub const PARAGRAPH: i32 = 0;
    pub const H1: i32 = 1;
    pub const H2: i32 = 2;
    pub const H3: i32 = 3;
    pub const BULLET: i32 = 4;
    pub const CODE: i32 = 5;
    pub const HR: i32 = 6;
}

/// Plain-Rust block record. Binaries map this to whatever Slint
/// MarkdownBlock struct they include via `include_modules!()`.
#[derive(Debug, Clone)]
pub struct Block {
    pub kind: i32,
    pub text: String,
    pub lang: String,
}

impl Block {
    fn new(kind: i32, text: String, lang: String) -> Self {
        Self { kind, text, lang }
    }
}

/// Parse a CommonMark source string into a `Vec<Block>`.
#[must_use]
pub fn parse(source: &str) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
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
                    _ => kind::H3,
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
                // Inside a list item — text goes into the current bullet.
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
                if list_depth > 1 {
                    current_text.push_str(&"  ".repeat(list_depth - 1));
                }
            }
            Event::Text(t) => {
                current_text.push_str(&t);
            }
            Event::Code(t) => {
                current_text.push('`');
                current_text.push_str(&t);
                current_text.push('`');
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
                out.push(Block::new(kind::HR, String::new(), String::new()));
            }
            _ => {}
        }
    }
    flush(
        &mut out,
        &mut current_text,
        &mut current_kind,
        &mut current_lang,
    );
    out
}

fn flush(out: &mut Vec<Block>, text: &mut String, kind_slot: &mut Option<i32>, lang: &mut String) {
    if let Some(k) = kind_slot.take() {
        if !text.is_empty() || k == kind::HR {
            out.push(Block::new(k, std::mem::take(text), std::mem::take(lang)));
        } else {
            text.clear();
            lang.clear();
        }
    }
    text.clear();
    lang.clear();
}

/// Sample tile markdown text — used by Phase 4's overlay-host stub
/// before the AI / knowledge-base backend wiring lands.
#[must_use]
pub fn sample_tile_markdown(sequence: u32) -> String {
    let template = r##"# Tile #{N} — Sample answer

This tile demonstrates the **Phase 4** markdown body adapter integrated into the tile window. The Rust side parses CommonMark via `pulldown-cmark` and emits `Vec<Block>` rows that Slint renders with kind-discriminant styling.

## What's working

- Headings (H1, H2, H3 — visible above and below)
- Paragraphs with **bold** (rendered plaintext for now) and `inline code` (wrapped in backticks)
- Bullet lists like this one
- Fenced code blocks

## Sample code

```rust
fn main() {
    println!("Hello from tile #{N}");
}
```

## Pending Phase 4.x work

- `syntect` colors on code blocks
- `StyledText` runs for proper **bold** / *italic* / `inline-code`
- Tables (GridLayout)
- Links (TouchArea + open_url)
- Images (HTTP fetch + cache)
"##;
    template.replace("{N}", &sequence.to_string())
}
