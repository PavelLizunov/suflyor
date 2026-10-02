# Archive, playback and offline retranscription: source-linked contract

**Evidence:** source inspection at `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`; no production database/audio read, native playback/STT/Slint build or deletion performed. Archive list, retained catalog, raw journal, recordings and sidecars have distinct lifecycles.

## Archive entry and queries

[F7 open_archive](../../../slint-experiment/src/bin/overlay_host/aux_windows/archive.rs#L32-L162) creates/reuses ArchiveWindow, snapshots recording/summary/debrief availability and opens/reindexes catalog on worker. It skips active session ID during reindex and surfaces catalog unavailable on open failure. Initial list caps at 300 UI rows (ARCHIVE_LIST_CAP; earlier task-owned 100 claim corrected against source), but [Store::list_sessions](../../../overlay-backend/src/persistence/sqlite_store.rs#L180-L202) loads all rows before cap; display bound is not query allocation bound.

[Search-as-you-type](../../../slint-experiment/src/bin/overlay_host/aux_windows/archive.rs#L165-L210) uses existing Store handle and fresh summary/debrief snapshots on UI callback, caps FTS hits 60 and falls back to empty results on query error. [fts_query](../../../slint-experiment/src/bin/overlay_host/aux_windows/archive.rs#L929-L947) splits punctuation and emits prefix tokens. Full list refresh clears index-keyed rename state so stale row index does not rename a different current row.

[Activation](../../../slint-experiment/src/bin/overlay_host/aux_windows/archive.rs#L213-L244) reads indexed session/utterances/AI turns and builds content tile. [Inline rename](../../../slint-experiment/src/bin/overlay_host/aux_windows/archive.rs#L247-L289) writes separate [session_names JSON map](../../../overlay-backend/src/session_names.rs#L35-L107); failed persistence is logged/best effort, not a Store migration or guaranteed UI-confirmed save.

## Transcript/player

[Transcript open](../../../slint-experiment/src/bin/overlay_host/aux_windows/archive.rs#L744-L807) queries session rows and opens reusable transcript window with fresh model. [Player wiring](../../../slint-experiment/src/bin/overlay_host/aux_windows/transcript.rs#L337-L467) seeds current session audio off-thread and polls position/state via timer, enabling row timecode seek and playback speed.

[session_audio mixing](../../../overlay-backend/src/session_audio.rs#L19-L110) full-loads available i16 WAV channels, adds/clamps PCM and applies recorded TTS masks when sample rate matches. Input helper is lenient on unreadable/torn samples and does not itself prove both channel formats/rates match; recording format and exact playback alignment need test. [line_start_offset](../../../overlay-backend/src/session_audio.rs#L179-L209) prefers stored audio_ms; old sessions fall back to previous wall-clock timestamp. Header prose still describes pre-padding append-only skew while newer recorder has wall-clock gap padding, so do not repeat an unconditional old no-shared-timeline claim.

[Transcript player](../../../slint-experiment/src/bin/overlay_host/transcript_player.rs#L22-L181) holds shared PCM slice, seeks by sample/time conversion and processes buffered WSOLA chunks for non-1x speed. Full playback load can grow with recording duration even though offline re-STT streams chunks. Seek/speed correctness and device lifecycle remain native acceptance, not source declarations.

## Re-summary input selection and latch

[Archive re-summary](../../../slint-experiment/src/bin/overlay_host/aux_windows/archive.rs#L475-L607) takes process-global retranscription busy guard across window close/reopen. If no recordings, it chooses catalog transcript then JSONL AI request prompt fallback and calls summary with `force=true`; if recordings exist, it re-STTs audio and then summary. This actual ordering differs from broad summary_source comment saying saved catalog always has priority.

[Catalog fallback](../../../overlay-backend/src/summary_source.rs#L29-L46) converts stored utterances to TranscriptLine. [Prompt fallback](../../../overlay-backend/src/summary_source.rs#L49-L95) deduplicates first-seen user_prompt lines from AI requests, emits one System line timestamp=0 and ignores AI response text. It reconstructs usable text, not original chronology/word timestamps or guaranteed complete transcript.

[Offline STT config and preflight](../../../overlay-backend/src/re_transcribe.rs#L170-L223) snapshots active backend/language/prompt, validates cloud key/GigaAM model and optionally loads TTS masks. [WAV stream](../../../overlay-backend/src/re_transcribe.rs#L224-L281) validates mono/16k/i16, rejects >24-hour channel, reads sequential backend windows (cloud 600s, Whisper 300s, GigaAM 60s), masks app-speech intervals, skips peak-amplitude-silent windows and concatenates text.

[assemble_lines](../../../overlay-backend/src/re_transcribe.rs#L148-L168) deliberately returns at most two aggregated labeled lines, mic timestamp=0 then system=1. This is not timestamped conversational interleaving or speaker diarization. Each audio window is bounded; cumulative transcript strings and summary outputs are not globally limited by that audio chunk size.

[retranscribe_and_summarize](../../../overlay-backend/src/re_transcribe.rs#L284-L305) awaits summary but returns Ok(line_count) after summary's internally handled success/error tile, so Ok means retranscription chain reached summary, not AI summary success. It passes `force=false`, unlike no-recordings archive branch; identical-input saved conspect may be reused despite generic “rebuild” UI wording. Busy latch has no shown cancel button/watchdog in this callback; close hides UI but does not by itself terminate work.

## Delete and retained data

[Archive confirmation/active guard](../../../slint-experiment/src/bin/overlay_host/aux_windows/archive.rs#L351-L474) prevents active session deletion and asks UI confirmation. The current confirmed-delete callback performs filesystem/DB mutation synchronously under Store lock, then releases it before invoking query_changed; it is not offloaded merely because initial archive indexing is. [delete_session_everywhere](../../../overlay-backend/src/session_admin.rs#L33-L99) rejects unsafe IDs, removes audio/JSONL first, best-effort deletes conspect/debrief, then catalog. File stage failure keeps row for retry; earlier artifacts might already be gone, so it is not an all-or-nothing cross-filesystem transaction. Ignored sidecar deletion failure and session_names entry retention are separate residuals, not guaranteed all-artifact cleanup.

[Store::delete_session](../../../overlay-backend/src/persistence/sqlite_store.rs#L155-L177) clears FTS and diarization and cascades indexed utterances/AI turns. Curated memory remains independent and can refer to deleted source session; raw evidence recovery is not guaranteed. Retention pruning differs from explicit user delete: catalog may intentionally outlive journal/WAV.

## Evidence still needed

Declared tests in [re_transcribe](../../../overlay-backend/src/re_transcribe.rs#L308-L428), [session_audio](../../../overlay-backend/src/session_audio.rs#L212-L375), [session_admin](../../../overlay-backend/src/session_admin.rs#L102-L146) and [archive](../../../slint-experiment/src/bin/overlay_host/aux_windows/archive.rs#L1184-L1455) were not natively run. Required exact-SHA UI/audio acceptance: empty/unavailable/malformed FTS, 100+ sessions, renamed/deleted rows, locked partial deletion, retained old sessions, same-session reopen, long recordings/TTS masks, two-channel mixing/seek/speed and cancellation/summary failures. No destructive fixture against owner data is permitted by this contract.
