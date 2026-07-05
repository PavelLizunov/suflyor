# Speaker-diarization architecture (ADR) — suflyor

Design consult by **fable** (2026-07-05), reconciled + independently code-verified against the repo
(every "current" claim cites `file:line`; the two load-bearing corrections in §0 were re-read by
hand). Written in the `docs/memory-architecture.md` house style.
**Status: APPROVED to build (owner, 2026-07-05).** Owner decisions: start the gated build (D1);
**FIX the recorder timebase (D0.5) rather than ship the banner-MVP** (Correction 2). Defaults
accepted: `diarize` subcommand on the existing exe, WeSpeaker embedding, the «сколько собеседников»
hint. House rule holds: gated increments, no release without «релизь».

## The ask (owner, 2026-07-04)
An OFFLINE, on-demand pass that re-maps a recorded session's transcript **by participant voice** and
shows it in an additional transcript view — so it's clear WHO is speaking. Today the transcript
only splits «Микрофон» (the owner) vs «Система» (system audio); the system side may hold several
remote people («Тимур, Стас, Никиту, Пашу») with no per-speaker attribution. A button «Определить
говорящих» on a finished session → cluster utterances by speaker → show each line's speaker + let
the user rename a speaker.

## 0. Verified current state (incl. two corrections to first assumptions)
- **Recordings** — `recorder.rs` writes per session `recordings/<id>/mic.wav` + `system.wav`,
  16 kHz mono i16 (`recorder::SAMPLE_RATE = 16000`; format asserted `re_transcribe.rs:72`).
  `mic.wav` = the OWNER; `system.wav` = the "other side" = **all remote participants mixed**
  (`re_transcribe.rs:193-196`).
- **Offline-reprocess precedent** — `re_transcribe.rs::transcribe_session` already does this shape
  (finished `session_id` → open `recordings/<id>/*.wav` → run a model offline → assemble lines).
  `aux_windows.rs:831-917` (`start_resummary`) is the archive-button wiring to clone: a
  **process-global one-job latch** (`try_acquire_retranscribe`, RAII), `set_*_busy/_status`,
  `rt.spawn` + an `on_progress` closure marshalled via `slint::invoke_from_event_loop`, refresh
  on done.
- **Transcript rows** — `models.rs:29` `Utterance { session_id, unix_ms, source ("mic"|"system"),
  text, audio_ms: Option<i64> }`. `audio_ms` = ms of the line's START; **no per-line end** (a line's
  end ≈ the next line's `audio_ms`). `audio_ms` present only on post-`0004` sessions.
- **⚠ Correction 1 — `system.wav` contains the app's OWN read-aloud voice.** The recorder tee
  (`slint_session.rs`: `recorder.feed(&chunk)`) is UNCONDITIONAL (bar pause/mic-mute) and runs
  BEFORE the STT anti-feedback gate, which drops read-aloud chunks from **transcription only**
  (`stt.rs:360-372`, `tts::is_speaking`). So Piper's voice is written to `system.wav` but produces
  no utterance. → a naïve diarizer mints a phantom "speaker" = the TTS voice. Handled in §5.
- **⚠ Correction 2 — the `audio_ms`↔`system.wav` timebase divergence is UNBOUNDED, not
  sub-second.** `audio_ms` is WALL-CLOCK elapsed since capture start (`audio.rs:276`
  `start_ts=Instant::now()` → `:313` `timestamp_ms=start_ts.elapsed()` → `stt.rs:313,883`). But the
  WAV advances only when WASAPI loopback delivers packets, and it delivers **nothing while no app
  renders audio** (`audio.rs:280-283`: `wait_for_event(2000).is_err() → continue`, no samples
  written). Every loopback-quiet stretch (session armed before the call; call ended but session
  running) pushes `audio_ms` permanently ahead of the WAV position. The player already has this and
  degrades gracefully (late seek); diarization would degrade to **wrong names**. **Owner decision
  (2026-07-05): FIX this in the recorder (D0.5)** — pad each channel's WAV with silence so
  WAV-position stays wall-clock-aligned (== `audio_ms`); forward-only, and it fixes the player's
  late-seek as a bonus. The §5.6 banner survives only for PRE-fix (legacy) recordings whose WAVs
  already bake in the divergence.
