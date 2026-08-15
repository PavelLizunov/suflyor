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
//! plaintext (bold/italic dropped; inline code rendered PLAIN — backticks
//! stripped, see Event::Code). GFM tables render as an aligned monospace block (#109);
//! links, images, footnotes, HTML are silently dropped — Phase 4.x.

use crate::math_display::{looks_like_delimited_math, normalize_math_fragment};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::borrow::Cow;

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
    pub display_text: String,
    pub lang: String,
}

impl Block {
    fn new(kind: i32, text: String, lang: String) -> Self {
        let display_text = text.clone();
        Self {
            kind,
            text,
            display_text,
            lang,
        }
    }
}

/// Parse a CommonMark source string into a `Vec<Block>`.
#[must_use]
pub fn parse(source: &str) -> Vec<Block> {
    let source = canonicalize_tex_math_delimiters(source);
    let source = source.as_ref();
    let mut raw = parse_variant(source, false);
    let display = parse_variant(source, true);
    for (block, shown) in raw.iter_mut().zip(display) {
        if block.kind == shown.kind {
            block.display_text = shown.text;
        }
    }
    raw
}

/// pulldown-cmark intentionally recognizes only `$...$` and `$$...$$` math.
/// AI providers also commonly emit TeX's `\[...\]` and `\(...\)` delimiters.
/// Convert paired delimiters before Markdown sees them so formulas become math
/// events and a standalone `=` cannot become a Setext heading. Fenced and
/// inline code stay exact.
fn canonicalize_tex_math_delimiters(source: &str) -> Cow<'_, str> {
    let has_display = source.contains("\\[") && source.contains("\\]");
    let has_inline = source.contains("\\(") && source.contains("\\)");
    if !has_display && !has_inline {
        return Cow::Borrowed(source);
    }

    let mut out = String::with_capacity(source.len());
    let mut fence: Option<(u8, usize)> = None;
    let mut inline_ticks = 0_usize;
    let mut math_close: Option<&'static str> = None;
    let mut changed = false;

    for line in source.split_inclusive('\n') {
        let trimmed = line.trim_start_matches([' ', '\t']);
        if inline_ticks == 0 && math_close.is_none() {
            if let Some((marker, count)) = fence_marker(trimmed) {
                match fence {
                    Some((active, minimum)) if marker == active && count >= minimum => {
                        out.push_str(line);
                        fence = None;
                        continue;
                    }
                    None => {
                        out.push_str(line);
                        fence = Some((marker, count));
                        continue;
                    }
                    Some(_) => {}
                }
            }
        }
        if fence.is_some() {
            out.push_str(line);
            continue;
        }

        let bytes = line.as_bytes();
        let mut index = 0_usize;
        while index < line.len() {
            if bytes[index] == b'`' {
                let start = index;
                while index < line.len() && bytes[index] == b'`' {
                    index += 1;
                }
                let run = index - start;
                inline_ticks = if inline_ticks == 0 {
                    run
                } else if inline_ticks == run {
                    0
                } else {
                    inline_ticks
                };
                out.push_str(&line[start..index]);
                continue;
            }
            if inline_ticks == 0 {
                if let Some(close) = math_close {
                    if line[index..].starts_with(close) {
                        out.push_str(if close == "\\]" { "$$" } else { "$" });
                        index += 2;
                        math_close = None;
                        continue;
                    }
                } else if line[index..].starts_with("\\[") {
                    out.push_str("$$");
                    index += 2;
                    math_close = Some("\\]");
                    changed = true;
                    continue;
                } else if line[index..].starts_with("\\(") {
                    out.push('$');
                    index += 2;
                    math_close = Some("\\)");
                    changed = true;
                    continue;
                }
            }
            let Some(ch) = line[index..].chars().next() else {
                break;
            };
            // pulldown-cmark's math span cannot cross a physical newline.
            // TeX display whitespace is insignificant here (matrix rows use
            // explicit `\\\\`), so keep the whole paired display in one span.
            if math_close.is_some() && matches!(ch, '\r' | '\n') {
                out.push(' ');
            } else {
                out.push(ch);
            }
            index += ch.len_utf8();
        }
    }

    if changed && math_close.is_none() {
        Cow::Owned(out)
    } else {
        // Never reinterpret a malformed, unfinished display fragment.
        Cow::Borrowed(source)
    }
}

fn fence_marker(line: &str) -> Option<(u8, usize)> {
    let marker = *line.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let count = line
        .as_bytes()
        .iter()
        .take_while(|byte| **byte == marker)
        .count();
    (count >= 3).then_some((marker, count))
}

