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
    if let Some(matrix) = normalize_pmatrices(input) {
        return matrix;
    }
    let restored_case_rows = (input.contains("\\begin{cases}") && !input.contains("\\\\"))
        .then(|| restore_case_rows(input));
    let input = restored_case_rows.as_deref().unwrap_or(input);
    let mut out = String::with_capacity(input.len());
    push_normalized(&mut out, input);
    out
}

fn restore_case_rows(input: &str) -> String {
    const BEGIN: &str = "\\begin{cases}";
    const END: &str = "\\end{cases}";
    let Some(begin) = input.find(BEGIN) else {
        return input.to_string();
    };
    let body_start = begin + BEGIN.len();
    let Some(relative_end) = input[body_start..].find(END) else {
        return input.to_string();
    };
    let body_end = body_start + relative_end;
    let body = &input[body_start..body_end];
    let mut restored = String::with_capacity(input.len() + 4);
    restored.push_str(&input[..body_start]);

    let mut index = 0;
    while index < body.len() {
        let rest = &body[index..];
        if let Some(after_slash) = rest.strip_prefix('\\') {
            let command_end = after_slash
                .char_indices()
                .find_map(|(offset, ch)| (!ch.is_ascii_alphabetic()).then_some(offset))
                .unwrap_or(after_slash.len());
            let command = &after_slash[..command_end];
            let is_tex_command = named_symbol(command).is_some()
                || matches!(
                    command,
                    "frac"
                        | "sqrt"
                        | "left"
                        | "right"
                        | "big"
                        | "bigl"
                        | "bigr"
                        | "quad"
                        | "qquad"
                        | "sin"
                        | "cos"
                        | "tan"
                        | "log"
                        | "ln"
                        | "exp"
                        | "text"
                        | "mathrm"
                        | "mathbf"
                        | "operatorname"
                );
            if is_tex_command {
                restored.push('\\');
            } else {
                restored.push_str("\\\\");
            }
            index += 1;
            continue;
        }
        let Some(ch) = rest.chars().next() else {
            break;
        };
        restored.push(ch);
        index += ch.len_utf8();
    }
    restored.push_str(&input[body_end..]);
    restored
}

fn normalize_pmatrices(input: &str) -> Option<String> {
    const BEGIN: &str = "\\begin{pmatrix}";
    const END: &str = "\\end{pmatrix}";
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    let mut found = false;

    while let Some(begin) = rest.find(BEGIN) {
        let body_start = begin + BEGIN.len();
        let Some(relative_end) = rest[body_start..].find(END) else {
            break;
        };
        let body_end = body_start + relative_end;

        found = true;
        push_normalized(&mut out, &rest[..begin]);
        out.push('(');
        let body = &rest[body_start..body_end];
        // Outside a math delimiter CommonMark unescapes TeX's `\\` row
        // separator to `\ ` before this display-only pass sees it.
        let body = if body.contains("\\\\") {
            body.to_string()
        } else {
            body.replace("\\ ", "\\\\ ")
        };
        for (row_index, row) in body.split("\\\\").enumerate() {
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
        rest = &rest[body_end + END.len()..];
    }

    if found {
        push_normalized(&mut out, rest);
        Some(out)
    } else {
        None
    }
}

fn has_math_delimiter(input: &str) -> bool {
    input.contains('$') || input.contains("\\(") || input.contains("\\[")
}

fn looks_like_bare_math(text: &str) -> bool {
    !text.contains("https://")
        && !text.contains("http://")
        && !text.contains('?')
        && !text.contains('/')
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
        if rest.starts_with("\\\\") {
            out.push('\n');
            out.push_str("  ");
            index += 2;
            continue;
        }
        if rest.starts_with("\\,") || rest.starts_with("\\;") || rest.starts_with("\\!") {
            out.push(' ');
            index += 2;
            continue;
        }
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
                    index = command_end;
                    continue;
                }
                if command == "frac" {
                    if let Some((numerator, after_numerator)) = tex_argument_at(text, command_end) {
                        if let Some((denominator, end)) = tex_argument_at(text, after_numerator) {
                            out.push('(');
                            push_normalized(out, numerator);
                            out.push_str(")/(");
                            push_normalized(out, denominator);
                            out.push(')');
                            index = end;
                            continue;
                        }
                    }
                } else if command == "sqrt" {
                    if let Some((radicand, end)) = tex_argument_at(text, command_end) {
                        out.push_str("√(");
                        push_normalized(out, radicand);
                        out.push(')');
                        index = end;
                        continue;
                    }
                } else if matches!(
                    command,
                    "left" | "right" | "big" | "bigl" | "bigr" | "Big" | "Bigl" | "Bigr"
                ) {
                    index = command_end;
                    continue;
                } else if matches!(command, "quad" | "qquad") {
                    out.push(' ');
                    index = command_end;
                    continue;
                } else if matches!(command, "begin" | "end") {
                    if let Some((environment, end)) = braced_group_at(text, command_end) {
                        if environment == "cases" {
                            if command == "begin" {
                                out.push_str("{ ");
                            }
                            index = end;
                            continue;
                        }
                    }
                }
                // Malformed or unknown commands remain literal.
                out.push_str(&text[index..command_end]);
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
            if let Some((script, end)) = tex_argument_at(text, index + ch.len_utf8()) {
                out.push(ch);
                out.push('(');
                push_normalized(out, script);
                out.push(')');
                index = end;
                continue;
            }
        }
        out.push(ch);
        index += ch.len_utf8();
    }
}

