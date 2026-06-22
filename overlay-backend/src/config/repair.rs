//! Self-heal for externally-mangled profile context: reverse the classic
//! "UTF-8 bytes read as Windows-1252 (cp1252) and re-saved as UTF-8" corruption
//! (mojibake — e.g. "**Ð Ð¾Ð»ÑŒ:**" is "**Роль:**"). See [`super::load`].
//!
//! The app's OWN read/write is pure UTF-8 (serde_json + reqwest json), so this
//! never fires for in-app edits. It only catches a config.json that an OUTSIDE
//! tool round-tripped through a single-byte codepage: Notepad "ANSI" save,
//! PowerShell `Get-Content`/`Set-Content`/`Out-File` without `-Encoding utf8`,
//! or a paste from a cp1252 source. The app's strict-UTF-8 load then accepts the
//! now-valid-UTF-8 mojibake; this reverses it so the user never sees garbled
//! Cyrillic.

/// Map a char back to the cp1252 byte it was decoded FROM. Identity for
/// `0x00..=0xFF` (ASCII + Latin-1 high range + the five cp1252-UNDEFINED bytes
/// `0x81/0x8D/0x8F/0x90/0x9D`, which the corrupting tool passes through as
/// `U+0081`/`8D`/`8F`/`90`/`9D`); the 27 cp1252 `0x80..=0x9F` graphic chars map
/// to their defining byte. `None` for any other char — i.e. real text that was
/// never a single-byte round-trip.
fn cp1252_char_to_byte(c: char) -> Option<u8> {
    let cp = c as u32;
    let byte = match cp {
        0x00..=0xFF => cp as u8,
        0x20AC => 0x80,
        0x201A => 0x82,
        0x0192 => 0x83,
        0x201E => 0x84,
        0x2026 => 0x85,
        0x2020 => 0x86,
        0x2021 => 0x87,
        0x02C6 => 0x88,
        0x2030 => 0x89,
        0x0160 => 0x8A,
        0x2039 => 0x8B,
        0x0152 => 0x8C,
        0x017D => 0x8E,
        0x2018 => 0x91,
        0x2019 => 0x92,
        0x201C => 0x93,
        0x201D => 0x94,
        0x2022 => 0x95,
        0x2013 => 0x96,
        0x2014 => 0x97,
        0x02DC => 0x98,
        0x2122 => 0x99,
        0x0161 => 0x9A,
        0x203A => 0x9B,
        0x0153 => 0x9C,
        0x017E => 0x9E,
        0x0178 => 0x9F,
        _ => return None,
    };
    Some(byte)
}

/// Return the de-mojibaked string if `s` is UTF-8 that was read as cp1252 and
/// re-saved (a single-byte round-trip of Russian text), else `None`.
///
/// CONSERVATIVE — fires ONLY when ALL of these hold, so legitimate text is never
/// touched:
///
/// - `s` is non-empty and has a char `>= U+0080` (a high byte to undo),
/// - `s` has NO real Cyrillic (`U+0400..=U+04FF`) yet,
/// - every char maps to a cp1252 byte, and
/// - the reconstructed bytes are valid UTF-8 that GAINS Cyrillic.
///
/// Legit Russian (already has Cyrillic) and legit Latin/accented text (a lone
/// high byte is invalid UTF-8, so reconstruction fails / gains no Cyrillic) both
/// return `None`.
pub(super) fn repair_cp1252_mojibake(s: &str) -> Option<String> {
    if s.is_empty() {
        return None;
    }
    let has_high = s.chars().any(|c| c as u32 >= 0x80);
    let has_cyrillic = s.chars().any(|c| matches!(c as u32, 0x0400..=0x04FF));
    if !has_high || has_cyrillic {
        return None;
    }
    let mut bytes = Vec::with_capacity(s.len());
    for c in s.chars() {
        bytes.push(cp1252_char_to_byte(c)?);
    }
    let repaired = String::from_utf8(bytes).ok()?;
    if repaired
        .chars()
        .any(|c| matches!(c as u32, 0x0400..=0x04FF))
    {
        Some(repaired)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    /// Map a raw byte to the char a cp1252 decoder produces — the forward
    /// direction of the bug, used to synthesise mojibake from clean UTF-8.
    fn byte_to_cp1252_char(b: u8) -> char {
        match b {
            0x80 => '\u{20AC}',
            0x82 => '\u{201A}',
            0x83 => '\u{0192}',
            0x84 => '\u{201E}',
            0x85 => '\u{2026}',
            0x86 => '\u{2020}',
            0x87 => '\u{2021}',
            0x88 => '\u{02C6}',
            0x89 => '\u{2030}',
            0x8A => '\u{0160}',
            0x8B => '\u{2039}',
            0x8C => '\u{0152}',
            0x8E => '\u{017D}',
            0x91 => '\u{2018}',
            0x92 => '\u{2019}',
            0x93 => '\u{201C}',
            0x94 => '\u{201D}',
            0x95 => '\u{2022}',
            0x96 => '\u{2013}',
            0x97 => '\u{2014}',
            0x98 => '\u{02DC}',
            0x99 => '\u{2122}',
            0x9A => '\u{0161}',
            0x9B => '\u{203A}',
            0x9C => '\u{0153}',
            0x9E => '\u{017E}',
            0x9F => '\u{0178}',
            // ASCII + Latin-1 high + the 5 cp1252-undefined passthroughs.
            _ => b as char,
        }
    }

    /// Reproduce the bug: UTF-8 bytes of `s`, each decoded as a cp1252 char.
    fn corrupt(s: &str) -> String {
        s.bytes().map(byte_to_cp1252_char).collect()
    }

    #[test]
    fn repairs_real_mojibake() {
        // Includes the cp1252-only signature byte 0x8C (Œ) from "ь" = D1 8C.
        let original = "**Роль:** Семейный психолог / Медиатор\n*   Конфликтология и медиация";
        let moji = corrupt(original);
        assert_ne!(
            moji, original,
            "synthetic corruption must change the string"
        );
        assert!(moji.contains('Ð'), "mojibake should carry the Ð marker");
        assert_eq!(repair_cp1252_mojibake(&moji).as_deref(), Some(original));
    }

    #[test]
    fn leaves_clean_russian_untouched() {
        assert_eq!(repair_cp1252_mojibake("**Роль:** Семейный психолог"), None);
    }

    #[test]
    fn leaves_ascii_untouched() {
        assert_eq!(repair_cp1252_mojibake("kubernetes k8s helm etcd"), None);
    }

    #[test]
    fn leaves_empty_untouched() {
        assert_eq!(repair_cp1252_mojibake(""), None);
    }

    #[test]
    fn leaves_legit_latin1_untouched() {
        // French accents: high bytes, but a lone 0xE9 etc. is invalid UTF-8 so
        // reconstruction fails — must not be "repaired".
        assert_eq!(repair_cp1252_mojibake("café résumé naïve"), None);
    }

    #[test]
    fn idempotent_on_already_clean() {
        let clean = "Роль: инженер";
        // corrupt → repair gives clean; repairing clean again is a no-op.
        let moji = corrupt(clean);
        let fixed = repair_cp1252_mojibake(&moji).unwrap();
        assert_eq!(fixed, clean);
        assert_eq!(repair_cp1252_mojibake(&fixed), None);
    }
}
