//! Small display-only normalizer for the common TeX fragments AI answers use.
//!
//! This deliberately is not a TeX parser: unknown commands stay intact, and
//! callers keep source text for copy, prompts, persistence, and export.

/// Make a small, safe subset of inline TeX legible in flat Slint text.
#[must_use]
pub fn normalize_math_display(input: &str) -> String {
    if !has_math_delimiter(input) {
        return if looks_like_bare_math(input) {
            normalize_math_fragment(input)
        } else {
            input.to_string()
        };
    }
    let mut out = String::with_capacity(input.len());
    let mut plain_start = 0;
    let mut index = 0;

    while index < input.len() {
        if starts_url(input, index) {
            out.push_str(&input[plain_start..index]);
            let end = url_end(input, index);
            out.push_str(&input[index..end]);
            index = end;
            plain_start = end;
            continue;
        }
        if let Some((open, close)) = delimiter_at(input, index) {
            let inner_start = index + open.len();
            if let Some(relative_end) = input[inner_start..].find(close) {
                let inner_end = inner_start + relative_end;
                let inner = &input[inner_start..inner_end];
                if looks_like_delimited_math(inner) {
                    out.push_str(&input[plain_start..index]);
                    out.push_str(&normalize_math_fragment(inner));
                    index = inner_end + close.len();
                    plain_start = index;
                    continue;
                }
            } else if looks_like_math(&input[inner_start..]) {
                // An unfinished math delimiter is safer and more useful verbatim.
                return input.to_string();
            }
        }
        let Some(ch) = input[index..].chars().next() else {
            break;
        };
        index += ch.len_utf8();
    }
    out.push_str(&input[plain_start..]);
    out
}

/// Normalize a fragment already known to be math (for pulldown-cmark math events).
#[must_use]
pub(crate) fn normalize_math_fragment(input: &str) -> String {
    if let Some(matrix) = normalize_pmatrix(input) {
        return matrix;
    }
    let mut out = String::with_capacity(input.len());
    push_normalized(&mut out, input);
    out
}

fn normalize_pmatrix(input: &str) -> Option<String> {
    const BEGIN: &str = "\\begin{pmatrix}";
    const END: &str = "\\end{pmatrix}";
    let begin = input.find(BEGIN)?;
    let body_start = begin + BEGIN.len();
    let body_end = body_start + input[body_start..].find(END)?;

    let mut out = String::with_capacity(input.len());
    push_normalized(&mut out, &input[..begin]);
    out.push('(');
    for (row_index, row) in input[body_start..body_end].split("\\\\").enumerate() {
        if row_index > 0 {
            out.push('\n');
            out.push_str("  ");
        }
        for (cell_index, cell) in row.trim().split('&').enumerate() {
            if cell_index > 0 {
                out.push_str("  ");
            }
            push_normalized(&mut out, cell.trim());
        }
    }
    out.push(')');
    push_normalized(&mut out, &input[body_end + END.len()..]);
    Some(out)
}

fn has_math_delimiter(input: &str) -> bool {
    input.contains('$') || input.contains("\\(") || input.contains("\\[")
}

fn looks_like_bare_math(text: &str) -> bool {
    !text.contains("https://")
        && !text.contains("http://")
        && text.contains('=')
        && looks_like_math(text)
}

pub(crate) fn looks_like_delimited_math(text: &str) -> bool {
    looks_like_math(text)
        || text
            .trim()
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic() && text.trim().chars().count() == 1)
}

fn starts_url(input: &str, index: usize) -> bool {
    input[index..].starts_with("https://") || input[index..].starts_with("http://")
}

fn url_end(input: &str, index: usize) -> usize {
    input[index..]
        .char_indices()
        .find_map(|(offset, ch)| ch.is_whitespace().then_some(index + offset))
        .unwrap_or(input.len())
}

fn delimiter_at(input: &str, index: usize) -> Option<(&'static str, &'static str)> {
    let rest = &input[index..];
    if rest.starts_with("$$") {
        Some(("$$", "$$"))
    } else if rest.starts_with("\\[") {
        Some(("\\[", "\\]"))
    } else if rest.starts_with("\\(") {
        Some(("\\(", "\\)"))
    } else if rest.starts_with('$') {
        Some(("$", "$"))
    } else {
        None
    }
}