fn tex_argument_at(text: &str, start: usize) -> Option<(&str, usize)> {
    let whitespace = text[start..]
        .char_indices()
        .take_while(|(_, ch)| ch.is_whitespace())
        .map(|(_, ch)| ch.len_utf8())
        .sum::<usize>();
    let start = start + whitespace;
    if text[start..].starts_with('{') {
        return braced_group_at(text, start);
    }
    let ch = text[start..].chars().next()?;
    Some((&text[start..start + ch.len_utf8()], start + ch.len_utf8()))
}

fn braced_group_at(text: &str, start: usize) -> Option<(&str, usize)> {
    if !text[start..].starts_with('{') {
        return None;
    }
    let inner_start = start + 1;
    let mut depth = 1_usize;
    for (offset, ch) in text[inner_start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let end = inner_start + offset;
                    return Some((&text[inner_start..end], end + 1));
                }
            }
            _ => {}
        }
    }
    None
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
        "pm" => "±",
        "neq" => "≠",
        "le" | "leq" => "≤",
        "ge" | "geq" => "≥",
        "approx" => "≈",
        "equiv" => "≡",
        "infty" => "∞",
        "to" => "→",
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
        let parsed_url_fragment = "example.test/c_{ij}?q=sum_k=1";
        assert_eq!(
            normalize_math_display(parsed_url_fragment),
            parsed_url_fragment
        );

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

    #[test]
    fn renders_every_pmatrix_in_one_display_fragment() {
        let raw = concat!(
            "\\begin{pmatrix}1 & 2 \\\\ 3 & 4\\end{pmatrix} + ",
            "\\begin{pmatrix}5 & 6 \\\\ 7 & 8\\end{pmatrix} = ",
            "\\begin{pmatrix}6 & 8 \\\\ 10 & 12\\end{pmatrix}"
        );
        let shown = super::normalize_math_fragment(raw);
        assert_eq!(shown.matches('(').count(), 3);
        assert_eq!(shown.matches(')').count(), 3);
        assert!(!shown.contains("\\begin"));
        assert!(!shown.contains("\\end"));
        assert!(shown.contains("10  12"));
    }

    #[test]
    fn renders_common_algebra_without_raw_tex_commands() {
        let shown =
            normalize_math_display(r"x_{1,2}=\frac{-b\pm\sqrt{b^2-4ac}}{2a},\quad b_n\neq0");
        assert!(shown.contains("(-b±√(b²-4ac))/(2a)"), "{shown}");
        assert!(shown.contains("bₙ≠0"), "{shown}");
        assert!(!shown.contains("\\frac"));
        assert!(!shown.contains("\\sqrt"));
        assert!(!shown.contains("\\quad"));
    }

    #[test]
    fn renders_cases_as_readable_lines() {
        let shown =
            normalize_math_display(r"\begin{cases}a_1x+b_1y=c_1,\\a_2x+b_2y=c_2\end{cases}");
        assert!(shown.starts_with("{ a₁x+b₁y=c₁,"), "{shown}");
        assert!(shown.contains("\n  a₂x+b₂y=c₂"), "{shown}");
        assert!(!shown.contains("\\begin"));
        assert!(!shown.contains("\\end"));
    }
}
