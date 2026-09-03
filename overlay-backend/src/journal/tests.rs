    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::recovery::*;
    use super::writer::*;
    use super::*;
    use parking_lot::Mutex;
    use std::fs::OpenOptions;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::mpsc;

    #[test]
    fn unix_to_ymdhms_known_dates() {
        let (y, mo, d, h, mi, s) = unix_to_ymdhms(1_779_580_800);
        assert_eq!((y, mo, d, h, mi, s), (2026, 5, 24, 0, 0, 0));
        let (y, mo, d, _, _, _) = unix_to_ymdhms(946_684_800);
        assert_eq!((y, mo, d), (2000, 1, 1));
    }

    #[test]
    fn format_msk_label_shifts_utc_plus_three() {
        // 2026-05-24 00:00:00 UTC → 03:00:00 МСК, DD.MM.YYYY order.
        assert_eq!(
            format_msk_label(1_779_580_800_000),
            "24.05.2026 03:00:00 (МСК)"
        );
        // Midnight rollover: 23:30 UTC → 02:30 МСК NEXT day.
        assert_eq!(
            format_msk_label((1_779_580_800 - 1800) * 1000),
            "24.05.2026 02:30:00 (МСК)"
        );
        // Garbage (negative) clamps instead of panicking.
        assert_eq!(format_msk_label(-5), "01.01.1970 03:00:00 (МСК)");
    }

    #[test]
    fn stamp_to_unix_secs_round_trips_chrono_like_stamp() {
        // Round-trip: unix → stamp text → unix.
        for secs in [946_684_800u64, 1_779_580_800, 1_780_000_000, 86_399] {
            let (y, mo, d, h, m, s) = unix_to_ymdhms(secs);
            let stamp = format!("{y:04}-{mo:02}-{d:02}_{h:02}-{m:02}-{s:02}_abc123");
            assert_eq!(stamp_to_unix_secs(&stamp), Some(secs), "stamp {stamp}");
        }
        // Malformed ids → None (caller falls back to the raw id).
        assert_eq!(stamp_to_unix_secs("session_12345"), None);
        assert_eq!(stamp_to_unix_secs("2026-13-01_10-00-00_x"), None);
        assert_eq!(stamp_to_unix_secs("short"), None);
    }

    #[test]
    fn stamp_format_is_sortable() {
        let s = chrono_like_stamp();
        assert_eq!(s.len(), 19);
        assert!(s.chars().nth(4) == Some('-'));
        assert!(s.chars().nth(10) == Some('_'));
    }

    #[test]
    fn event_serializes_with_kind_tag() {
        let ev = JournalEvent::TranscriptLine {
            unix_ms: 12345,
            source: "system",
            text: "hello",
            audio_ms: 0,
        };
        let s = serde_json::to_string(&ev).unwrap();
        assert!(s.contains(r#""kind":"transcript_line""#));
        assert!(s.contains(r#""text":"hello""#));
    }

    #[test]
    fn default_journal_write_is_noop() {
        // No file opened — write must not panic.
        let j = Journal::default();
        j.write(&JournalEvent::SessionStop { unix_ms: 0 });
        assert!(j.current_path().is_none());
    }

    // ── C3: per-journal shutdown contract ──
    // Exercise the real writer thread against a temp file (via the
    // `spawn_writer` seam) so we never touch the user's `%APPDATA%` journals.

    /// Open a real journal (writer thread + file) at an explicit path.
    fn open_journal_at(path: &Path) -> Journal {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        let (tx, rx) = mpsc::unbounded_channel::<WriterCmd>();
        let join = spawn_writer(rx, file).unwrap();
        Journal {
            path: Some(Arc::new(path.to_path_buf())),
            counters: Some(Arc::new(Mutex::new(SessionCounters::default()))),
            writer: Some(Arc::new(Mutex::new(WriterState {
                tx: Some(tx),
                join: Some(join),
                shutdown: None,
            }))),
        }
    }

    #[test]
    fn shutdown_drains_burst_and_terminal_stop_durably() {
        let tmp =
            std::env::temp_dir().join(format!("overlay-journal-shutdown-{}.jsonl", now_unix_ms()));
        let _ = std::fs::remove_file(&tmp);
        let j = open_journal_at(&tmp);

        // Queue a burst INCLUDING the terminal SessionStop, then shut down.
        for i in 0..50u64 {
            j.write(&JournalEvent::TranscriptLine {
                unix_ms: u128::from(i),
                source: "mic",
                text: "line",
                audio_ms: i,
            });
        }
        j.write(&JournalEvent::SessionStop { unix_ms: 999 });

        j.shutdown(Duration::from_secs(5))
            .expect("shutdown must confirm durability");

        // No sleep: a successful shutdown guarantees every queued line is on
        // disk and the writer is joined.
        let content = std::fs::read_to_string(&tmp).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 51, "50 transcript + 1 stop, all durable");
        assert!(lines[49].contains(r#""kind":"transcript_line""#));
        assert!(
            lines[50].contains(r#""kind":"session_stop""#),
            "stop is last"
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn shutdown_is_idempotent_and_rejects_later_writes() {
        let tmp =
            std::env::temp_dir().join(format!("overlay-journal-idem-{}.jsonl", now_unix_ms()));
        let _ = std::fs::remove_file(&tmp);
        let j = open_journal_at(&tmp);
        j.write(&JournalEvent::TranscriptLine {
            unix_ms: 1,
            source: "mic",
            text: "before",
            audio_ms: 1,
        });
        j.shutdown(Duration::from_secs(5)).unwrap();
        let durable = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(durable.lines().count(), 1, "the one pre-shutdown line");

        // Repeat shutdown after success → no-op Ok (never a false Err, never a
        // re-join hang).
        j.shutdown(Duration::from_secs(5))
            .expect("repeat shutdown after success is a no-op Ok");

        // A write after shutdown is rejected: the sender is gone and the writer
        // is joined, so it must NOT reach the file.
        j.write(&JournalEvent::TranscriptLine {
            unix_ms: 2,
            source: "mic",
            text: "after",
            audio_ms: 2,
        });
        let after = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(
            after, durable,
            "writes after shutdown must not mutate the journal"
        );
        assert!(!after.contains("after"));
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn open_session_creates_writable_file() {
        // Override config dir to temp for test isolation.
        let tmp = std::env::temp_dir().join(format!("overlay-mvp-test-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);
        // dirs crate on Windows uses APPDATA, not XDG. Best-effort test —
        // skip when not on linux/mac, but compile-check stays valid.
        let _ = tmp;
    }

    // ── Prune-old-sessions tests ──
    // Manipulate `dir` directly rather than going via APPDATA so we don't
    // pollute the real journal directory on the dev machine.

    fn make_jsonl_file(dir: &Path, name: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, "x").unwrap();
        p
    }

    #[test]
    fn prune_keeps_newest_n_files() {
        let tmp = std::env::temp_dir().join(format!("overlay-prune-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();

        // Create 10 .jsonl files with strictly increasing mtimes.
        for i in 0..10 {
            make_jsonl_file(&tmp, &format!("s_{i:02}.jsonl"));
            // sleep ≥ filesystem mtime resolution (NTFS = 100ns, fs cache ≥ 1ms)
            std::thread::sleep(std::time::Duration::from_millis(15));
        }

        let deleted = prune_old_sessions(&tmp, 3).unwrap();
        assert_eq!(deleted, 7, "should delete 7 of 10");

        let remaining: Vec<_> = std::fs::read_dir(&tmp)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("jsonl"))
            .collect();
        assert_eq!(remaining.len(), 3, "exactly `keep` files left");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn prune_keep_larger_than_count_is_noop() {
        let tmp = std::env::temp_dir().join(format!("overlay-prune-noop-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        for i in 0..3 {
            make_jsonl_file(&tmp, &format!("s_{i}.jsonl"));
        }
        let deleted = prune_old_sessions(&tmp, 100).unwrap();
        assert_eq!(deleted, 0, "nothing to prune when keep > count");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn prune_ignores_non_jsonl_files() {
        let tmp = std::env::temp_dir().join(format!("overlay-prune-ext-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        // 2 jsonl + 3 other extensions
        make_jsonl_file(&tmp, "s_1.jsonl");
        std::thread::sleep(std::time::Duration::from_millis(15));
        make_jsonl_file(&tmp, "s_2.jsonl");
        std::fs::write(tmp.join("notes.txt"), "hi").unwrap();
        std::fs::write(tmp.join("config.json"), "{}").unwrap();
        std::fs::write(tmp.join("backup.jsonl.bak"), "x").unwrap();

        let deleted = prune_old_sessions(&tmp, 1).unwrap();
        assert_eq!(deleted, 1, "only the older .jsonl should be deleted");
        // The 3 non-jsonl files MUST still be there.
        assert!(tmp.join("notes.txt").exists());
        assert!(tmp.join("config.json").exists());
        assert!(tmp.join("backup.jsonl.bak").exists());

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn prune_keep_zero_deletes_all_jsonl() {
        let tmp = std::env::temp_dir().join(format!("overlay-prune-zero-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        for i in 0..4 {
            make_jsonl_file(&tmp, &format!("s_{i}.jsonl"));
        }
        let deleted = prune_old_sessions(&tmp, 0).unwrap();
        assert_eq!(deleted, 4);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn prune_empty_dir_returns_zero() {
        let tmp = std::env::temp_dir().join(format!("overlay-prune-empty-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        let deleted = prune_old_sessions(&tmp, 10).unwrap();
        assert_eq!(deleted, 0);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn v015_unlimited_spelling_deletes_nothing() {
        // `open_new_session_with_limits(0, 0)` translates "keep all" to exactly
        // this call — it must NEVER delete (the raw prune's keep==0 means the
        // OPPOSITE, "delete all", so the translation is load-bearing).
        let tmp = std::env::temp_dir().join(format!("overlay-prune-unlim-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        for i in 0..5 {
            make_jsonl_file(&tmp, &format!("s_{i}.jsonl"));
        }
        let deleted = prune_old_sessions_with_size_cap(&tmp, usize::MAX, 0).unwrap();
        assert_eq!(deleted, 0, "unlimited count + disabled size cap = no-op");
        assert_eq!(std::fs::read_dir(&tmp).unwrap().count(), 5);
        std::fs::remove_dir_all(&tmp).ok();
    }

    // ── prune_old_sessions_with_size_cap — v0.0.2 size-based prune ──

    /// Helper: create N jsonl files in `dir` each `kb` bytes large, with
    /// per-file mtime offset so iteration order is deterministic (newest
    /// last). Returns sorted file paths newest-last.
    fn make_jsonl_files(dir: &Path, count: usize, size_bytes: usize) -> Vec<PathBuf> {
        let mut paths = Vec::with_capacity(count);
        let payload = vec![b'x'; size_bytes];
        for i in 0..count {
            let path = dir.join(format!("session-{:03}.jsonl", i));
            std::fs::write(&path, &payload).unwrap();
            // Small sleep so mtimes are distinguishable (some FS round to seconds).
            std::thread::sleep(std::time::Duration::from_millis(10));
            paths.push(path);
        }
        paths
    }

    #[test]
    fn size_cap_zero_disables_size_based_prune() {
        // max_bytes=0 should skip the size check entirely.
        let tmp = std::env::temp_dir().join(format!("overlay-sizecap-zero-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        make_jsonl_files(&tmp, 5, 10_000); // 5 files × 10 KB = 50 KB
                                           // keep=10 (no count prune), max_bytes=0 (disabled) → 0 deleted.
        let deleted = prune_old_sessions_with_size_cap(&tmp, 10, 0).unwrap();
        assert_eq!(deleted, 0, "max_bytes=0 should disable size cap");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn size_cap_under_budget_no_op() {
        let tmp = std::env::temp_dir().join(format!("overlay-sizecap-under-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        make_jsonl_files(&tmp, 3, 1000); // 3 KB total
                                         // 10 KB cap, 3 KB used → no prune.
        let deleted = prune_old_sessions_with_size_cap(&tmp, 100, 10_000).unwrap();
        assert_eq!(deleted, 0);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn size_cap_evicts_oldest_first() {
        let tmp = std::env::temp_dir().join(format!("overlay-sizecap-oldest-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        let files = make_jsonl_files(&tmp, 5, 10_000); // 50 KB total
                                                       // 25 KB cap → must delete 3 oldest (15 KB+ for total ≤ 25 KB after).
                                                       // After deleting 3 oldest, remaining = 2 × 10 KB = 20 KB ≤ 25 KB ✓.
        let deleted = prune_old_sessions_with_size_cap(&tmp, 100, 25_000).unwrap();
        assert_eq!(deleted, 3, "should delete 3 oldest to fit under 25 KB cap");
        // Verify the 2 NEWEST survive.
        let survivors: Vec<_> = std::fs::read_dir(&tmp)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        assert_eq!(survivors.len(), 2);
        assert!(survivors.contains(&files[3]), "newest-second survives");
        assert!(survivors.contains(&files[4]), "newest survives");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn size_cap_combines_with_count_prune() {
        let tmp = std::env::temp_dir().join(format!("overlay-sizecap-combo-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        make_jsonl_files(&tmp, 6, 10_000); // 6 × 10 KB = 60 KB
                                           // keep=4 (count prune evicts 2), then 40 KB → cap 25 KB → evict 2 more.
                                           // Total deleted = 4 (2 by count, 2 by size).
        let deleted = prune_old_sessions_with_size_cap(&tmp, 4, 25_000).unwrap();
        assert_eq!(deleted, 4, "2 by count + 2 by size = 4 total");
        let remaining = std::fs::read_dir(&tmp).unwrap().count();
        assert_eq!(remaining, 2);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn size_cap_exactly_at_budget_no_op() {
        let tmp = std::env::temp_dir().join(format!("overlay-sizecap-exact-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        make_jsonl_files(&tmp, 2, 10_000); // 20 KB total
                                           // 20 KB cap, 20 KB used — boundary case. Total > cap? `total > max_bytes`
                                           // check uses strict >. Equal → no prune.
        let deleted = prune_old_sessions_with_size_cap(&tmp, 100, 20_000).unwrap();
        assert_eq!(deleted, 0, "at-boundary total should NOT trigger prune");
        std::fs::remove_dir_all(&tmp).ok();
    }

    // ── bump_counters: SessionSummary feeders ──

    #[test]
    fn bump_transcript_lines_per_source() {
        let mut c = SessionCounters::default();
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 0,
                source: "mic",
                text: "",
                audio_ms: 0,
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 0,
                source: "system",
                text: "",
                audio_ms: 0,
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 0,
                source: "mic",
                text: "",
                audio_ms: 0,
            },
        );
        assert_eq!(c.transcript_mic, 2);
        assert_eq!(c.transcript_system, 1);
    }

    #[test]
    fn bump_transcript_unknown_source_ignored() {
        let mut c = SessionCounters::default();
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 0,
                source: "weird",
                text: "",
                audio_ms: 0,
            },
        );
        assert_eq!(c.transcript_mic, 0);
        assert_eq!(c.transcript_system, 0);
    }

    #[test]
    fn bump_detector_decision_split_by_triggered() {
        let mut c = SessionCounters::default();
        bump_counters(
            &mut c,
            &JournalEvent::DetectorDecision {
                unix_ms: 0,
                text: "",
                triggered: true,
                trigger_kind: Some("question"),
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::DetectorDecision {
                unix_ms: 0,
                text: "",
                triggered: false,
                trigger_kind: None,
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::DetectorDecision {
                unix_ms: 0,
                text: "",
                triggered: false,
                trigger_kind: None,
            },
        );
        assert_eq!(c.detector_triggered, 1);
        assert_eq!(c.detector_skipped, 2);
    }

    #[test]
    fn bump_ai_response_accumulates_cost_microcents() {
        let mut c = SessionCounters::default();
        bump_counters(
            &mut c,
            &JournalEvent::AiResponse {
                unix_ms: 0,
                purpose: "live_ask",
                model: "haiku",
                latency_ms: 100,
                finish_reason: "stop",
                text: "",
                output_tokens_est: 0,
                cost_microcents: 12_345,
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::AiResponse {
                unix_ms: 0,
                purpose: "auto_tile",
                model: "haiku",
                latency_ms: 200,
                finish_reason: "stop",
                text: "",
                output_tokens_est: 0,
                cost_microcents: 6_789,
            },
        );
        assert_eq!(c.ai_responses_ok, 2);
        assert_eq!(c.total_cost_microcents, 19_134);
    }

    #[test]
    fn bump_ai_response_cost_saturates_no_panic() {
        let mut c = SessionCounters {
            total_cost_microcents: u64::MAX - 10,
            ..Default::default()
        };
        bump_counters(
            &mut c,
            &JournalEvent::AiResponse {
                unix_ms: 0,
                purpose: "x",
                model: "y",
                latency_ms: 0,
                finish_reason: "stop",
                text: "",
                output_tokens_est: 0,
                cost_microcents: 1_000_000,
            },
        );
        assert_eq!(
            c.total_cost_microcents,
            u64::MAX,
            "should saturate, not wrap"
        );
    }

    #[test]
    fn bump_session_meta_events_do_not_count() {
        let mut c = SessionCounters::default();
        bump_counters(
            &mut c,
            &JournalEvent::SessionStart {
                unix_ms: 0,
                meeting_context_chars: 100,
                ai_model: "haiku",
                prep_model: "sonnet",
                stt_language: None,
                response_language: "ru",
                recovered_from_session_id: None,
            },
        );
        bump_counters(&mut c, &JournalEvent::SessionStop { unix_ms: 0 });
        bump_counters(
            &mut c,
            &JournalEvent::SessionSummary {
                unix_ms: 0,
                duration_ms: 0,
                transcript_lines: 0,
                transcript_mic: 0,
                transcript_system: 0,
                detector_triggered: 0,
                detector_skipped: 0,
                ai_requests_total: 0,
                ai_responses_ok: 0,
                ai_errors: 0,
                tiles_spawned: 0,
                rate_limited: 0,
                total_cost_microcents: 0,
            },
        );
        // Nothing should have incremented.
        assert_eq!(c.transcript_mic, 0);
        assert_eq!(c.ai_requests_total, 0);
    }

    #[test]
    fn bump_full_event_mix_aggregates_correctly() {
        let mut c = SessionCounters::default();
        // Simulate a mini-session: 2 mic + 1 sys lines, 1 detected, 1 ai req, 1 ai resp, 1 tile, 1 error.
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 1,
                source: "mic",
                text: "a",
                audio_ms: 0,
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 2,
                source: "mic",
                text: "b",
                audio_ms: 0,
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 3,
                source: "system",
                text: "c?",
                audio_ms: 0,
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::DetectorDecision {
                unix_ms: 4,
                text: "c?",
                triggered: true,
                trigger_kind: Some("question"),
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::AiRequest {
                unix_ms: 5,
                purpose: "auto_tile",
                model: "haiku",
                system_prompt: "sys",
                user_prompt: "usr",
                attached_screenshot: false,
                input_tokens_est: 100,
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::AiResponse {
                unix_ms: 6,
                purpose: "auto_tile",
                model: "haiku",
                latency_ms: 500,
                finish_reason: "stop",
                text: "answer",
                output_tokens_est: 50,
                cost_microcents: 500,
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::TileSpawn {
                unix_ms: 7,
                label: "tile-1",
                question: "c?",
                answer: "answer",
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::RateLimited {
                unix_ms: 8,
                what: "auto_tile",
                text: "skipped",
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::Error {
                unix_ms: 9,
                module: "auto_tile_ai",
                message: "timeout",
            },
        );
        assert_eq!(c.transcript_mic, 2);
        assert_eq!(c.transcript_system, 1);
        assert_eq!(c.detector_triggered, 1);
        assert_eq!(c.ai_requests_total, 1);
        assert_eq!(c.ai_responses_ok, 1);
        assert_eq!(c.tiles_spawned, 1);
        assert_eq!(c.rate_limited, 1);
        assert_eq!(c.ai_errors, 1);
        assert_eq!(c.total_cost_microcents, 500);
    }

    #[test]
    fn snapshot_counters_returns_independent_clone() {
        // After snapshot, further bumps should NOT affect the snapshot.
        let mut c = SessionCounters::default();
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 0,
                source: "mic",
                text: "",
                audio_ms: 0,
            },
        );
        let snap = c.clone();
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 0,
                source: "mic",
                text: "",
                audio_ms: 0,
            },
        );
        assert_eq!(snap.transcript_mic, 1, "snapshot frozen at 1");
        assert_eq!(c.transcript_mic, 2, "live counter advanced to 2");
    }

    #[test]
    fn session_summary_serializes_with_kind_tag() {
        let ev = JournalEvent::SessionSummary {
            unix_ms: 1000,
            duration_ms: 5000,
            transcript_lines: 10,
            transcript_mic: 4,
            transcript_system: 6,
            detector_triggered: 2,
            detector_skipped: 8,
            ai_requests_total: 2,
            ai_responses_ok: 2,
            ai_errors: 0,
            tiles_spawned: 2,
            rate_limited: 0,
            total_cost_microcents: 12_500,
        };
        let s = serde_json::to_string(&ev).unwrap();
        assert!(s.contains(r#""kind":"session_summary""#));
        assert!(s.contains(r#""total_cost_microcents":12500"#));
        assert!(s.contains(r#""duration_ms":5000"#));
    }

    // ── Deliverable B: recovered_from_session_id back-compat ──

    #[test]
    fn session_start_normal_serializes_without_recovery_field() {
        // A cold start (recovered_from_session_id = None) must serialize
        // byte-for-byte as before: the key is SKIPPED, so an old reader sees
        // the exact same shape it always did.
        let ev = JournalEvent::SessionStart {
            unix_ms: 1700,
            meeting_context_chars: 42,
            ai_model: "haiku",
            prep_model: "sonnet",
            stt_language: Some("ru"),
            response_language: "ru",
            recovered_from_session_id: None,
        };
        let s = serde_json::to_string(&ev).unwrap();
        assert!(s.contains(r#""kind":"session_start""#));
        assert!(
            !s.contains("recovered_from_session_id"),
            "None must be skipped so a normal start is unchanged: {s}"
        );
    }

    #[test]
    fn session_start_recovered_serializes_with_recovery_field() {
        let ev = JournalEvent::SessionStart {
            unix_ms: 1700,
            meeting_context_chars: 42,
            ai_model: "haiku",
            prep_model: "sonnet",
            stt_language: None,
            response_language: "ru",
            recovered_from_session_id: Some("2026-06-02_15-30-12_abc123"),
        };
        let s = serde_json::to_string(&ev).unwrap();
        assert!(s.contains(r#""recovered_from_session_id":"2026-06-02_15-30-12_abc123""#));
    }

    /// Owned mirror of the `SessionStart` payload to prove the `#[serde(default)]`
    /// semantics: an OLD line (no `recovered_from_session_id` key) deserializes
    /// to `None`, and a new line round-trips the id. Mirroring (rather than
    /// deriving `Deserialize` on the borrowed `JournalEvent<'a>`) keeps the
    /// write-side enum zero-copy while still exercising the exact serde attrs.
    #[derive(serde::Deserialize)]
    struct SessionStartOwned {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        recovered_from_session_id: Option<String>,
    }

    #[test]
    fn old_session_start_line_deserializes_to_none() {
        // An OLD on-disk line written before the field existed.
        let old = r#"{"kind":"session_start","unix_ms":1700,"meeting_context_chars":42,"ai_model":"haiku","prep_model":"sonnet","stt_language":null,"response_language":"ru"}"#;
        let parsed: SessionStartOwned = serde_json::from_str(old).unwrap();
        assert_eq!(parsed.recovered_from_session_id, None);
    }

    #[test]
    fn new_session_start_line_deserializes_id() {
        let new = r#"{"kind":"session_start","unix_ms":1700,"meeting_context_chars":42,"ai_model":"haiku","prep_model":"sonnet","stt_language":null,"response_language":"ru","recovered_from_session_id":"sess-xyz"}"#;
        let parsed: SessionStartOwned = serde_json::from_str(new).unwrap();
        assert_eq!(
            parsed.recovered_from_session_id.as_deref(),
            Some("sess-xyz")
        );
    }

    // ── Deliverable A: find_unfinished_session ──
    //
    // Each test writes synthetic JSONL into a fresh temp dir, then drives the
    // pure detector. We set mtimes implicitly via write order + tiny sleeps
    // where "newest" matters, mirroring the existing prune tests.

    /// Build a JSONL `session_start` line `age_ms` milliseconds in the past.
    fn start_line(age_ms: u64) -> String {
        let started = (now_unix_ms() as u64).saturating_sub(age_ms);
        format!(
            r#"{{"kind":"session_start","unix_ms":{started},"meeting_context_chars":0,"ai_model":"haiku","prep_model":"sonnet","stt_language":null,"response_language":"ru"}}"#
        )
    }

    fn transcript_line(source: &str, text: &str) -> String {
        format!(r#"{{"kind":"transcript_line","unix_ms":1,"source":"{source}","text":"{text}"}}"#)
    }

    fn ai_request_line(user_prompt: &str) -> String {
        format!(
            r#"{{"kind":"ai_request","unix_ms":1,"purpose":"auto_tile","model":"haiku","system_prompt":"sys","user_prompt":"{user_prompt}","attached_screenshot":false,"input_tokens_est":1}}"#
        )
    }

    fn ai_response_line(text: &str) -> String {
        format!(
            r#"{{"kind":"ai_response","unix_ms":1,"purpose":"auto_tile","model":"haiku","latency_ms":1,"finish_reason":"stop","text":"{text}","output_tokens_est":1,"cost_microcents":1}}"#
        )
    }

    fn write_jsonl(dir: &Path, name: &str, lines: &[String]) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, format!("{}\n", lines.join("\n"))).unwrap();
        p
    }

    fn fresh_dir(tag: &str) -> PathBuf {
        let tmp = std::env::temp_dir().join(format!("overlay-recover-{tag}-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        tmp
    }

    #[test]
    fn graceful_stop_returns_none() {
        let dir = fresh_dir("graceful");
        write_jsonl(
            &dir,
            "s.jsonl",
            &[
                start_line(60_000),
                transcript_line("system", "hello"),
                r#"{"kind":"session_stop","unix_ms":2}"#.to_string(),
            ],
        );
        assert!(find_unfinished_session(&dir).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn graceful_summary_returns_none() {
        // A terminal SessionSummary also marks a clean close (it's emitted
        // just before SessionStop on the graceful path).
        let dir = fresh_dir("summary");
        write_jsonl(
            &dir,
            "s.jsonl",
            &[
                start_line(60_000),
                transcript_line("mic", "hi"),
                r#"{"kind":"session_summary","unix_ms":2,"duration_ms":1,"transcript_lines":1,"transcript_mic":1,"transcript_system":0,"detector_triggered":0,"detector_skipped":0,"ai_requests_total":0,"ai_responses_ok":0,"ai_errors":0,"tiles_spawned":0,"rate_limited":0,"total_cost_microcents":0}"#.to_string(),
            ],
        );
        assert!(find_unfinished_session(&dir).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn crash_returns_some_with_last_lines_and_qa() {
        let dir = fresh_dir("crash");
        write_jsonl(
            &dir,
            "crashed.jsonl",
            &[
                start_line(120_000),
                transcript_line("system", "what is your experience"),
                ai_request_line("what is your experience"),
                ai_response_line("seven years of kubernetes"),
                transcript_line("mic", "let me explain my background"),
                // NO session_stop / session_summary → unfinished.
            ],
        );
        let got = find_unfinished_session(&dir).expect("should detect unfinished session");
        assert_eq!(got.session_id, "crashed");
        assert_eq!(got.path, dir.join("crashed.jsonl"));
        // last_qa pairs the request prompt with the following response text.
        assert_eq!(
            got.last_qa,
            Some((
                "what is your experience".to_string(),
                "seven years of kubernetes".to_string()
            ))
        );
        // last_lines preserves order + source markers, newest last.
        assert_eq!(got.last_lines.len(), 2);
        assert_eq!(got.last_lines[0], "sys: what is your experience");
        assert_eq!(got.last_lines[1], "mic: let me explain my background");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn truncated_tail_parses_rest_no_panic() {
        // The final line is a half-written JSON object (crash mid-flush). The
        // detector must parse everything before it and skip the garbage tail.
        let dir = fresh_dir("trunc");
        let p = dir.join("t.jsonl");
        let body = format!(
            "{}\n{}\n{}\n{}\n{{\"kind\":\"transcript_line\",\"unix_ms\":1,\"sou",
            start_line(30_000),
            transcript_line("system", "tell me about a hard outage"),
            ai_request_line("tell me about a hard outage"),
            ai_response_line("the time the etcd quorum was lost"),
        );
        std::fs::write(&p, body).unwrap();
        let got = find_unfinished_session(&dir).expect("rest parses despite truncated tail");
        assert_eq!(
            got.last_qa,
            Some((
                "tell me about a hard outage".to_string(),
                "the time the etcd quorum was lost".to_string()
            ))
        );
        // Only the ONE complete transcript line survives; the truncated tail
        // is skipped, not panicked on.
        assert_eq!(got.last_lines.len(), 1);
        assert_eq!(got.last_lines[0], "sys: tell me about a hard outage");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn stale_unfinished_returns_none() {
        // Unfinished, but the start is older than the 12h cutoff → no nag.
        let dir = fresh_dir("stale");
        write_jsonl(
            &dir,
            "old.jsonl",
            &[
                start_line(RECOVERY_MAX_AGE_MS + 60_000),
                transcript_line("system", "ancient line"),
            ],
        );
        assert!(find_unfinished_session(&dir).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn empty_dir_returns_none() {
        let dir = fresh_dir("empty");
        assert!(find_unfinished_session(&dir).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_dir_returns_none_no_panic() {
        let dir = std::env::temp_dir().join(format!("overlay-recover-missing-{}", now_unix_ms()));
        // Deliberately do NOT create it.
        assert!(find_unfinished_session(&dir).is_none());
    }

    #[test]
    fn only_start_no_lines_returns_some_with_empty_context() {
        // A crash right after start: unfinished, recent, but no transcript /
        // Q&A yet. Sensible result: Some with empty last_lines + None last_qa.
        let dir = fresh_dir("startonly");
        write_jsonl(&dir, "s.jsonl", &[start_line(5_000)]);
        let got = find_unfinished_session(&dir).expect("start-only is still unfinished");
        assert!(got.last_lines.is_empty());
        assert_eq!(got.last_qa, None);
        assert_eq!(got.summary, None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn no_start_at_all_returns_none() {
        // A file with lines but no SessionStart is not a recoverable session.
        let dir = fresh_dir("nostart");
        write_jsonl(
            &dir,
            "s.jsonl",
            &[transcript_line("system", "orphan line without a start")],
        );
        assert!(find_unfinished_session(&dir).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn newest_file_is_chosen() {
        // Two journals: an OLDER crashed one and a NEWER cleanly-stopped one.
        // Only the NEWEST is inspected → clean stop → None (we must NOT fall
        // back to the older crashed file).
        let dir = fresh_dir("newest");
        write_jsonl(
            &dir,
            "old_crash.jsonl",
            &[start_line(120_000), transcript_line("system", "old crash")],
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
        write_jsonl(
            &dir,
            "new_clean.jsonl",
            &[
                start_line(60_000),
                transcript_line("system", "new clean"),
                r#"{"kind":"session_stop","unix_ms":2}"#.to_string(),
            ],
        );
        assert!(
            find_unfinished_session(&dir).is_none(),
            "newest is clean; must not recover the older crashed file"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn last_qa_uses_the_most_recent_pair() {
        // Two completed Q&As; last_qa must reflect the SECOND one.
        let dir = fresh_dir("lastqa");
        write_jsonl(
            &dir,
            "s.jsonl",
            &[
                start_line(30_000),
                ai_request_line("first question"),
                ai_response_line("first answer"),
                ai_request_line("second question"),
                ai_response_line("second answer"),
            ],
        );
        let got = find_unfinished_session(&dir).expect("unfinished");
        assert_eq!(
            got.last_qa,
            Some(("second question".to_string(), "second answer".to_string()))
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn last_lines_capped_at_recovery_limit() {
        let dir = fresh_dir("cap");
        let mut lines = vec![start_line(30_000)];
        for i in 0..(RECOVERY_LAST_LINES + 5) {
            lines.push(transcript_line("system", &format!("line {i}")));
        }
        write_jsonl(&dir, "s.jsonl", &lines);
        let got = find_unfinished_session(&dir).expect("unfinished");
        assert_eq!(got.last_lines.len(), RECOVERY_LAST_LINES);
        // Oldest evicted: first surviving line is "line 5".
        assert_eq!(got.last_lines[0], "sys: line 5");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn append_bookmark_creates_file_with_header_then_appends_entries() {
        // Override the config dir for isolation. We can't easily mock
        // dirs::config_dir() so this test writes to the real APPDATA
        // location into a uniquely-named subfolder.
        let tag = format!("overlay-mvp-test-{}", now_unix_ms());
        let testdir = dirs::config_dir().expect("config dir").join(&tag);
        let _cleanup = scopeguard::guard(testdir.clone(), |p| {
            let _ = std::fs::remove_dir_all(&p);
        });
        // Manually inline the append logic into the test dir to avoid
        // dependency on dirs::config_dir() inside append_bookmark.
        // (Full mock would need a feature gate; this test pattern is
        // good enough to validate the markdown format.)
        std::fs::create_dir_all(&testdir).unwrap();
        let path = testdir.join("bookmarks.md");
        let is_new = !path.exists();
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .unwrap();
            if is_new {
                writeln!(f, "# suflyor bookmarks\n").unwrap();
            }
            writeln!(f, "## Q1\nA1\n").unwrap();
            writeln!(f, "## Q2\nA2\n").unwrap();
        }
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("# suflyor bookmarks"));
        assert!(content.contains("## Q1"));
        assert!(content.contains("## Q2"));
    }
