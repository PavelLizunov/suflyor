//! Russian number normalization for the neural TTS sidecar.

const GROUPS: [[&str; 3]; 5] = [
    ["", "", ""],
    ["тысяча", "тысячи", "тысяч"],
    ["миллион", "миллиона", "миллионов"],
    ["миллиард", "миллиарда", "миллиардов"],
    ["триллион", "триллиона", "триллионов"],
];

/// Replace supported numeric tokens with words for Russian neural voices.
#[must_use]
pub fn normalize_for_speech(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    while pos < text.len() {
        if let Some((used, spoken)) = parse_at(text, pos) {
            out.push_str(&spoken);
            pos += used;
        } else {
            let ch = text[pos..].chars().next().unwrap_or_default();
            out.push(ch);
            pos += ch.len_utf8();
        }
    }
    out
}

fn parse_at(text: &str, pos: usize) -> Option<(usize, String)> {
    let rest = &text[pos..];
    let prev = text[..pos].chars().next_back();
    if prev.is_some_and(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }

    if rest.starts_with(['v', 'V']) {
        return parse_version(rest).filter(|(used, _)| token_ends(text, pos + used));
    }

    let (sign_len, negative) = match rest.chars().next()? {
        '-' | '−' if rest.chars().nth(1).is_some_and(|c| c.is_ascii_digit()) => {
            (rest.chars().next()?.len_utf8(), true)
        }
        c if c.is_ascii_digit() => (0, false),
        _ => return None,
    };
    let digits_len = rest[sign_len..]
        .bytes()
        .take_while(u8::is_ascii_digit)
        .count();
    let first = &rest[sign_len..sign_len + digits_len];
    let after = &rest[sign_len + digits_len..];

    if let Some((separator_len, second)) = parse_pair(after, ':') {
        let hour = first.parse::<u8>().ok()?;
        let minute = second.parse::<u8>().ok()?;
        if !negative && hour < 24 && minute < 60 {
            let used = sign_len + digits_len + separator_len + second.len();
            if token_ends(text, pos + used) {
                return Some((
                    used,
                    format!(
                        "{} {}",
                        integer_words(hour.into())?,
                        integer_words(minute.into())?
                    ),
                ));
            }
        }
    }

    for separator in ['-', '–', '—'] {
        if let Some((separator_len, second)) = parse_pair(after, separator) {
            let left = signed_integer_words(first, negative)?;
            let right = integer_words(second.parse().ok()?)?;
            let used = sign_len + digits_len + separator_len + second.len();
            if token_ends(text, pos + used) {
                return Some((used, format!("{left}-{right}")));
            }
        }
    }

    for separator in ['.', ','] {
        if let Some((separator_len, second)) = parse_pair(after, separator) {
            let left = signed_integer_words(first, negative)?;
            let right = digits_words(second)?;
            let used = sign_len + digits_len + separator_len + second.len();
            if token_ends(text, pos + used) {
                return Some((used, format!("{left} и {right}")));
            }
        }
    }

    let spaces = after.bytes().take_while(u8::is_ascii_whitespace).count();
    if after[spaces..].starts_with('%') {
        let value = first.parse::<u64>().ok()?;
        let number = signed_integer_words(first, negative)?;
        let used = sign_len + digits_len + spaces + 1;
        if token_ends(text, pos + used) {
            return Some((
                used,
                format!(
                    "{number} {}",
                    plural(value, ["процент", "процента", "процентов"])
                ),
            ));
        }
    }

    let used = sign_len + digits_len;
    token_ends(text, pos + used)
        .then(|| signed_integer_words(first, negative).map(|words| (used, words)))?
}

fn parse_version(rest: &str) -> Option<(usize, String)> {
    let prefix_len = 1;
    let tail = &rest[prefix_len..];
    let mut used = 0;
    let mut parts = Vec::new();
    loop {
        let len = tail[used..].bytes().take_while(u8::is_ascii_digit).count();
        if len == 0 {
            break;
        }
        parts.push(&tail[used..used + len]);
        used += len;
        if !tail[used..].starts_with('.') {
            break;
        }
        used += 1;
    }
    if parts.len() < 2 || tail[..used].ends_with('.') {
        return None;
    }
    let spoken = parts
        .into_iter()
        .map(digits_or_integer_words)
        .collect::<Option<Vec<_>>>()?
        .join(" ");
    Some((prefix_len + used, format!("вэ {spoken}")))
}

fn parse_pair(after: &str, separator: char) -> Option<(usize, &str)> {
    let separator_len = separator.len_utf8();
    if !after.starts_with(separator) {
        return None;
    }
    let tail = &after[separator_len..];
    let len = tail.bytes().take_while(u8::is_ascii_digit).count();
    (len > 0).then_some((separator_len, &tail[..len]))
}

fn token_ends(text: &str, end: usize) -> bool {
    text[end..]
        .chars()
        .next()
        .is_none_or(|c| !c.is_alphanumeric() && c != '_')
}

fn signed_integer_words(digits: &str, negative: bool) -> Option<String> {
    let value = digits.parse::<u64>().ok()?;
    let words = integer_words(value)?;
    Some(if negative {
        format!("минус {words}")
    } else {
        words
    })
}

fn digits_or_integer_words(digits: &str) -> Option<String> {
    if digits.len() > 1 && digits.starts_with('0') {
        digits_words(digits)
    } else {
        integer_words(digits.parse().ok()?)
    }
}

