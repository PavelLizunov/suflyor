    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::control::*;
    use super::*;
    use serde_json::json;

    #[test]
    fn managed_mlx_intent_is_exact_and_does_not_capture_external_local_servers() {
        let managed = AiEndpoint {
            protocol: AiProtocol::OpenAiCompatible,
            base_url: String::new(),
            bearer: String::new(),
            model: crate::mlx_install::DEFAULT_TEXT_MODEL.into(),
            reasoning_effort: None,
            is_local: true,
        };
        assert!(is_managed_mlx_endpoint(&managed));

        let mut external = managed.clone();
        external.base_url = "http://external.invalid/v1".into();
        assert!(!is_managed_mlx_endpoint(&external));

        let mut unknown = managed;
        unknown.model = "user/model".into();
        assert!(!is_managed_mlx_endpoint(&unknown));
    }

    #[test]
    fn managed_gemma_uses_the_handoff_sampler_without_forced_seed() {
        let mut managed = json!({});
        apply_managed_gemma_sampler(
            &mut managed,
            crate::local_ai::LLAMA_BASE_URL,
            "gemma-4-26B-A4B-it-UD-Q2_K_XL.gguf",
        );
        assert_eq!(managed["temperature"], json!(1.0));
        assert_eq!(managed["top_p"], json!(0.95));
        assert_eq!(managed["top_k"], json!(64));
        assert!(managed.get("seed").is_none());

        let mut external = json!({});
        apply_managed_gemma_sampler(&mut external, "http://127.0.0.1:9999/v1", "gemma-custom");
        assert_eq!(external, json!({}));
    }

    #[tokio::test]
    async fn queued_stream_stops_when_receiver_is_dropped() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let permits = AI_SEMAPHORE.acquire_many(2).await.unwrap();

        let rx = stream_chat(base_url, String::new(), String::new(), Vec::new(), 1);
        tokio::task::yield_now().await;
        drop(rx);
        tokio::task::yield_now().await;
        drop(permits);

        let reacquired = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            AI_SEMAPHORE.acquire_many(2),
        )
        .await;
        assert!(
            reacquired.is_ok(),
            "a queued stream kept a permit after its receiver was dropped"
        );
    }

    /// Structuring (`force = true`) must disable local thinking REGARDLESS of the
    /// global `ai_local_thinking` toggle — this is the v0.18.6 fix for the tester
    /// bug where "режим рассуждение" ON broke the meeting summary. The live-answer
    /// path (`force = false`) must keep honoring the toggle.
    #[test]
    fn force_no_think_overrides_global_toggle() {
        fn thinking_disabled(body: &Value) -> bool {
            body.get("chat_template_kwargs")
                .and_then(|k| k.get("enable_thinking"))
                .and_then(serde_json::Value::as_bool)
                == Some(false)
        }
        // Global OFF (the default): live answers think, structuring does NOT.
        set_local_no_think(false);
        let mut live = json!({});
        apply_local_no_think(&mut live, false);
        assert!(
            !thinking_disabled(&live),
            "live answer with global-off must not force no-think"
        );
        let mut structuring = json!({});
        apply_local_no_think(&mut structuring, true);
        assert!(
            thinking_disabled(&structuring),
            "structuring must force no-think even when the global toggle is off"
        );

        // Global ON (user disabled thinking): both paths disable it.
        set_local_no_think(true);
        let mut live2 = json!({});
        apply_local_no_think(&mut live2, false);
        assert!(thinking_disabled(&live2));
        // Restore the default so other tests / process state aren't perturbed.
        set_local_no_think(false);
    }

    // ── Regression: P0 bug — UTF-8 split across network chunks must NOT panic ──

    #[test]
    fn drain_returns_empty_when_no_complete_frame() {
        let mut b: Vec<u8> = b"data: hello".to_vec();
        let s = drain_complete_frames(&mut b);
        assert_eq!(s, "");
        assert_eq!(b, b"data: hello"); // bytes preserved for next chunk
    }

    #[test]
    fn drain_splits_at_double_newline() {
        let mut b: Vec<u8> = b"data: a\n\ndata: b".to_vec();
        let s = drain_complete_frames(&mut b);
        assert_eq!(s, "data: a\n\n");
        assert_eq!(b, b"data: b"); // unfinished frame stays
    }

    /// THE bug we're guarding against: a Russian 2-byte char's bytes are
    /// split across two network reads. The first read ends mid-char; the
    /// second completes it. Old code did `from_utf8(&chunk).unwrap()` and
    /// would panic. New code must keep the leftover for the next call.
    #[test]
    fn drain_does_not_panic_when_utf8_split_across_chunks() {
        // "Привет" — П = 0xD0 0x9F. Find the byte offset that lands mid-char.
        let full = "data: \"Привет\"\n\n";
        let bytes = full.as_bytes();
        // First non-ASCII byte should be П's leading 0xD0. Split right after it.
        let p_start = bytes.iter().position(|&b| b == 0xD0).unwrap();
        let split = p_start + 1; // includes 0xD0 (leading byte) but not 0x9F (trailing)
        let chunk1 = &bytes[..split];
        let chunk2 = &bytes[split..];
        assert!(
            std::str::from_utf8(chunk1).is_err(),
            "test setup: chunk1 must be invalid UTF-8 (split mid Cyrillic char)"
        );

        let mut b: Vec<u8> = chunk1.to_vec();
        let s1 = drain_complete_frames(&mut b);
        // No \n\n yet, so nothing decoded, and no panic.
        assert_eq!(s1, "");

        b.extend_from_slice(chunk2);
        let s2 = drain_complete_frames(&mut b);
        // Now we have a complete frame ending in \n\n. Must decode cleanly.
        assert_eq!(s2, full);
        assert!(b.is_empty());
    }

    #[test]
    fn drain_handles_multiple_frames_in_one_chunk() {
        let mut b: Vec<u8> = b"data: a\n\ndata: b\n\ndata: c".to_vec();
        let s = drain_complete_frames(&mut b);
        assert_eq!(s, "data: a\n\ndata: b\n\n");
        assert_eq!(b, b"data: c");
    }

    #[test]
    fn drain_normalizes_crlf_sse_frames() {
        let mut bytes =
            b"event: message\r\ndata: {\"type\":\"response.completed\"}\r\n\r\n".to_vec();
        let text = drain_complete_frames(&mut bytes);
        assert_eq!(
            text,
            "event: message\ndata: {\"type\":\"response.completed\"}\n\n"
        );
        assert!(bytes.is_empty());
    }

    // ── Smoke check on build_request shape ──

    #[test]
    fn build_request_always_includes_system_prompt() {
        let msgs = build_request("", "ru", &[], None, None);
        // system + user
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "system");
        assert_eq!(msgs[1].role, "user");
        // Russian directive present
        if let MessageContent::Text(s) = &msgs[0].content {
            assert!(s.contains("русском"));
        } else {
            panic!("system message should be text");
        }
    }

    #[test]
    fn build_request_injects_kb_reference_for_named_term() {
        // A question naming a KB term (Exasol) pulls its entry into the system
        // prompt. Regression guard for the byte-cap bug: the Cyrillic Exasol
        // body is ~1.8 KB, so too small a cap silently dropped it.
        let msgs = build_request("", "ru", &[], None, Some("Что такое Exasol?"));
        if let MessageContent::Text(s) = &msgs[0].content {
            assert!(
                s.contains("Справка из базы знаний"),
                "KB reference block missing from system prompt"
            );
            assert!(s.contains("Exasol"), "Exasol entry not injected");
            assert!(
                s.contains("MPP") || s.contains("columnar"),
                "Exasol body not injected"
            );
        } else {
            panic!("system message should be text");
        }
        // A generic question naming no KB key must NOT inject a block (no noise).
        let plain = build_request("", "ru", &[], None, Some("zzqq xkcdq vmwpq blortz"));
        if let MessageContent::Text(s) = &plain[0].content {
            assert!(
                !s.contains("Справка из базы знаний"),
                "KB block wrongly injected for a generic question"
            );
        } else {
            panic!("system message should be text");
        }
    }

    // ── NEW: cost/pricing math ──

    #[test]
    fn cost_microcents_haiku_known_value() {
        // Haiku: $1/M input + $5/M output. 1M input + 1M output = $6 = 600M microcents.
        // microcents per token: input=100, output=500
        assert_eq!(
            cost_microcents("claude-haiku-4-5", 1_000_000, 1_000_000),
            600_000_000
        );
    }

    #[test]
    fn cost_microcents_sonnet_pricing() {
        // Sonnet: $3/M + $15/M. 100k+50k = 300k*3/M + 50k*15/M ≈ $0.3 + $0.75 = $1.05
        // microcents per token: input=300, output=1500
        let m = cost_microcents("claude-sonnet-4-6", 100_000, 50_000);
        assert_eq!(m, 100_000 * 300 + 50_000 * 1500);
        assert!((microcents_to_usd(m) - 1.05).abs() < 0.001);
    }

    #[test]
    fn gpt_5_2_pricing_and_endpoint_debug_are_safe() {
        assert_eq!(pricing_per_million("gpt-5.2"), (1.75, 14.0));
        let endpoint = AiEndpoint {
            protocol: AiProtocol::OpenAiResponses,
            base_url: "https://private.example/v1".into(),
            bearer: "super-secret-token".into(),
            model: "gpt-5.2".into(),
            reasoning_effort: None,
            is_local: false,
        };
        let debug = format!("{endpoint:?}");
        assert!(!debug.contains("private.example"));
        assert!(!debug.contains("super-secret-token"));
        assert!(debug.contains("has_credential: true"));

        let codex = AiEndpoint {
            protocol: AiProtocol::CodexSubscription,
            base_url: String::new(),
            bearer: String::new(),
            model: "gpt-safe".into(),
            reasoning_effort: Some("high".into()),
            is_local: false,
        };
        assert!(!codex.requires_bearer());
        assert!(codex.is_unmetered());
        assert!(codex.accepts_images());
    }

    #[test]
    fn cost_unknown_model_defaults_to_sonnet() {
        // Per pricing_per_million fallback.
        let m_known = cost_microcents("claude-sonnet-4-5", 1000, 1000);
        let m_unknown = cost_microcents("qwen-14b", 1000, 1000);
        assert_eq!(
            m_known, m_unknown,
            "unknown model should fall back to sonnet pricing"
        );
    }

    #[test]
    fn cost_zero_tokens_is_zero() {
        assert_eq!(cost_microcents("claude-haiku-4-5", 0, 0), 0);
        assert_eq!(
            microcents_to_usd(cost_microcents("claude-haiku-4-5", 0, 0)),
            0.0
        );
    }

    #[test]
    fn microcents_to_usd_boundaries() {
        assert_eq!(microcents_to_usd(0), 0.0);
        assert!((microcents_to_usd(50_000_000) - 0.5).abs() < 1e-12);
        assert!((microcents_to_usd(MICROCENTS_PER_USD as u64) - 1.0).abs() < 1e-12);
        // u64::MAX must not panic; the float view stays finite.
        assert!(microcents_to_usd(u64::MAX).is_finite());
    }

    #[test]
    fn cost_saturating_no_overflow() {
        // Max u64 input shouldn't panic.
        let m = cost_microcents("claude-opus-4-7", u64::MAX, u64::MAX);
        assert_eq!(m, u64::MAX, "should saturate, not panic");
    }

    // ── is_permanent_ai_error classifier (used by retry wrapper) ──

    #[test]
    fn permanent_error_400_no_retry() {
        // 400 = bad request payload (e.g. oversized prompt, malformed JSON).
        // Retrying won't fix the request — fail fast.
        assert!(is_permanent_ai_error("HTTP 400: invalid request"));
    }

    #[test]
    fn permanent_error_auth_no_retry() {
        // 401 = bad bearer token. 403 = forbidden / quota exceeded.
        // User must fix Settings → no retry.
        assert!(is_permanent_ai_error("HTTP 401: unauthorized"));
        assert!(is_permanent_ai_error("HTTP 403: forbidden"));
    }

    #[test]
    fn permanent_error_404_no_retry() {
        // 404 = endpoint missing (typo in ai_base_url) or model not found.
        // Will keep 404'ing on retry — fail fast.
        assert!(is_permanent_ai_error("HTTP 404: not found"));
    }

    #[test]
    fn permanent_error_413_no_retry() {
        // 413 = payload too large. Retry without changing payload pointless.
        assert!(is_permanent_ai_error("HTTP 413: request entity too large"));
    }

    #[test]
    fn transient_error_5xx_retries() {
        // Server-side problems — bridge restart, upstream Claude blip, etc.
        // Retry MAY succeed.
        assert!(!is_permanent_ai_error("HTTP 500: internal server error"));
        assert!(!is_permanent_ai_error("HTTP 502: bad gateway"));
        assert!(!is_permanent_ai_error("HTTP 503: service unavailable"));
        assert!(!is_permanent_ai_error("HTTP 504: gateway timeout"));
    }

    #[test]
    fn transient_error_429_retries() {
        // Rate limit — retry after exponential backoff usually clears it.
        // Note: NOT in the permanent list per the docstring (4xx EXCEPT 429).
        assert!(!is_permanent_ai_error("HTTP 429: rate limited"));
    }

    #[test]
    fn transient_network_errors_retry() {
        // Connection refused, timeout, DNS — all transient.
        assert!(!is_permanent_ai_error("Connection refused"));
        assert!(!is_permanent_ai_error("request timed out"));
        assert!(!is_permanent_ai_error("DNS resolution failed"));
        assert!(!is_permanent_ai_error("connection reset by peer"));
    }

    #[test]
    fn empty_error_does_not_match_permanent() {
        // Defensive: empty error string should NOT be classified as permanent
        // (otherwise we'd suppress retry for any error that gets stringified
        // to "").
        assert!(!is_permanent_ai_error(""));
    }

    #[test]
    fn build_request_attaches_screenshot_as_image_part() {
        let msgs = build_request(
            "",
            "ru",
            &["[System] что такое etcd?".to_string()],
            Some("data:image/jpeg;base64,XXX"),
            None,
        );
        if let MessageContent::Parts(parts) = &msgs[1].content {
            assert!(parts
                .iter()
                .any(|p| matches!(p, ContentPart::ImageUrl { .. })));
        } else {
            panic!("user content should be parts when screenshot attached");
        }
    }

    // ── Audit D4: provider finish_reason must survive the non-streaming path ──

    /// One-shot mock OpenAI-compatible server: answers the FIRST
    /// /chat/completions POST with `body`, then exits. Mirrors the bridge.rs
    /// tiny_http pattern (same dependency, no new test infra).
    fn serve_one_completion(body: &'static str) -> String {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        std::thread::spawn(move || {
            if let Ok(req) = server.recv() {
                let mut resp = tiny_http::Response::from_string(body);
                if let Ok(h) =
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                {
                    resp = resp.with_header(h);
                }
                let _ = req.respond(resp);
            }
        });
        url
    }

    #[derive(Debug)]
    struct CapturedRequest {
        path: String,
        headers: Vec<(String, String)>,
        body: String,
    }

    fn serve_one_capture(
        response_body: &'static str,
        content_type: &'static str,
    ) -> (String, std::sync::mpsc::Receiver<CapturedRequest>) {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            if let Ok(mut req) = server.recv() {
                let path = req.url().to_string();
                let headers = req
                    .headers()
                    .iter()
                    .map(|header| (header.field.to_string(), header.value.as_str().to_string()))
                    .collect();
                let mut body = String::new();
                let _ = std::io::Read::read_to_string(req.as_reader(), &mut body);
                let _ = tx.send(CapturedRequest {
                    path,
                    headers,
                    body,
                });
                let mut response = tiny_http::Response::from_string(response_body);
                if let Ok(header) =
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes())
                {
                    response = response.with_header(header);
                }
                let _ = req.respond(response);
            }
        });
        (url, rx)
    }

    fn header<'a>(captured: &'a CapturedRequest, name: &str) -> Option<&'a str> {
        captured
            .headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    #[tokio::test]
    async fn direct_openai_uses_responses_contract() {
        let (url, captured) = serve_one_capture(
            r#"{"status":"completed","output":[{"content":[{"type":"output_text","text":"ok"}]}],"usage":{"input_tokens":4,"output_tokens":1}}"#,
            "application/json",
        );
        let endpoint = AiEndpoint {
            protocol: AiProtocol::OpenAiResponses,
            base_url: url,
            bearer: "openai-secret".into(),
            model: "gpt-test".into(),
            reasoning_effort: None,
            is_local: false,
        };
        let (text, usage) = complete_with_usage_endpoint(
            &endpoint,
            vec![ChatMessage {
                role: "user".into(),
                content: MessageContent::Text("hello".into()),
            }],
            42,
        )
        .await
        .unwrap();
        assert_eq!(text, "ok");
        assert_eq!(usage.output, 1);
        let request = captured.recv().unwrap();
        assert_eq!(request.path, "/responses");
        assert_eq!(
            header(&request, "authorization"),
            Some("Bearer openai-secret")
        );
        let body: Value = serde_json::from_str(&request.body).unwrap();
        assert_eq!(body["max_output_tokens"], 42);
        assert!(body.get("messages").is_none());
    }

    #[tokio::test]
    async fn direct_anthropic_uses_messages_contract_and_headers() {
        let (url, captured) = serve_one_capture(
            r#"{"content":[{"type":"text","text":"ok"}],"stop_reason":"end_turn","usage":{"input_tokens":5,"output_tokens":1}}"#,
            "application/json",
        );
        let endpoint = AiEndpoint {
            protocol: AiProtocol::AnthropicMessages,
            base_url: url,
            bearer: "anthropic-secret".into(),
            model: "claude-test".into(),
            reasoning_effort: None,
            is_local: false,
        };
        let (text, usage) = complete_with_usage_endpoint(
            &endpoint,
            vec![ChatMessage {
                role: "user".into(),
                content: MessageContent::Text("hello".into()),
            }],
            43,
        )
        .await
        .unwrap();
        assert_eq!(text, "ok");
        assert_eq!(usage.finish_reason, "end_turn");
        let request = captured.recv().unwrap();
        assert_eq!(request.path, "/messages");
        assert_eq!(header(&request, "x-api-key"), Some("anthropic-secret"));
        assert_eq!(header(&request, "anthropic-version"), Some("2023-06-01"));
        assert!(header(&request, "authorization").is_none());
        let body: Value = serde_json::from_str(&request.body).unwrap();
        assert_eq!(body["max_tokens"], 43);
    }

    #[tokio::test]
    async fn openai_finish_reason_is_terminal_without_optional_metrics() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4_096];
            let _ = std::io::Read::read(&mut stream, &mut request);
            let frame = b"data: {\"choices\":[{\"delta\":{\"content\":\"hi\"},\"finish_reason\":\"stop\"}]}\n\n";
            let headers = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: keep-alive\r\n\r\n";
            std::io::Write::write_all(&mut stream, headers).unwrap();
            std::io::Write::write_all(&mut stream, format!("{:X}\r\n", frame.len()).as_bytes())
                .unwrap();
            std::io::Write::write_all(&mut stream, frame).unwrap();
            std::io::Write::write_all(&mut stream, b"\r\n").unwrap();
            std::io::Write::flush(&mut stream).unwrap();
            std::thread::sleep(std::time::Duration::from_secs(5));
        });

        let mut rx = stream_chat(url, String::new(), "model".into(), Vec::new(), 8);
        assert!(matches!(rx.recv().await, Some(AiEvent::Delta { text }) if text == "hi"));
        let done = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
            .await
            .expect("finish_reason must not wait for optional telemetry");
        assert!(matches!(done, Some(AiEvent::Done { reason }) if reason == "stop"));
        let terminal = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv())
            .await
            .expect("stream producer should stop after finish_reason");
        assert!(
            terminal.is_none(),
            "finish_reason must emit exactly one terminal event"
        );
    }

    #[tokio::test]
    async fn native_streams_emit_delta_and_terminal_event() {
        for (protocol, body) in [
            (
                AiProtocol::OpenAiResponses,
                "data: {\"type\":\"response.created\",\"response\":{\"id\":\"r1\"}}\n\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"hi\"}\n\ndata: {\"type\":\"response.completed\"}\n\n",
            ),
            (
                AiProtocol::AnthropicMessages,
                "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m1\"}}\n\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}}\n\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\n",
            ),
        ] {
            let (url, _captured) = serve_one_capture(body, "text/event-stream");
            let mut rx = stream_chat_endpoint(
                AiEndpoint {
                    protocol,
                    base_url: url,
                    bearer: "secret".into(),
                    model: "model".into(),
                    reasoning_effort: None,
                    is_local: false,
                },
                vec![ChatMessage {
                    role: "user".into(),
                    content: MessageContent::Text("hello".into()),
                }],
                8,
            );
            assert!(matches!(rx.recv().await, Some(AiEvent::Start { .. })));
            assert!(matches!(rx.recv().await, Some(AiEvent::Delta { text }) if text == "hi"));
            assert!(matches!(rx.recv().await, Some(AiEvent::Done { .. })));
        }
    }

    /// A provider reporting `finish_reason: "length"` (answer truncated by
    /// max_tokens) must surface the REAL reason to callers — the non-streaming
    /// journaling sites (reask_last, manual_spawn_tile, auto_tile) write
    /// `usage.finish_reason` verbatim into JournalEvent::AiResponse.
    #[tokio::test]
    async fn complete_with_usage_surfaces_provider_length_finish_reason() {
        let url = serve_one_completion(
            r#"{"choices":[{"message":{"content":"truncated answer"},"finish_reason":"length"}],"usage":{"prompt_tokens":11,"completion_tokens":7}}"#,
        );
        let (text, usage) = complete_with_usage(&url, "", "mock-model", Vec::new(), 16)
            .await
            .unwrap();
        assert_eq!(text, "truncated answer");
        assert_eq!(usage.input, 11);
        assert_eq!(usage.output, 7);
        assert_eq!(
            usage.finish_reason, "length",
            "the provider's real finish_reason must reach the caller, not a hardcoded stop"
        );
    }

    /// When the provider omits finish_reason entirely, fall back to "stop"
    /// (the value every non-streaming site journaled before) — never empty.
    #[tokio::test]
    async fn complete_with_usage_defaults_finish_reason_to_stop_when_absent() {
        let url = serve_one_completion(
            r#"{"choices":[{"message":{"content":"plain answer"}}],"usage":{"prompt_tokens":3,"completion_tokens":2}}"#,
        );
        let (_text, usage) = complete_with_usage(&url, "", "mock-model", Vec::new(), 16)
            .await
            .unwrap();
        assert_eq!(usage.finish_reason, "stop");
    }

    /// Deep lock (v0.37): while active, EVERY managed-local sender refuses
    /// instantly with the marker error — no network, no retry, no hang. The
    /// guard fires BEFORE any transport, so these never touch the wire. A
    /// NON-managed URL must bypass the guard entirely. One test on purpose:
    /// the process-wide flag would race across parallel #[tokio::test]s.
    #[tokio::test]
    async fn deep_lock_guard_refuses_every_managed_sender() {
        crate::deep_lock::set_deep_lock_active(true);
        let managed = crate::local_ai::LLAMA_BASE_URL.to_string();

        let err = test_connection(managed.clone(), String::new(), "m".to_string())
            .await
            .unwrap_err();
        assert!(crate::deep_lock::is_blocked_error(&err.to_string()));

        let err = list_models(&managed, "").await.unwrap_err();
        assert!(crate::deep_lock::is_blocked_error(&err.to_string()));

        let err = complete(&managed, "", "m", Vec::new(), 8)
            .await
            .unwrap_err();
        assert!(crate::deep_lock::is_blocked_error(&err.to_string()));

        let err = complete_with_usage(&managed, "", "m", Vec::new(), 8)
            .await
            .unwrap_err();
        assert!(crate::deep_lock::is_blocked_error(&err.to_string()));

        let err = count_chat_tokens(&managed, "", "m", &[]).await.unwrap_err();
        assert!(crate::deep_lock::is_blocked_error(&err.to_string()));

        // Streaming surfaces the guard as an Error event (never a hang).
        let mut rx = stream_chat(
            managed.clone(),
            String::new(),
            "m".to_string(),
            Vec::new(),
            8,
        );
        match rx.recv().await {
            Some(AiEvent::Error { message }) => {
                assert!(crate::deep_lock::is_blocked_error(&message));
            }
            other => panic!("expected a blocked Error event, got {other:?}"),
        }

        // Scoped guard: a non-managed URL bypasses it even while locked. It
        // fails on TRANSPORT here (bound-then-dropped loopback listener),
        // which is the proof the guard let the request through.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let foreign = format!("http://{}/v1", listener.local_addr().unwrap());
        drop(listener);
        let err = list_models(&foreign, "").await.unwrap_err();
        assert!(
            !crate::deep_lock::is_blocked_error(&err.to_string()),
            "non-managed endpoints must bypass the deep-lock guard"
        );

        crate::deep_lock::set_deep_lock_active(false);
    }