- **Re-index caveat** — sessions/utterances are rebuilt from JSONL journals on re-index
  (`indexer.rs` `replace_session` touches only `sessions/utterances/ai_turns`); user-owned
  side-tables survive (like the memory tables, `models.rs:89`). ⇒ diarization result lives in a
  side-table keyed by `session_id`.
- **The engine we already ship** — `suflyor-tts/` links `sherpa-onnx = "1.13"` static
  (`Cargo.toml:20`) because two static onnxruntimes in one process crash on the 2nd model load (the
  app owns `ort`/GigaAM). The installed crate **`sherpa-onnx-1.13.3` already exposes
  `OfflineSpeakerDiarization`** (`create → process(&[f32]) → sort_by_start_time() ->
  Vec<Segment{start,end,speaker}>` in f32 seconds, `+ num_speakers()`; config = pyannote seg +
  `SpeakerEmbeddingExtractorConfig` + `FastClusteringConfig{num_clusters:-1, threshold:0.5}` +
  `min_duration_on 0.3/off 0.5`). `main.rs` is a **no-arg stdin loop** — an argv subcommand slot is
  free. The **17.5 MB** exe is already built (`build-slint-release.ps1:42`) and bundled
  (`slint-installer.nsi:44`, uninstall `:85`), spawned sibling-of-exe with piped stdin (`tts.rs:487-520`).
  ⇒ the diarization engine is **already in the tree and already shipping**.
- **`process()` takes the WHOLE waveform** (`&[f32]`) — global clustering, no streaming API. RAM/CPU
  ceiling in §11.
- **Model-install precedent** — `tts_install.rs`: `curl.exe --retry` → SHA-256 pin → System32
  `bsdtar` → `%APPDATA%\suflyor\…`, per-pack independence, Settings button. Reused as-is.
- **Transcript UI** — `transcript.slint:17` `TranscriptLine { offset-label, speaker, text, checked,
  marked, start-ms }`; `speaker` is a **Rust-built string** (today "Система"/"Микрофон"). Window
  `TranscriptWindow`, opened by `aux_windows.rs::open_transcript` (📄 archive button already exists).

## 1. Principles
1. **No new in-process native runtime** — sherpa's onnxruntime NEVER shares the host process with
   `ort`/GigaAM; diarization is a separate OS process (the sidecar lesson).
2. **Reuse before adding** (ponytail) — engine, crate, exe, installer entry, download/verify flow,
   offline-job wiring, and the `speaker` field ALL exist. Build the smallest bridge.
3. **Diarize `system.wav` only.** Mic is one known speaker (the owner → «Вы»); the unknown-speaker
   problem lives entirely in the system channel.
4. **Offline + explicit** — a button on a FINISHED session, one job at a time, progress shown; never
   live, never automatic.
5. **Durable + additive** — side-table keyed by `session_id`; migration additive-only; the original
   transcript is untouched; the new view is a relabel.
6. **Approximate, and honest.** VoIP audio + the timebase caveat make this good-not-perfect; the UI
   frames speakers as auto-detected + renameable, with an explicit "unreliable" banner when a guard
   trips. Acceptance target: **≥80 % of lines correct on 3–4 voices**, not 100 %.
7. **Pure-function-first** — segment→utterance alignment and the JSON parser are pure, unit-tested
   Rust (fixtures, like `line_start_offset_ms`).

## 2. Pipeline (target)
`«Определить говорящих» → spawn suflyor-tts.exe diarize <system.wav> (separate process) → sherpa:
pyannote seg → speaker embeddings → agglomerative clustering (unknown N) → JSON segments on stdout →
parent parses → DROP phantom clusters (zero utterance overlap) → align each system Utterance by
max-overlap over [audio_ms, next.audio_ms) (mic → «Вы») → persist (diarization side-table) →
«По голосам» view relabels + colors each line by speaker + rename`.

## 3. Engine + models (reuse sherpa; install two small ONNX models)
- **Segmentation** — pyannote-segmentation-3.0 (~6 MB), asset
  `sherpa-onnx-pyannote-segmentation-3-0.tar.bz2` from the `speaker-segmentation-models` release.