fn digits_words(digits: &str) -> Option<String> {
    digits
        .bytes()
        .map(|digit| integer_words(u64::from(digit.checked_sub(b'0')?)))
        .collect::<Option<Vec<_>>>()
        .map(|parts| parts.join(" "))
}

fn integer_words(value: u64) -> Option<String> {
    if value == 0 {
        return Some("ноль".to_string());
    }
    if value >= 1_000_000_000_000_000 {
        return None;
    }
    let mut parts = Vec::new();
    for group_index in (0..GROUPS.len()).rev() {
        let divisor = 1_000_u64.pow(group_index as u32);
        let group = (value / divisor) % 1_000;
        if group == 0 {
            continue;
        }
        parts.extend(group_words(group as u16, group_index == 1));
        if group_index > 0 {
            parts.push(plural(group, GROUPS[group_index]));
        }
    }
    Some(parts.join(" "))
}

fn group_words(value: u16, feminine: bool) -> Vec<&'static str> {
    const HUNDREDS: [&str; 10] = [
        "",
        "сто",
        "двести",
        "триста",
        "четыреста",
        "пятьсот",
        "шестьсот",
        "семьсот",
        "восемьсот",
        "девятьсот",
    ];
    const TEENS: [&str; 10] = [
        "десять",
        "одиннадцать",
        "двенадцать",
        "тринадцать",
        "четырнадцать",
        "пятнадцать",
        "шестнадцать",
        "семнадцать",
        "восемнадцать",
        "девятнадцать",
    ];
    const TENS: [&str; 10] = [
        "",
        "",
        "двадцать",
        "тридцать",
        "сорок",
        "пятьдесят",
        "шестьдесят",
        "семьдесят",
        "восемьдесят",
        "девяносто",
    ];
    const ONES: [&str; 10] = [
        "",
        "один",
        "два",
        "три",
        "четыре",
        "пять",
        "шесть",
        "семь",
        "восемь",
        "девять",
    ];
    let mut words = Vec::new();
    let hundreds = value / 100;
    if hundreds > 0 {
        words.push(HUNDREDS[usize::from(hundreds)]);
    }
    let rest = value % 100;
    if (10..20).contains(&rest) {
        words.push(TEENS[usize::from(rest - 10)]);
        return words;
    }
    let tens = rest / 10;
    if tens > 0 {
        words.push(TENS[usize::from(tens)]);
    }
    let ones = rest % 10;
    if ones > 0 {
        words.push(match (feminine, ones) {
            (true, 1) => "одна",
            (true, 2) => "две",
            _ => ONES[usize::from(ones)],
        });
    }
    words
}

fn plural(value: u64, forms: [&'static str; 3]) -> &'static str {
    let last_two = value % 100;
    if (11..=14).contains(&last_two) {
        forms[2]
    } else {
        match value % 10 {
            1 => forms[0],
            2..=4 => forms[1],
            _ => forms[2],
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

    use super::normalize_for_speech as n;

    #[test]
    fn integers() {
        assert_eq!(
            n("0 7 11 21 105 123"),
            "ноль семь одиннадцать двадцать один сто пять сто двадцать три"
        );
    }

    #[test]
    fn negative_integer() {
        assert_eq!(
            n("Температура -25 градусов"),
            "Температура минус двадцать пять градусов"
        );
    }

    #[test]
    fn thousands_use_feminine_forms() {
        assert_eq!(n("21002"), "двадцать одна тысяча два");
    }

    #[test]
    fn large_integer() {
        assert_eq!(
            n("1002003004005"),
            "один триллион два миллиарда три миллиона четыре тысячи пять"
        );
    }

    #[test]
    fn value_above_trillions_is_unchanged() {
        assert_eq!(n("1000000000000000"), "1000000000000000");
    }

    #[test]
    fn percents_with_and_without_space() {
        assert_eq!(
            n("1% 2 % 5% 11% 21%"),
            "один процент два процента пять процентов одиннадцать процентов двадцать один процент"
        );
    }

    #[test]
    fn clock_time() {
        assert_eq!(n("Встреча в 14:30."), "Встреча в четырнадцать тридцать.");
    }

    #[test]
    fn invalid_clock_is_left_as_punctuation() {
        assert_eq!(n("25:99"), "двадцать пять:девяносто девять");
    }

    #[test]
    fn hyphen_range() {
        assert_eq!(n("3-5 дней"), "три-пять дней");
    }

    #[test]
    fn unicode_dash_ranges() {
        assert_eq!(n("3–5 и 7—9"), "три-пять и семь-девять");
    }

    #[test]
    fn decimal_dot_and_comma() {
        assert_eq!(n("3.5 и -0,25"), "три и пять и минус ноль и два пять");
    }

    #[test]
    fn version() {
        assert_eq!(n("v0.33.0"), "вэ ноль тридцать три ноль");
    }

    #[test]
    fn version_preserves_leading_zeroes() {
        assert_eq!(n("V01.002"), "вэ ноль один ноль ноль два");
    }

    #[test]
    fn no_numbers_is_identity() {
        assert_eq!(n("Привет, мир!"), "Привет, мир!");
    }

    #[test]
    fn mixed_russian_english_sentence() {
        assert_eq!(
            n("API v0.33.0 готов на 45%."),
            "API вэ ноль тридцать три ноль готов на сорок пять процентов."
        );
    }

    #[test]
    fn digits_inside_identifiers_are_untouched() {
        assert_eq!(n("win32 x86_64"), "win32 x86_64");
    }
}
