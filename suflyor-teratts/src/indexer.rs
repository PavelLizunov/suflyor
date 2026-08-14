//! Unicode-indexer tokenizer: the released `unicode_indexer.json` table maps
//! BMP codepoints to model token ids (negative = unsupported). Mirrors the
//! reference `UnicodeIndexer` class, including its 65,536-entry invariant.

use std::path::Path;

use anyhow::{anyhow, Result};

pub const TABLE_LEN: usize = 65_536;

#[derive(Debug)]
pub struct UnicodeIndexer {
    table: Vec<i64>,
}

impl UnicodeIndexer {
    pub fn load(path: &Path) -> Result<Self> {
        let raw =
            std::fs::read_to_string(path).map_err(|e| anyhow!("read unicode indexer: {e}"))?;
        Self::from_json(&raw)
    }

    pub fn from_json(raw: &str) -> Result<Self> {
        let table: Vec<i64> =
            serde_json::from_str(raw).map_err(|e| anyhow!("parse unicode indexer: {e}"))?;
        if table.len() != TABLE_LEN {
            return Err(anyhow!(
                "unicode_indexer.json must have {} entries, got {}",
                TABLE_LEN,
                table.len()
            ));
        }
        Ok(Self { table })
    }

    /// Token id for one character, or −1 when unsupported. Characters outside
    /// the BMP are always unsupported in the released table.
    pub fn token(&self, c: char) -> i64 {
        let cp = c as u32;
        if cp >= TABLE_LEN as u32 {
            return -1;
        }
        self.table[cp as usize]
    }

    /// `true` when every NFKD component of `c` has a token in the table —
    /// the exact support test the reference pipeline applies per character.
    pub fn supports(&self, c: char) -> bool {
        use unicode_normalization::UnicodeNormalization;
        let mut any = false;
        for part in c.nfkd() {
            any = true;
            if self.token(part) < 0 {
                return false;
            }
        }
        any
    }

    /// Batch-encode text → `(text_ids [1,N] i64, text_mask [1,1,N] f32)`.
    pub fn batch(&self, text: &str) -> Result<(Vec<i64>, Vec<f32>)> {
        let mut ids: Vec<i64> = Vec::new();
        for c in text.chars() {
            let token = self.token(c);
            if token < 0 {
                return Err(anyhow!("unsupported character U+{:04X}", c as u32));
            }
            ids.push(token);
        }
        if ids.is_empty() {
            return Err(anyhow!("text produced no tokens"));
        }
        let mask = vec![1.0_f32; ids.len()];
        Ok((ids, mask))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn fake_table(f: impl Fn(u32) -> i64) -> UnicodeIndexer {
        let table = (0..TABLE_LEN as u32).map(f).collect();
        UnicodeIndexer { table }
    }

    #[test]
    fn rejects_wrong_table_size() {
        let err = UnicodeIndexer::from_json("[1, 2, 3]").unwrap_err();
        assert!(err.to_string().contains("65536"));
    }

    #[test]
    fn batch_encodes_and_masks() {
        let idx = fake_table(|cp| if cp < 128 { cp as i64 } else { -1 });
        let (ids, mask) = idx.batch("ab").unwrap();
        assert_eq!(ids, vec![97, 98]);
        assert_eq!(mask, vec![1.0, 1.0]);
    }

    #[test]
    fn batch_rejects_unsupported_and_empty() {
        let idx = fake_table(|cp| if cp < 128 { cp as i64 } else { -1 });
        assert!(idx.batch("a\u{4e2d}").is_err());
        assert!(idx.batch("").is_err());
    }

    #[test]
    fn supports_follows_nfkd_components() {
        // Table with base letters but no combining marks: composed "й"
        // (NFKD → и + combining breve) is unsupported, plain "a" is.
        let idx = fake_table(|cp| {
            let c = char::from_u32(cp);
            match c {
                Some(c) if c.is_ascii_lowercase() || "аеиоу".contains(c) => cp as i64,
                _ => -1,
            }
        });
        assert!(idx.supports('a'));
        assert!(!idx.supports('й'));
        assert!(!idx.supports('\u{1F600}'));
    }
}