- **Speaker embedding — default WeSpeaker VoxCeleb ResNet34** (~26 MB, `speaker-recongition-models`
  release). Rationale: embeddings model the voice not the words, but training distribution still
  moves the borderline cases — VoxCeleb2 is multilingual with many Slavic speakers, the safer prior
  for Russian than 3D-Speaker CAM++ (Mandarin-only training) or TitaNet (English-heavy, least-
  exercised ONNX). It decides only the hard cases (two similar male voices); for 2–4 distinct voices
  all three separate fine. **Upgrade path (one, named):** swap the pinned pack to 3D-Speaker
  ERes2NetV2 / CAM++ zh-en (~2–3× faster on CPU) **if** CPU wall-time is the complaint after
  shipping — a one-line URL+SHA change (the side-table stores segments, not embeddings, so nothing
  else moves).
- **Clustering** — `FastClusteringConfig`. **⚠ CLI-verified finding (D1, 2026-07-05): auto count
  (`num_clusters:-1`) is UNUSABLE on real VoIP loopback audio.** On a 30-min meeting, auto gave
  **150 "speakers" at threshold 0.5, 67 at 0.7, still 23 at 0.9** — the compressed loopback
  embeddings are too noisy for distance-threshold count detection. But **forcing the count works
  cleanly**: `--num-speakers 2` → a 71 %/29 % two-way split; `--num-speakers 3` → 70/29/0 (only ~2
  real speakers). The *segmentation* is fine either way (~350 real turns); only auto count detection
  fails. ⇒ **the «сколько собеседников» count is the PRIMARY control, not an optional hint** (§7):
  the client passes `num_clusters = count`. Auto (`-1`) is kept only as a last-resort fallback (when
  the user truly can't say) and its output is flagged low-confidence. `min_duration_on/off` at
  defaults (0.3/0.5).
- **Provider** `cpu`, `num_threads: 4`. Both models 16 kHz — matches the recorder, **no resampling**.

Two SHA-pinned packs install via a Settings → AI button (mirroring `tts_install.rs`) into
`%APPDATA%\suflyor\diar\`. Not bundled (kept out of the base installer, like the voices). New backend
module `diar_install.rs` ≈ a near-copy of `tts_install.rs` with two `ModelPack` rows (~32 MB total);
SHAs pinned at implementation.

## 4. Sidecar shape — a `diarize` subcommand on the EXISTING exe (recommended)
`main()` dispatches on `argv[1]`: **no arg → today's TTS stdin loop, byte-identical**; **`diarize
<system.wav> [--num-speakers N] [--threshold T] [--models-dir D]` → load WAV → i16→f32 →
`OfflineSpeakerDiarization::process` → print `{"num_speakers":N,"segments":[{"s":ms,"e":ms,"sp":i}]}`
to stdout → exit** (non-zero + reason on stderr on failure). New module `suflyor-tts/src/diar.rs`.
It runs as a *separate OS process instance* — a live read-aloud and a diarize batch coexist; the two
protocols can never interleave.

**Why not a new `suflyor-diar.exe` crate** (owner's initial guess): a second ~17 MB static-sherpa exe
roughly doubles this subsystem's installer payload; a THIRD standalone crate rebuilds the whole
sherpa-onnx-sys native lib into its own `target/` (no workspace — and `target/` bloat is a documented
pain here); plus a new build step, NSIS File+Delete lines, spawn path, lint/test surface — all to buy
a nicer name. The engine, the process-isolation guarantee, and the ship vehicle already live in
`suflyor-tts.exe`. **Honest naming caveat:** an exe called "tts" that also diarizes is a small lie;
it is really "the sherpa-onnx audio sidecar". Keep the name (renaming touches build-script :42/:49,
NSIS :44/:85, `tts.rs:491`, plus an installer-upgrade stale-exe edge — churn for cosmetics) and mark
it: `// ponytail: 'tts' name is historical — this is the sherpa-onnx audio sidecar (TTS + diarize);
rename to suflyor-audio.exe if a third job lands`. **Tradeoffs named:** TTS + diarize are version-
locked to one sherpa (one CI run re-gates both — fine, same vendor lib); the exe grows +1–3 MB
(diarization symbols no longer stripped).

Client side (`overlay-backend/src/diarize.rs`, no onnx): resolve `suflyor-tts.exe` sibling-of-exe
(reuse `tts.rs::sidecar_exe_path`), `Command::spawn` `CREATE_NO_WINDOW` with piped stdout, wait,
`serde_json` parse → segments. Mirrors `tts.rs` sidecar management minus the long-lived stdin.

## 5. Alignment — max-overlap, gated, phantom-filtered (pure Rust, tested)
1. **Phantom filter (do it — ~10 lines):** DROP any cluster whose segments overlap ZERO utterance
   windows before display — "a speaker nobody transcribed doesn't exist". Kills the TTS phantom
   (Correction 1), hold-music, notification pings, a background YouTube. This is the cheapest, highest-
   value guard.
2. **Assign** each `system` utterance over the window `[audio_ms_i, next_utterance.audio_ms)` (next of
   ANY source; last line capped at `min(audio_ms+60s, wav_end)`): the speaker with **maximum
   overlapped duration**. Point-in-segment at `audio_ms` fails structurally — utterance starts cluster
   at segment boundaries (STT finalizes after a VAD silence gap, exactly where turns happen) + ~200 ms
   chunk granularity (`audio.rs:272,313`); max-overlap is robust to the whole sub-second error budget
   at ~zero cost.
3. **No overlapping segment** → `speaker = None` → render as today's «Система» (never force-assign).
   **Line spanning two speakers** → majority wins, one label (no word timestamps exist to split on —
   splitting would invent data).
4. **Mic** utterances → always «Вы». *Mic simplification confirmed sound:* loopback is a render-
   endpoint capture (`audio.rs:257-258`); conferencing apps don't render your own voice locally, and
   remote echo is killed by their AEC → the owner's voice is structurally absent from `system.wav`.
   (Pre-existing out-of-scope caveat: an owner on **speakers** not headphones leaks remote voices into
   `mic.wav` — already mislabeled «Микрофон» today; this feature neither fixes nor worsens it.)
5. **Gate the button** on: session **finished** (a live WAV is unfinalized) AND system utterances have
   `audio_ms` (post-`0004`). The wall-clock fallback is documented SECONDS late (`session_audio.rs:109-116`)
   → confidently wrong names; old sessions show «недоступно для старых записей».
6. **Timebase (Correction 2) — fixed at the source (D0.5), banner for legacy.** The recorder is
   changed to pad each channel's WAV with silence up to the chunk's wall-clock offset, so
   WAV-position == `audio_ms` for all recordings made after the fix (see D0.5). For PRE-fix
   recordings only, keep the honest fallback: if `last system audio_ms > wav_duration + 30 s`, still
   show the view but with a «метки времени неточны — в записи были паузы» banner.

## 6. Storage — migration `0006_diarization.sql` (additive)
One durable JSON row per session (write-all-once per run, read-all-once per open, map-in-Rust at
render — nothing queries segments across sessions, so a normalized table models queries that don't
exist):
```
diarization(
  session_id          TEXT PRIMARY KEY,        -- stable journal-stem id (not FK'd to the projection)
  created_at_ms       INTEGER NOT NULL,
  num_speakers        INTEGER NOT NULL,
  model_id            TEXT NOT NULL,            -- "pyannote-3.0+wespeaker-resnet34" (provenance)
  segments_json       TEXT NOT NULL,            -- [{s,e,sp}] sorted; ~10-20 KB for a 1 h call
  speaker_names_json  TEXT NOT NULL DEFAULT '{}' -- {"0":"Тимур"} renames; default «Говорящий N»
)
```
`migrations.rs`: append `(6, include_str!("0006_diarization.sql"))`, bump `LATEST_VERSION = 6` (never
edit an applied migration). Store methods (`sqlite_store.rs`): `get_diarization`, `put_diarization`,
`rename_speaker(session_id, idx, name)` (`UPDATE speaker_names_json`). Durability = the memory-table
pattern (`replace_session` never touches side-tables). **Re-run = REPLACE the row AND clear
`speaker_names_json`** — cluster ids permute across runs, so stale names would silently mislabel;
confirm-dialog if names already exist. `// ponytail: segments as one JSON blob; normalize only if we
ever query segments by time-range across sessions — we don't`.

## 7. UI — a «По голосам» view in the transcript window
Reuse `TranscriptWindow` + `TranscriptLine` — `speaker` is already Rust-built, so "by voice" is
largely a relabel:
- **Toggle** «По ролям» (today's default) ↔ «По голосам». In «По голосам», Rust rebuilds `lines`
  with `speaker` = the diarized name (renamed / «Говорящий N» / «Вы») + a new
  `TranscriptLine.speaker-color` (deterministic per-speaker palette by index).
- **Button** «Определить говорящих» — visible when the session is finished + `has-audio` +
  `audio_ms` present; disabled while a job runs (mirrors ↻ re-transcribe, `aux_windows.rs:831`).
  Existing diarization → the toggle shows it; the button re-runs behind a confirm (which clears
  names). Old/live sessions → «недоступно для старых записей».
- **Speaker count — the PRIMARY input** (D1 finding, §3): a field «Сколько собеседников (без вас)?»
  → maps 1:1 to `num_clusters`, because auto count-detection is unusable on VoIP audio. It's the ONLY
  repair a human can perform (the owner can count "Тимур/Стас/Никита/Паша = 4"; he can never tune a
  cosine threshold). Default it to **2** (the common one-interviewer case); the «Определить
  говорящих» flow lets the user set it before running and re-run with a different count. Label says
  **без вас** (mic isn't diarized). «Авто» is offered as an explicit low-confidence choice, not the
  default. Threshold is NOT exposed.
- **Rename** — a compact per-speaker list (color chip + editable name) atop «По голосам»; edits call
  `rename_speaker` + refresh; persist in `speaker_names_json`.
- **Timebase banner** — «метки времени неточны — в записи были паузы» when §5.6 trips.
- **i18n** — chrome `@tr("English")` + `msgid`/`msgstr` in `slint-replay.po` («Определить
  говорящих», «По голосам», «По ролям», «Говорящий {N}», «Вы», «Сколько собеседников…», banner);
  speaker NAMES are data. Runs through `i18n_guard`.
- **Wiring** — `aux_windows.rs`: an `on_diarize_requested` callback cloning `start_resummary`'s
  latch/spawn/progress/refresh; a pure `rebuild_lines_by_speaker(utts, diar)`.

## 8. Methodology / gate (mandatory — CLAUDE.md)
Every commit: independent **review-agent** over the diff (briefed cold) + `scripts/ci.ps1`
(fmt --check, clippy `-D warnings`, tests ×3 crates incl. new `suflyor-tts diar` + backend tests, and
`i18n_guard`) → "All gating layers green". `git-gate.ps1` enforces fmt/clippy on commit, tests on
push, blocks `gh release` without `docs/retest-*<ver>*.html`. UI verified live (CopyFromScreen at the
HWND / Slint-MCP — computer-use mis-renders the overlay). **No release without «релизь».** New
user-facing strings need `@tr` + `.po` in the same commit. Acceptance criterion (into the retest
checklist): **≥80 % of lines correctly attributed on a real 3–4-voice session**, phantom TTS speaker
absent, rename + re-run behave.

## 9. Phased plan (each phase behind the gate; owner tests before the next)
| Phase | Contents | Effort | Risk / mitigation |
|---|---|---|---|
| **D0 — Sign-off** | ✅ DONE 2026-07-05 — subcommand + WeSpeaker + count-hint accepted; **recorder-fix chosen over the banner-MVP** | — | wrong assumptions caught before code |
| **D0.5 — Recorder timebase fix** (first, owner's call) | ✅ IMPLEMENTED — gate green + independent review passed; **commit pending owner go**. `recorder.rs` pads each channel's WAV with silence to the chunk's wall-clock offset (`sample_offset == audio_ms`); forward-only; **bonus: fixes the player's late-seek**; 9 unit tests. Two guards added from review: **per-gap (10 min) + per-session (30 min) pad caps** with over-cap excess absorbed into `skew` (bounds disk, no post-gap garble), and a **silence-skip in `re_transcribe.rs`** so padded windows aren't fed to STT (also helps pre-D0.5 recordings). Legacy WAVs → §5.6 banner | **S-M** done | live capture hot path → tested + reviewed; disk/STT cost of padding → the two caps + silence-skip |
| **D1 — Sidecar + models** | `suflyor-tts diarize` subcommand (`diar.rs`); `diar_install.rs` (2 SHA-pinned models) + Settings button; **CLI-verifiable end-to-end** on a real `system.wav` (eyeball JSON) — no UI | **M** ~2-3 d | model quality on VoIP → test on a real session; whole-file RAM → `MAX_DIAR_SECS` 3 h guard |
| **D2 — Persist + align** | `0006` migration + store methods; `diarize.rs` (spawn/parse/align); pure `align_speakers` + phantom filter + parser unit tests; timebase sanity | **S-M** ~1-2 d | re-index wipe → side-table (tested); alignment edges → fixtures |
| **D3 — «По голосам» view** | toggle + gated button + `speaker-color` + rename + count hint + banner + i18n; live visual verify | **M** ~2-3 d | overlay paint/i18n drift → UI-diff 3-check (screenshots+checklist, UI-review agent, Slint-MCP) |
| **D4 — Polish** | threshold tuning from owner retest; «Говорящий N» legend; export-by-speaker in Copy | **S** | none material |

**First shippable increment = D1+D2 proven at CLI/DB level before any UI** — the engine + alignment
are the risk; the tab is the easy part.

## 10. Deliberately NOT designed (named re-entry conditions)
- **Retro-fixing LEGACY recordings' timebase** — D0.5 fixes it forward-only; already-recorded WAVs
  keep the divergence and rely on the §5.6 banner. Re-enters only if the owner needs accurate
  diarization on pre-fix recordings (would require re-deriving WAV offsets from the journal).
- **Chunked 8 h+ diarization** with cross-window cluster re-identification — capped by
  `MAX_DIAR_SECS`; re-enters when the cap is actually hit.
- **Per-word / line-splitting attribution** — needs word timestamps the pipeline doesn't produce.
- **Enrollment / persistent voice profiles** ("this is always Тимур across sessions") — needs stored
  embeddings + consent design; re-enters if the owner asks.
- **Threshold in the UI** — re-enters if the count-hint + rename prove insufficient.
- **Mic-channel diarization** — mic is one speaker by construction.
- **Exe rename to `suflyor-audio.exe`** — re-enters if a third audio job lands in the sidecar.

## 11. Risks & the cheap guards (fable's ranking)
1. **TTS phantom speaker in `system.wav`** (Correction 1) → the §5.1 phantom filter (drop
   zero-utterance-overlap clusters). ~10 lines; also kills music/pings/YouTube.
2. **Unbounded `audio_ms`↔WAV divergence** (Correction 2) → **FIXED at source in D0.5** (recorder
   silence-padding, WAV-position == `audio_ms`; also fixes the player's late-seek). §5.6 banner
   remains only for legacy pre-fix recordings.
3. **Whole-waveform RAM/CPU** — 1 h ≈ 230 MB f32, 8 h ≈ 1.84 GB + sherpa copies → **hard cap, refuse
   > 3 h** with a clear message (mirrors the old re-transcribe guard); worker thread + step progress.
4. **Real call audio is hard** — Opus-compressed, AGC'd, denoised. 2 voices good; 3–4 usable with
   occasional swaps; two similar male voices confuse at any threshold. Rename + count-hint ARE the
   repair loop → the ≥80 % acceptance target, not 100 %.
5. **Small** — gate on session-finished (unfinalized live WAV); DirectML not worth it for a
   minutes-scale batch; ~32 MB model pack via the existing installer flow.

## 12. Files touched (surface preview)
New: `suflyor-tts/src/diar.rs`, `overlay-backend/src/diarize.rs`,
`overlay-backend/src/diar_install.rs`, `overlay-backend/migrations/0006_diarization.sql`.
Edited: `suflyor-tts/src/main.rs` (argv branch), `overlay-backend/src/lib.rs` (+2 modules),
`persistence/{migrations,sqlite_store,models}.rs`, `ui/transcript.slint`,
`overlay_host/aux_windows.rs` (+ the Settings panel for the install button), `slint-replay.po`.
**Installer: no new artifact** — reuses `suflyor-tts.exe`; only the runtime model-download is new.
New code is roughly: one argv branch + one client fn + one migration + one alignment fn + one
Settings install button + one transcript tab.
