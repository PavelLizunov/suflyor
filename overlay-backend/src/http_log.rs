//! SECURITY (P0-1): safe HTTP-error logging for the shareable file log.
//!
//! AI/STT/Vision error paths used to log a 200–500-char snippet of the server
//! RESPONSE BODY at `warn!`. The FacadeLogger forwards that into
//! `overlay-host.log`, and "Собрать логи" exports it to `suflyor-log.txt` with
//! only path/host/IP redaction — NOT arbitrary body text. A server (esp. a local
//! bridge) that echoes the failed chat request, STT prompt/context, or transcript
//! in its error body could therefore place meeting content into the support file.
//!
//! The body NEVER carries information the file log needs that the status + size
//! don't. Log the operation, the status code, and the body BYTE LENGTH only.

/// Build the one safe line to log for a failed HTTP request. Deliberately takes
/// only the body LENGTH, never the body text, so a body snippet cannot reach
/// the (shareable) file log.
pub(crate) fn http_error_line(op: &str, status: u16, body_len: usize) -> String {
    format!("{op} HTTP {status} ({body_len} bytes)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_op_status_and_length() {
        let line = http_error_line("AI complete", 503, 1234);
        assert!(line.contains("AI complete"));
        assert!(line.contains("503"));
        assert!(line.contains("1234"));
    }

    #[test]
    fn never_carries_body_text() {
        // The body that WOULD have been snippet-logged before the fix. The helper
        // only ever receives its length, so none of its content can appear.
        let body = "TRANSCRIPT_SENTINEL secret prompt http://192.168.55.66/v1 C:/Users/alice";
        let line = http_error_line("STT", 500, body.len());
        assert!(
            !line.contains("TRANSCRIPT_SENTINEL"),
            "body text leaked: {line}"
        );
        assert!(!line.contains("secret"));
        assert!(!line.contains("192.168.55.66"));
        assert!(!line.contains("alice"));
        assert!(line.contains(&body.len().to_string()));
    }
}