pub(crate) fn looks_like_math(text: &str) -> bool {
    well_formed_fragment(text)
        && (text.contains('\\')
            || text.contains('_')
            || text.contains('^')
            || (text.contains('=') && !looks_like_numeric_amounts(text)))
}

fn well_formed_fragment(text: &str) -> bool {
    let mut depth = 0_usize;
    for ch in text.chars() {
        match ch {
            '{' => depth += 1,
            '}' if depth == 0 => return false,
            '}' => depth -= 1,
            _ => {}
        }
    }
    if depth != 0 {
        return false;
    }

    let mut index = 0;
    while index < text.len() {
        let Some(ch) = text[index..].chars().next() else {
            break;
        };
        index += ch.len_utf8();
        if !matches!(ch, '_' | '^') {
            continue;
        }
        let rest = &text[index..];
        if let Some(braced) = rest.strip_prefix('{') {
            if braced.find('}').is_none_or(|closing| closing == 0) {
                return false;
            }
        } else if rest
            .chars()
            .next()
            .is_none_or(|next| next.is_whitespace() || matches!(next, '_' | '^' | '}'))
        {
            return false;
        }
    }
    true
}

fn looks_like_numeric_amounts(text: &str) -> bool {
    let Some((left, right)) = text.split_once('=') else {
        return false;
    };
    [left, right].into_iter().all(|amount| {
        let amount = amount.trim();
        !amount.is_empty()
            && amount
                .chars()
                .all(|ch| ch.is_ascii_digit() || matches!(ch, '.' | ',' | ' '))
    })
}

fn push_normalized(out: &mut String, text: &str) {
    let mut index = 0;
    while index < text.len() {
        let rest = &text[index..];
        if rest.starts_with('\\') {
            let command_start = index + 1;
            let command_end = text[command_start..]
                .char_indices()
                .find_map(|(offset, ch)| {
                    (!ch.is_ascii_alphabetic()).then_some(command_start + offset)
                })
                .unwrap_or(text.len());
            if command_end > command_start {
                let command = &text[command_start..command_end];
                if let Some(symbol) = named_symbol(command) {
                    out.push_str(symbol);
                } else {
                    out.push_str(&text[index..command_end]);
                }
                index = command_end;
                continue;
            }
        }
        let Some(ch) = rest.chars().next() else {
            break;
        };
        if matches!(ch, '_' | '^') {
            if let Some((script, end)) = script_at(text, index + ch.len_utf8(), ch == '_') {
                out.push_str(&script);
                index = end;
                continue;
            }
        }
        out.push(ch);
        index += ch.len_utf8();
    }
}

fn script_at(text: &str, start: usize, subscript: bool) -> Option<(String, usize)> {
    let rest = &text[start..];
    let (content, end) = if let Some(braced) = rest.strip_prefix('{') {
        let closing = braced.find('}')?;
        (&braced[..closing], start + closing + 2)
    } else {
        let ch = rest.chars().next()?;
        (&rest[..ch.len_utf8()], start + ch.len_utf8())
    };
    if content.is_empty() {
        return None;
    }
    let mut converted = String::with_capacity(content.len());
    for ch in content.chars() {
        converted.push(script_char(ch, subscript)?);
    }
    Some((converted, end))
}