fn parse_variant(source: &str, normalize_math: bool) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    let mut current_text = String::new();
    let mut current_kind: Option<i32> = None;
    let mut current_lang = String::new();
    let mut in_math_fence = false;
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

    let mut options = Options::ENABLE_TABLES;
    options.insert(Options::ENABLE_MATH);
    for event in Parser::new_ext(source, options) {
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
                let lang = match cb {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                in_math_fence = matches!(
                    lang.trim().to_ascii_lowercase().as_str(),
                    "math" | "latex" | "tex"
                );
                current_kind = Some(if in_math_fence {
                    kind::PARAGRAPH
                } else {
                    kind::CODE
                });
                current_lang = if in_math_fence { String::new() } else { lang };
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
                } else if in_math_fence && normalize_math {
                    current_text.push_str(&normalize_math_fragment(&t));
                } else {
                    current_text.push_str(&t);
                }
            }
            Event::Code(t) => {
                // B-inline (ТЗ 2026-07-02): inline code renders as PLAIN text — drop
                // the literal backticks the tester saw around `journalctl`. Slint's
                // flat Text has neither styled runs nor inline flow across multi-style
                // runs, so true monospace-inline would need a custom text-layout engine
                // (out of scope). Whole-answer copy is unaffected — it copies the raw
                // markdown, not these blocks.
                // ponytail: inline code renders plain; upgrade path = styled runs.
                let buf = if in_cell {
                    &mut current_cell
                } else {
                    &mut current_text
                };
                buf.push_str(&t);
            }
            Event::InlineMath(t) => {
                let buf = if in_cell {
                    &mut current_cell
                } else {
                    &mut current_text
                };
                if normalize_math && looks_like_delimited_math(&t) {
                    buf.push_str(&normalize_math_fragment(&t));
                } else {
                    buf.push('$');
                    buf.push_str(&t);
                    buf.push('$');
                }
            }
            Event::DisplayMath(t) => {
                let buf = if in_cell {
                    &mut current_cell
                } else {
                    &mut current_text
                };
                if normalize_math && looks_like_delimited_math(&t) {
                    buf.push_str(&normalize_math_fragment(&t));
                } else {
                    buf.push_str("$$");
                    buf.push_str(&t);
                    buf.push_str("$$");
                }
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
            | Event::End(TagEnd::Item) => {
                flush(
                    &mut out,
                    &mut current_text,
                    &mut current_kind,
                    &mut current_lang,
                );
            }
            Event::End(TagEnd::CodeBlock) => {
                flush(
                    &mut out,
                    &mut current_text,
                    &mut current_kind,
                    &mut current_lang,
                );
                in_math_fence = false;
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
- Paragraphs with **bold** (rendered plaintext for now) and `inline code` (backticks stripped)
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

    #[test]
    fn math_display_handles_display_math_but_preserves_code_urls_and_currency() {
        let blocks = parse(
            "Math $$c_{ij} = \\sum_{k=1}^{n} a_{ik}b_{kj}$$. `c_{ij}`. \
             https://example.test/c_{ij}?q=sum_k=1 costs $100 = 90$.\n\n\
             ```text\nc_{ij} = \\sum_{k=1}^{n}\n```",
        );
        let math = blocks
            .iter()
            .find(|block| block.display_text.contains("cᵢⱼ = ∑ₖ₌₁ⁿ aᵢₖbₖⱼ"))
            .expect("math paragraph");
        assert!(math
            .text
            .contains("$$c_{ij} = \\sum_{k=1}^{n} a_{ik}b_{kj}$$"));
        assert!(blocks
            .iter()
            .any(|block| block.text.contains(". c_{ij}. https://")));
        assert!(blocks.iter().any(|block| {
            block
                .display_text
                .contains("https://example.test/c_{ij}?q=sum_k=1")
                && block.display_text.contains("$100 = 90$")
        }));
        assert!(blocks.iter().any(|block| {
            block.kind == kind::CODE
                && block.text.trim_end_matches('\n') == "c_{ij} = \\sum_{k=1}^{n}"
                && block.display_text == block.text
        }));
    }

    #[test]
    fn math_display_normalizes_single_letter_inline_math() {
        let blocks = parse("Элемент из $i$-й строки и $j$-го столбца");
        assert_eq!(
            blocks[0].display_text,
            "Элемент из i-й строки и j-го столбца"
        );
        assert_eq!(blocks[0].text, "Элемент из $i$-й строки и $j$-го столбца");
    }

    #[test]
    fn fenced_math_is_displayed_as_math_instead_of_a_code_box() {
        let blocks = parse(
            "```math\nA = \\begin{pmatrix}\na_{11} & a_{12} \\\\\na_{21} & a_{22}\n\\end{pmatrix}\n```",
        );
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].kind, kind::PARAGRAPH);
        assert!(blocks[0].text.contains("\\begin{pmatrix}"));
        assert!(!blocks[0].display_text.contains("\\begin"));
        assert!(blocks[0].display_text.contains("a₁₁  a₁₂"));
        assert!(blocks[0].display_text.contains("a₂₁  a₂₂"));
        assert!(blocks[0].display_text.contains('\n'));
    }

    #[test]
    fn tex_bracket_display_is_math_before_markdown_heading_rules() {
        let blocks = parse(
            r#"## Matrix addition

\[
\begin{pmatrix} 1 & 2 \\ 3 & 4 \end{pmatrix}
+
\begin{pmatrix} 5 & 6 \\ 7 & 8 \end{pmatrix}
=
\begin{pmatrix} 6 & 8 \\ 10 & 12 \end{pmatrix}
\]
"#,
        );

        assert!(
            blocks.iter().all(|block| block.kind != kind::H1),
            "the '=' inside display math must not become a Setext heading: {blocks:?}"
        );
        let formula = blocks
            .iter()
            .find(|block| block.display_text.contains("10  12"))
            .expect("matrix display block");
        assert_eq!(formula.kind, kind::PARAGRAPH);
        assert_eq!(formula.display_text.matches('(').count(), 3);
        assert!(!formula.display_text.contains("\\begin"));
        assert!(!formula.display_text.contains("\\end"));
        assert!(
            formula.text.contains("$$"),
            "copy text keeps math delimiters"
        );
    }

    #[test]
    fn tex_bracket_delimiters_inside_code_stay_literal() {
        let blocks =
            parse("Literal `\\[x = 1\\]` and `\\(y = 2\\)`.\n\n```text\n\\[\nx = 1\n\\]\n```");
        assert!(blocks.iter().any(|block| {
            block.kind == kind::PARAGRAPH
                && block.display_text.contains("\\[x = 1\\]")
                && block.display_text.contains("\\(y = 2\\)")
        }));
        assert!(blocks.iter().any(|block| {
            block.kind == kind::CODE
                && block.display_text.contains("\\[")
                && block.display_text.contains("\\]")
        }));
    }

    #[test]
    fn tex_parenthesis_inline_becomes_normalized_math() {
        let blocks = parse(r"Matrix product: \(c_{ij} = \sum_{k=1}^{n} a_{ik} \cdot b_{kj}\).");
        let shown = &blocks[0].display_text;
        assert!(shown.contains('∑'));
        assert!(shown.contains('·'));
        assert!(!shown.contains("\\("));
        assert!(!shown.contains("\\sum"));
    }

    /// Micro-bench for audit P1: the tile re-`parse`s the WHOLE accumulated answer
    /// on every throttled (~50ms) streaming delta, which is O(n²) in answer length
    /// over a stream. This measures whether the PARSE itself is the bottleneck (vs
    /// the Slint VecModel rebuild + relayout, which a bench can't reach). Run:
    ///   cargo test --manifest-path slint-experiment/Cargo.toml -- --ignored --nocapture parse_streaming_cost
    #[test]
    #[ignore = "perf micro-bench — run explicitly with --ignored --nocapture"]
    fn parse_streaming_cost() {
        // A representative ~5 KB answer: headings, prose, a bullet list, a fenced
        // code block, inline code — the mix a 12B/Sonnet answer produces.
        let unit = "## Section\n\nHere is a paragraph of explanation with some \
            `inline code` and a [link](https://example.com/page) that the parser \
            must handle.\n\n- first bullet point about the topic\n- second bullet \
            with more detail\n- third\n\n```rust\nfn demo() -> i32 {\n    let x = 42;\n    x + 1\n}\n```\n\n";
        let mut answer = String::new();
        while answer.len() < 5000 {
            answer.push_str(unit);
        }
        let answer = &answer[..5000];

        // Simulate a 50ms-throttled stream of a 5000-char answer ≈ 200 renders,
        // each re-parsing the growing prefix (the current O(n²) behavior).
        const RENDERS: usize = 200;
        let t0 = std::time::Instant::now();
        let mut total_blocks = 0usize;
        for i in 1..=RENDERS {
            let end = (i * answer.len() / RENDERS).min(answer.len());
            // Slice on a char boundary so parse gets valid UTF-8.
            let end = (0..=end)
                .rev()
                .find(|&e| answer.is_char_boundary(e))
                .unwrap_or(0);
            total_blocks += parse(&answer[..end]).len();
        }
        let elapsed = t0.elapsed();
        // Also time ONE full parse of the final answer for reference.
        let t1 = std::time::Instant::now();
        let one = parse(answer).len();
        let one_elapsed = t1.elapsed();

        eprintln!(
            "P1 bench: {RENDERS} streamed re-parses of a {}-char answer = {:?} total \
             ({:?}/render avg); ONE full parse = {:?} ({one} blocks, {total_blocks} block-parses)",
            answer.len(),
            elapsed,
            elapsed / RENDERS as u32,
            one_elapsed,
        );
    }
}
