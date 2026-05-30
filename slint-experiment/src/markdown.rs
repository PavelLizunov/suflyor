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
//! backticks). GFM tables render as an aligned monospace block (#109);
//! links, images, footnotes, HTML are silently dropped — Phase 4.x.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

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
    /// GFM table rendered as an aligned monospace block (#109).
    pub const TABLE: i32 = 7;
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
    // #109 — GFM table accumulation. pulldown-cmark only emits table
    // events when ENABLE_TABLES is set; otherwise `| a | b |` arrives as
    // plain paragraph text (the old "tables silently dropped" behaviour
    // that overlapped in the tile). Cells are collected per row, then
    // rendered to an aligned monospace block on End(Table).
    let mut in_cell = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();
    // audit/#134 — preserve link destinations. The link TEXT already flows
    // through as Event::Text; only the URL was dropped (the catch-all arm
    // swallowed Tag::Link), so AI answers lost every link. Stash on Start(Link),
    // append " (url)" on End(Link).
    let mut link_url: Option<String> = None;

    for event in Parser::new_ext(source, Options::ENABLE_TABLES) {
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
                if in_cell {
                    current_cell.push_str(&t);
                } else {
                    current_text.push_str(&t);
                }
            }
            Event::Code(t) => {
                let buf = if in_cell {
                    &mut current_cell
                } else {
                    &mut current_text
                };
                buf.push('`');
                buf.push_str(&t);
                buf.push('`');
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_cell {
                    current_cell.push(' ');
                } else {
                    current_text.push(' ');
                }
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
            Event::Start(Tag::Table(_)) => {
                flush(
                    &mut out,
                    &mut current_text,
                    &mut current_kind,
                    &mut current_lang,
                );
                table_rows.clear();
                current_row.clear();
            }
            Event::Start(Tag::TableHead | Tag::TableRow) => {
                current_row.clear();
            }
            Event::Start(Tag::TableCell) => {
                current_cell.clear();
                in_cell = true;
            }
            Event::End(TagEnd::TableCell) => {
                current_row.push(std::mem::take(&mut current_cell));
                in_cell = false;
            }
            Event::End(TagEnd::TableHead | TagEnd::TableRow) => {
                table_rows.push(std::mem::take(&mut current_row));
            }
            Event::End(TagEnd::Table) => {
                if !table_rows.is_empty() {
                    out.push(Block::new(
                        kind::TABLE,
                        format_table(&table_rows),
                        String::new(),
                    ));
                }
                table_rows.clear();
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
            Event::Start(Tag::Link { dest_url, .. }) => {
                link_url = Some(dest_url.to_string());
            }
            Event::End(TagEnd::Link) => {
                if let Some(url) = link_url.take() {
                    let buf = if in_cell {
                        &mut current_cell
                    } else {
                        &mut current_text
                    };
                    // Skip autolinks (text already == url) to avoid "x (x)".
                    if !url.is_empty() && !buf.ends_with(url.as_str()) {
                        buf.push_str(" (");
                        buf.push_str(&url);
                        buf.push(')');
                    }
                }
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

/// Render parsed table rows (rows\[0\] = header) into an aligned monospace
/// block. GFM tables used to fall through pulldown-cmark as raw `|`
/// paragraph text that overlapped in the tile (#109). Each column is
/// padded to its (capped) max width and separated with box-drawing
/// glyphs; the tile renders this with `wrap: no-wrap` so the alignment
/// holds. Over-long cells are truncated with `…` to bound the width.
fn format_table(rows: &[Vec<String>]) -> String {
    /// Per-column character cap so one verbose cell can't blow the width.
    const MAX_COL: usize = 28;
    let ncols = rows.iter().map(Vec::len).max().unwrap_or(0);
    if ncols == 0 {
        return String::new();
    }
    let mut widths = vec![0_usize; ncols];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            let w = cell.trim().chars().count().min(MAX_COL);
            if w > widths[i] {
                widths[i] = w;
            }
        }
    }
    let mut lines: Vec<String> = Vec::with_capacity(rows.len() + 1);
    for (ri, row) in rows.iter().enumerate() {
        let cells: Vec<String> = widths
            .iter()
            .enumerate()
            .map(|(i, width)| {
                let cell = truncate_cell(row.get(i).map_or("", |s| s.trim()), MAX_COL);
                let pad = width.saturating_sub(cell.chars().count());
                format!("{cell}{}", " ".repeat(pad))
            })
            .collect();
        lines.push(cells.join(" │ ").trim_end().to_string());
        if ri == 0 {
            let sep: Vec<String> = widths.iter().map(|w| "─".repeat(*w)).collect();
            lines.push(sep.join("─┼─"));
        }
    }
    lines.join("\n")
}

/// Truncate a cell to `max` chars, appending `…` when cut.
fn truncate_cell(cell: &str, max: usize) -> String {
    if cell.chars().count() <= max {
        cell.to_string()
    } else {
        let mut s: String = cell.chars().take(max.saturating_sub(1)).collect();
        s.push('…');
        s
    }
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn link_url_is_preserved_in_text() {
        // The link TEXT already survived (inner Event::Text); the URL was being
        // dropped by the catch-all arm. Now it's appended as " (url)".
        let blocks = parse("See the [docs](https://example.com/guide) for more.");
        let para = blocks.iter().find(|b| b.kind == kind::PARAGRAPH).unwrap();
        assert!(
            para.text.contains("docs"),
            "link text kept: {:?}",
            para.text
        );
        assert!(
            para.text.contains("https://example.com/guide"),
            "link URL preserved: {:?}",
            para.text
        );
    }

    #[test]
    fn autolink_url_not_duplicated() {
        // text already == url → must not render "url (url)".
        let blocks = parse("[https://example.com](https://example.com)");
        let para = blocks.iter().find(|b| b.kind == kind::PARAGRAPH).unwrap();
        assert_eq!(
            para.text.matches("https://example.com").count(),
            1,
            "autolink must not duplicate the URL: {:?}",
            para.text
        );
    }

    #[test]
    fn gfm_table_parses_to_single_aligned_table_block() {
        let md = "\
| A | B |
|---|---|
| 1 | 22 |
| 333 | 4 |
";
        let blocks = parse(md);
        let tables: Vec<&Block> = blocks.iter().filter(|b| b.kind == kind::TABLE).collect();
        assert_eq!(tables.len(), 1, "exactly one TABLE block expected");
        let t = &tables[0].text;
        // Header + body cells survive.
        assert!(t.contains('A') && t.contains('B') && t.contains("333"));
        // Box-drawing column separator + header underline are present.
        assert!(t.contains('│'), "column separator missing: {t:?}");
        assert!(t.contains('─'), "header underline missing: {t:?}");
        // The raw GFM dashes separator row must NOT leak as content.
        assert!(!t.contains("---"), "raw pipe separator leaked: {t:?}");
        // Column A is padded so every data line starts at the same width
        // ("333" is the widest → width 3): the "1" cell becomes "1  ".
        assert!(t.contains("1  "), "column A not padded to width 3: {t:?}");
    }

    #[test]
    fn table_cells_do_not_bleed_into_surrounding_paragraphs() {
        let md = "before\n\n| X | Y |\n|---|---|\n| a | b |\n\nafter";
        let blocks = parse(md);
        assert!(
            blocks
                .iter()
                .any(|b| b.kind == kind::PARAGRAPH && b.text == "before"),
            "leading paragraph lost"
        );
        assert!(
            blocks.iter().any(|b| b.kind == kind::TABLE),
            "table not detected"
        );
        assert!(
            blocks
                .iter()
                .any(|b| b.kind == kind::PARAGRAPH && b.text == "after"),
            "trailing paragraph lost or merged into the table"
        );
    }

    #[test]
    fn over_long_table_cell_is_truncated_with_ellipsis() {
        let long = "x".repeat(60);
        let md = format!("| H |\n|---|\n| {long} |\n");
        let blocks = parse(&md);
        let t = &blocks
            .iter()
            .find(|b| b.kind == kind::TABLE)
            .expect("table block")
            .text;
        assert!(t.contains('…'), "long cell should be truncated: {t:?}");
        // No single line should exceed the cap by much (28 + separators).
        assert!(
            t.lines().all(|l| l.chars().count() <= 40),
            "line exceeded width cap: {t:?}"
        );
    }
}
