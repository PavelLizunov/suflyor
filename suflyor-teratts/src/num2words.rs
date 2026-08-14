//! Minimal `num2words`-compatible spelling of integers and decimals for `ru`
//! and `en`, matching the shapes the TeraTTS reference pipeline produces for
//! the values a read-aloud path realistically meets (cardinals, signed values,
//! short decimal fractions). The upstream dependency is Python `num2words`;
//! this module reproduces its output grammar without a Python runtime.

/// Spell a numeric literal (already matched by the tagged-number scanner).
/// `literal` may start with `-` or `−` and may contain one `.` or `,`
/// decimal separator. Returns `None` when the literal cannot be parsed.
pub fn num2words(literal: &str, lang: &str) -> Option<String> {
    let literal = literal.replace('−', "-");
    let (negative, rest) = literal
        .strip_prefix('-')
        .map(|r| (true, r))
        .unwrap_or((false, literal.as_str()));
    let (int_part, frac_part) = match rest.split_once(['.', ',']) {
        Some((i, f)) => (i, Some(f)),
        None => (rest, None),
    };
    if int_part.is_empty() || !int_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if let Some(f) = frac_part {
        if f.is_empty() || !f.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
    }
    let int_value: u64 = int_part.parse().ok()?;
    let mut out = match lang {
        "ru" => spell_decimal_ru(int_value, frac_part)?,
        "en" => spell_decimal_en(int_value, frac_part)?,
        _ => return None,
    };
    if negative {
        out.insert_str(
            0,
            if lang == "ru" {
                "минус "
            } else {
                "minus "
            },
        );
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// English
// ---------------------------------------------------------------------------

const EN_UNITS: [&str; 20] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];
const EN_TENS: [&str; 10] = [
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];

fn spell_below_1000_en(n: u64) -> String {
    let mut parts: Vec<String> = Vec::new();
    let hundreds = n / 100;
    let rest = n % 100;
    if hundreds > 0 {
        parts.push(format!("{} hundred", EN_UNITS[hundreds as usize]));
    }
    if rest > 0 {
        if hundreds > 0 {
            parts.push("and".into());
        }
        parts.push(spell_below_100_en_words(rest));
    }
    if parts.is_empty() {
        "zero".into()
    } else {
        parts.join(" ")
    }
}

fn spell_below_100_en_words(n: u64) -> String {
    if n < 20 {
        EN_UNITS[n as usize].to_string()
    } else {
        let tens = EN_TENS[(n / 10) as usize];
        let unit = n % 10;
        if unit == 0 {
            tens.to_string()
        } else {
            format!("{tens}-{}", EN_UNITS[unit as usize])
        }
    }
}

fn spell_int_en(n: u64) -> String {
    if n == 0 {
        return "zero".into();
    }
    let scales: [(u64, &str); 3] = [
        (1_000_000_000, "billion"),
        (1_000_000, "million"),
        (1_000, "thousand"),
    ];
    let mut parts: Vec<String> = Vec::new();
    let mut rest = n;
    for (value, name) in scales {
        if rest >= value {
            let count = rest / value;
            rest %= value;
            parts.push(format!("{} {name}", spell_below_1000_en(count)));
        }
    }
    if rest > 0 {
        parts.push(spell_below_1000_en(rest));
    }
    parts.join(", ")
}

fn spell_decimal_en(int_value: u64, frac: Option<&str>) -> Option<String> {
    let int_words = spell_int_en(int_value);
    let Some(frac) = frac else {
        return Some(int_words);
    };
    let mut digits = Vec::new();
    for c in frac.chars() {
        digits.push(EN_UNITS[(c as u8 - b'0') as usize]);
    }
    Some(format!("{} point {}", int_words, digits.join(" ")))
}

// ---------------------------------------------------------------------------
// Russian
// ---------------------------------------------------------------------------

const RU_UNITS_M: [&str; 10] = [
    "ноль",
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
const RU_UNITS_F: [&str; 10] = [
    "ноль",
    "одна",
    "две",
    "три",
    "четыре",
    "пять",
    "шесть",
    "семь",
    "восемь",
    "девять",
];
const RU_TEENS: [&str; 10] = [
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
const RU_TENS: [&str; 10] = [
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
const RU_HUNDREDS: [&str; 10] = [
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

/// Grammatical form triple for a Russian scale word: (1, 2..4, 5+) —
/// e.g. ("тысяча", "тысячи", "тысяч").
struct RuScale {
    value: u64,
    forms: [&'static str; 3],
    feminine: bool,
}

const RU_SCALES: [RuScale; 3] = [
    RuScale {
        value: 1_000_000_000,
        forms: ["миллиард", "миллиарда", "миллиардов"],
        feminine: false,
    },
    RuScale {
        value: 1_000_000,
        forms: ["миллион", "миллиона", "миллионов"],
        feminine: false,
    },
    RuScale {
        value: 1_000,
        forms: ["тысяча", "тысячи", "тысяч"],
        feminine: true,
    },
];

fn plural_index(n: u64) -> usize {
    let d10 = n % 10;
    let d100 = n % 100;
    if d10 == 1 && d100 != 11 {
        0
    } else if (2..=4).contains(&d10) && !(12..=14).contains(&d100) {
        1
    } else {
        2
    }
}

fn spell_below_1000_ru(n: u64, feminine: bool) -> String {
    let units = if feminine { RU_UNITS_F } else { RU_UNITS_M };
    let mut words: Vec<&str> = Vec::new();
    let hundreds = (n / 100) as usize;
    if hundreds > 0 {
        words.push(RU_HUNDREDS[hundreds]);
    }
    let rest = n % 100;
    if (10..20).contains(&rest) {
        words.push(RU_TEENS[(rest - 10) as usize]);
    } else {
        if rest >= 20 {
            words.push(RU_TENS[(rest / 10) as usize]);
        }
        let unit = rest % 10;
        if unit > 0 || n == 0 {
            words.push(units[unit as usize]);
        }
    }
    words.join(" ")
}

fn spell_int_ru(n: u64) -> String {
    if n == 0 {
        return "ноль".into();
    }
    let mut parts: Vec<String> = Vec::new();
    let mut rest = n;
    for scale in &RU_SCALES {
        if rest >= scale.value {
            let count = rest / scale.value;
            rest %= scale.value;
            parts.push(format!(
                "{} {}",
                spell_below_1000_ru(count, scale.feminine),
                scale.forms[plural_index(count)]
            ));
        }
    }
    if rest > 0 {
        parts.push(spell_below_1000_ru(rest, false));
    }
    parts.join(" ")
}

/// Denominator names for decimal fractions by digit count: (1, 5+).
fn frac_denominator_ru(digits: usize, one: bool) -> Option<&'static str> {
    let table: [(&str, &str); 4] = [
        ("одна десятая", "десятых"),
        ("одна сотая", "сотых"),
        ("одна тысячная", "тысячных"),
        ("одна десятитысячная", "десятитысячных"),
    ];
    let (one_form, many_form) = table.get(digits - 1)?;
    if one {
        Some(one_form)
    } else {
        Some(many_form)
    }
}

fn spell_decimal_ru(int_value: u64, frac: Option<&str>) -> Option<String> {
    let Some(frac) = frac else {
        return Some(spell_int_ru(int_value));
    };
    // Mirror float semantics: trailing zeros do not change the value.
    let frac = frac.trim_end_matches('0');
    if frac.is_empty() {
        return Some(spell_int_ru(int_value));
    }
    let digits = frac.len();
    let frac_value: u64 = frac.parse().ok()?;
    let int_words = format!(
        "{} {}",
        spell_below_1000_ru(int_value, true),
        whole_form_ru(int_value)
    );
    if digits <= 4 {
        let one = frac_value % 10 == 1 && frac_value % 100 != 11;
        let denominator = frac_denominator_ru(digits, one)?;
        if one {
            // "одна десятая" already carries the numeral.
            return Some(format!("{int_words} и {denominator}"));
        }
        return Some(format!(
            "{int_words} и {} {denominator}",
            spell_below_1000_ru(frac_value, true)
        ));
    }
    // Longer fractions than the named table: spell digits one by one. This
    // deviates from num2words (rare in practice) and is documented in the
    // RC17 goal doc.
    let mut digit_words: Vec<&str> = Vec::new();
    for c in frac.chars() {
        digit_words.push(RU_UNITS_F[(c as u8 - b'0') as usize]);
    }
    Some(format!("{int_words} и {}", digit_words.join(" ")))
}

/// Agreement for "целая/целые/целых" with the integer part (feminine).
fn whole_form_ru(n: u64) -> &'static str {
    match plural_index(n) {
        0 => "целая",
        1 => "целые",
        _ => "целых",
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn english_integers() {
        assert_eq!(num2words("0", "en").unwrap(), "zero");
        assert_eq!(num2words("21", "en").unwrap(), "twenty-one");
        assert_eq!(num2words("42", "en").unwrap(), "forty-two");
        assert_eq!(num2words("100", "en").unwrap(), "one hundred");
        assert_eq!(num2words("101", "en").unwrap(), "one hundred and one");
        assert_eq!(num2words("1000", "en").unwrap(), "one thousand");
        assert_eq!(num2words("21000", "en").unwrap(), "twenty-one thousand");
        assert_eq!(num2words("1000000", "en").unwrap(), "one million");
    }

    #[test]
    fn english_decimals_and_negatives() {
        assert_eq!(num2words("1.5", "en").unwrap(), "one point five");
        assert_eq!(num2words("1.25", "en").unwrap(), "one point two five");
        assert_eq!(num2words("-3", "en").unwrap(), "minus three");
        assert_eq!(num2words("−2.5", "en").unwrap(), "minus two point five");
    }

    #[test]
    fn russian_integers() {
        assert_eq!(num2words("0", "ru").unwrap(), "ноль");
        assert_eq!(num2words("21", "ru").unwrap(), "двадцать один");
        assert_eq!(num2words("42", "ru").unwrap(), "сорок два");
        assert_eq!(num2words("100", "ru").unwrap(), "сто");
        assert_eq!(num2words("1000", "ru").unwrap(), "одна тысяча");
        assert_eq!(num2words("2000", "ru").unwrap(), "две тысячи");
        assert_eq!(num2words("5000", "ru").unwrap(), "пять тысяч");
        assert_eq!(num2words("1000000", "ru").unwrap(), "один миллион");
        assert_eq!(
            num2words("123456", "ru").unwrap(),
            "сто двадцать три тысячи четыреста пятьдесят шесть"
        );
    }

    #[test]
    fn russian_decimals() {
        assert_eq!(num2words("1.5", "ru").unwrap(), "одна целая и пять десятых");
        assert_eq!(num2words("0.1", "ru").unwrap(), "ноль целых и одна десятая");
        assert_eq!(
            num2words("2.25", "ru").unwrap(),
            "две целые и двадцать пять сотых"
        );
        assert_eq!(
            num2words("3,14", "ru").unwrap(),
            "три целые и четырнадцать сотых"
        );
        assert_eq!(num2words("5.0", "ru").unwrap(), "пять");
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(num2words("", "ru"), None);
        assert_eq!(num2words(".", "ru"), None);
        assert_eq!(num2words("1.", "ru"), None);
        assert_eq!(num2words("abc", "en"), None);
        assert_eq!(num2words("99999999999999999999999", "en"), None);
        assert_eq!(num2words("5", "de"), None);
    }
}