fn script_char(ch: char, subscript: bool) -> Option<char> {
    let value = if subscript {
        match ch {
            '0' => '₀',
            '1' => '₁',
            '2' => '₂',
            '3' => '₃',
            '4' => '₄',
            '5' => '₅',
            '6' => '₆',
            '7' => '₇',
            '8' => '₈',
            '9' => '₉',
            '+' => '₊',
            '-' => '₋',
            '=' => '₌',
            '(' => '₍',
            ')' => '₎',
            'i' => 'ᵢ',
            'j' => 'ⱼ',
            'k' => 'ₖ',
            'l' => 'ₗ',
            'm' => 'ₘ',
            'n' => 'ₙ',
            'p' => 'ₚ',
            'r' => 'ᵣ',
            's' => 'ₛ',
            't' => 'ₜ',
            'x' => 'ₓ',
            _ => return None,
        }
    } else {
        match ch {
            '0' => '⁰',
            '1' => '¹',
            '2' => '²',
            '3' => '³',
            '4' => '⁴',
            '5' => '⁵',
            '6' => '⁶',
            '7' => '⁷',
            '8' => '⁸',
            '9' => '⁹',
            '+' => '⁺',
            '-' => '⁻',
            '=' => '⁼',
            '(' => '⁽',
            ')' => '⁾',
            'a' => 'ᵃ',
            'b' => 'ᵇ',
            'c' => 'ᶜ',
            'd' => 'ᵈ',
            'e' => 'ᵉ',
            'f' => 'ᶠ',
            'g' => 'ᵍ',
            'h' => 'ʰ',
            'i' => 'ⁱ',
            'j' => 'ʲ',
            'k' => 'ᵏ',
            'l' => 'ˡ',
            'm' => 'ᵐ',
            'n' => 'ⁿ',
            'o' => 'ᵒ',
            'p' => 'ᵖ',
            'r' => 'ʳ',
            's' => 'ˢ',
            't' => 'ᵗ',
            'u' => 'ᵘ',
            'v' => 'ᵛ',
            'w' => 'ʷ',
            'x' => 'ˣ',
            'y' => 'ʸ',
            'z' => 'ᶻ',
            _ => return None,
        }
    };
    Some(value)
}

fn named_symbol(command: &str) -> Option<&'static str> {
    Some(match command {
        "sum" => "∑",
        "prod" => "∏",
        "times" => "×",
        "cdot" => "·",
        "alpha" => "α",
        "beta" => "β",
        "gamma" => "γ",
        "delta" => "δ",
        "epsilon" => "ε",
        "theta" => "θ",
        "lambda" => "λ",
        "mu" => "μ",
        "pi" => "π",
        "rho" => "ρ",
        "sigma" => "σ",
        "tau" => "τ",
        "phi" => "φ",
        "omega" => "ω",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
    use super::normalize_math_display;

    #[test]
    fn normalizes_reference_formula_with_or_without_delimiters() {
        let raw = "c_{ij} = \\sum_{k=1}^{n} a_{ik}b_{kj}";
        let expected = "cᵢⱼ = ∑ₖ₌₁ⁿ aᵢₖbₖⱼ";
        assert_eq!(normalize_math_display(raw), expected);
        assert_eq!(normalize_math_display(&format!("${raw}$")), expected);
    }

    #[test]
    fn malformed_unknown_and_currency_like_text_stays_literal() {
        let cases = [
            "Неполное c_{ij и \\frac{a}{b}",
            "x_{} и x^ и \\sqrt{x}",
            "$c_{ij} = \\sum_{k=1}^{n}",
            "$100 = 90$",
            "$x^$",
            "$x_{}$",
        ];
        for input in cases {
            assert_eq!(normalize_math_display(input), input);
        }
        assert_eq!(
            normalize_math_display("Use file_name and x^ray literally."),
            "Use file_name and x^ray literally."
        );
    }

    #[test]
    fn urls_are_verbatim_and_normalization_is_idempotent() {
        let url = "https://example.test/c_{ij}?q=\\sum_{k=1}";
        assert_eq!(normalize_math_display(url), url);

        let once = normalize_math_display("$c_{ij} = \\sum_{k=1}^{n}$");
        assert_eq!(normalize_math_display(&once), once);
    }

    #[test]
    fn strips_delimiters_from_single_letter_variables() {
        assert_eq!(
            normalize_math_display("Элемент из $i$-й строки и $j$-го столбца"),
            "Элемент из i-й строки и j-го столбца"
        );
    }

    #[test]
    fn renders_pmatrix_rows_without_latex_scaffolding() {
        let raw = "A = \\begin{pmatrix} a_{11} & a_{12} \\\\ a_{21} & a_{22} \\end{pmatrix}";
        assert_eq!(normalize_math_display(raw), "A = (a₁₁  a₁₂\n  a₂₁  a₂₂)");
    }
}
