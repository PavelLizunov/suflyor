//! Utterance chunking, ported from suflyor-tts so both sidecars bound their
//! per-synthesis latency the same way. Short chunks make first audio arrive
//! quickly while playback continues to buffer the following chunks.

pub const MAX_CHUNK_CHARS: usize = 120;

pub fn sanitize(text: &str) -> String {
    text.chars()
        .filter(|&c| c == '\n' || c == '\t' || !c.is_control())
        .collect()
}

pub fn chunk_text(text: &str) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let mut cur = String::new();
    for sentence in split_sentences(text) {
        let s_len = sentence.chars().count();
        if s_len > MAX_CHUNK_CHARS {
            push_trimmed(&mut chunks, std::mem::take(&mut cur));
            for piece in hard_split(&sentence, MAX_CHUNK_CHARS) {
                push_trimmed(&mut chunks, piece);
            }
            continue;
        }
        if cur.chars().count() + s_len > MAX_CHUNK_CHARS && !cur.is_empty() {
            push_trimmed(&mut chunks, std::mem::take(&mut cur));
        }
        cur.push_str(&sentence);
    }
    push_trimmed(&mut chunks, cur);
    chunks
}

fn push_trimmed(out: &mut Vec<String>, s: String) {
    let t = s.trim();
    if !t.is_empty() {
        out.push(t.to_string());
    }
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        cur.push(ch);
        if matches!(ch, '.' | '!' | '?' | '…' | '\n' | ';') {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn hard_split(s: &str, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        if !cur.is_empty() && cur.chars().count() + 1 + word.chars().count() > max {
            out.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn splits_on_sentence_boundaries() {
        let chunks = chunk_text("Первое. Второе! Третье?");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "Первое. Второе! Третье?");
    }

    #[test]
    fn hard_splits_long_sentences() {
        let long = "слово ".repeat(200);
        let chunks = chunk_text(&long);
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.chars().count() <= MAX_CHUNK_CHARS);
        }
    }

    #[test]
    fn empty_and_blank_produce_nothing() {
        assert!(chunk_text("").is_empty());
        assert!(chunk_text("   \n ").is_empty());
    }

    #[test]
    fn sanitize_drops_control_chars_but_keeps_newlines() {
        assert_eq!(sanitize("a\u{0007}b\nc"), "ab\nc");
    }
}
