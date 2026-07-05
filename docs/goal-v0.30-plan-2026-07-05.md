# v0.30 plan — fable breakdown of v0.29.0 owner feedback (A–H)

Owner live-tested v0.29.0 (`docs/retest-v0.29.0-filled.html`) → functional, but wants UX
reworks + flagged bugs. Three parallel fable design passes (2026-07-05) broke down A–H.
This is the consolidated plan. Branch: `claude/diarization-d05`.

## The items (A–I)

| # | Item (owner ask) | Approach (fable) | Effort | Origin |
|---|---|---|---|---|
| **A** | Player: pitch-preserving speed (YouTube-style, no chipmunk) | `timestretch` crate (MIT, pure-Rust; `rustfft` already in tree) as a `StretchSource` rodio adapter between `PcmCursor`→`Sink`, rebuilt per speed change. **Position clock unchanged.** Fallback: hand-rolled WSOLA (~150 lines, 0 dep). Reject soundtouch (LGPL), signalsmith (needs libclang), rubato (resampler, not stretch). | **M** | feature |
| **B** | Speed as dropdown 1×…3× step 0.25 | std `ComboBox` (already in replay/settings); `speed-index` prop → `set-speed(1+idx*0.25)`. Rust wiring unchanged. | **S** | feature |
| **C** | Volume as slider | std `Slider` (already in settings); `value<=>volume`, live to sink (no rebuild). | **S** | feature |
| **D** | Diarization: AUTO speaker-count | **Forced-count SWEEP + degeneracy elbow** — never use threshold clustering (that's the 139–150 bug); force 2,3,4… and stop at the first run that leaves a near-empty cluster (ADR data: forcing 3 on a 2-person call → 70/29/**0**). App-side (`diarize.rs`), sidecar untouched. Common case = 2 runs. «Авто» chip default-on; on success set stepper to found N + uncheck (correction UI). | **M** | feature |
| **E** | D5 banner "timings approximate" = BAD | **Root cause:** `timeline_reliable` spans ALL utterances incl. mic, but alignment uses only system lines; system.wav freezes at call-end while the goodbye-into-mic line is timestamped later → **banner fires on ~every session** = meaningless. Fix: system-only span (1 line, `diarize.rs:71`) + show only with `has-diarization` + 1 test. | **S** | **bug (v0.29.0/mine)** |
| **F** | Re-detect wipes speaker names, no guard | Clone the archive "Recreate summary?" confirm (pure-Slint, no new Rust callback); guard **only when custom names exist**. | **S** | feature/gap |
| **G** | Transcript layout gap + squished list + narrower window | **Root cause:** empty `HorizontalLayout` (`transcript.slint:328`) whose sole child is `if has-diarization` — an empty box-layout in Slint 1.16.1 reports `stretch=f32::MAX` (eats all vertical space → starves the ListView) **and** `max-width=0` (clamps window narrower). "Fixes after formation" = the **diarization** job's `set_has_diarization(true)` de-empties it. Fix: delete wrapper, hoist the `if`. | **S** | **bug — NEW, D3 commit `8b4fa2c` on THIS branch** |
| **H** | First lines of both channels show 0:00 / wrong order | **Root cause:** `stt.rs:314-316` stamps an utterance from its buffer's FIRST chunk (incl. leading silence); both channels' first chunk ≈ t0 → first voiced line inherits `start_ts_ms≈0`; display sorts by that offset → mic 0:00 ties/sorts above the system line. Fix: snap start to voice onset (`stt.rs:318-323`, ~3 lines). Also improves diar alignment for opening lines. | **S** | **bug — pre-existing, exposed v0.27** |
| **J** | Read-aloud (TTS) of a SUMMARY doesn't stop on window close (X); works in a regular tile | Wire the summary window's close (and any non-tile read-aloud surface) to the TTS stop that tiles already call. Small. | **S** | **bug — owner 2026-07-05, «на будущее» / low priority** |
| **I** | In-transcript word search → jump to the MOMENT it was said (owner asked 2026-07-05: find where a topic was mentioned) | Search box in the transcript window: filter/highlight matching lines; Enter/click a match → the existing `play-line`/`seek-and-play` jumps the player to that line's `audio_ms` + plays. Infra already there (per-line timecodes + click-to-seek). **Distinct** from the archive FTS5 search (that's cross-session — which session mentions X; this is where-in-THIS-transcript + play). | **S–M** | feature — part of the original "полноценный плеер" ask (search+volume+speed); volume/speed shipped, search was dropped |

## Correction to my earlier live report
I told the owner the transcript gap (G) looked **pre-existing / not my change**. fable refuted that: it was introduced by the **D3 diarization commit `8b4fa2c` on this branch** (the empty wrapper). My check only covered my own `dcc2d61` diff, not the earlier D3 commit on the same branch. G is a diar-branch regression with a one-line fix.

## Recommended sequencing (accumulate into verified releases)
- **Phase 1 — bugs (fast, safe, S each):** G → H → E → F. ✅ DONE + owner-accepted (`b9ccdd2`, `66494bb`, `35a5cc9`).
- **Phase 2a — I (in-transcript search): ✅ DONE (`20901b7`).** Gate 0/0 (3 crates) + independent review SHIP (borrow-safety traced safe). Built + installed to `%LOCALAPPDATA%` (hash-verified, `Search in transcript` string confirmed in exe) + relaunched. Pure helpers `transcript_search_hits` + `next_hit_index` unit-tested. **Pending owner live-check** via `docs/retest-v0.30-I.html` — esp. **I0** (empty-state layout: the ListView was made unconditional so the `scroll-to-line` fn can reference it → confirm no phantom scrollbar/gap when the transcript is empty).
- **Phase 2b — player (A+B+C): ✅ DONE (retest #1 → A re-engined to WSOLA).** A = pitch-preserving speed via the crate's **time-domain `timestretch::stretch::Wsola`** driven directly (`StretchSource`, ratio = 1/speed, chunk-continuity harness = re-feed overlap + boundary crossfade, 1× bit-exact bypass, position clock unchanged); B = speed ComboBox 1×–3× step 0.25 (`speed-index`); C = volume Slider 0–3× (live, no rebuild). **Retest #1 (owner):** A REJECTED — the initial `StreamProcessor` (phase-vocoder) gave echoey/hollow speech (fable diagnosis: 256 ms FFT window @16 kHz unrescaled + onset overlay + 3× pitch-shifted ghost blend); footer controls were equal-thirds. **Fix:** A switched to WSOLA (time-domain, no phasiness — what podcast players use); footer ComboBox/Slider pinned to fixed widths so the stretchy seek-bar dominates. Gate 0/0 (3 crates, +multi-chunk timeline test) + 2 independent reviews SHIP (timeline exact 0% / −0.006% at 2×/3×, no drop/dup, termination bounded by EOF, no-panic). **Pending owner LISTEN #2 (WSOLA speech quality at 2×/3×; possible faint ~1/s boundary seam → fix = larger `XFADE_OUT`) + VISUAL (footer proportions)** via `docs/retest-v0.30-player-audiofix.html`.
- **Phase 3 — auto-diarization (D):** the meaty change; its retest exercises E/F too.
- **Backlog:** J (summary read-aloud stop-on-close, low).

## Decisions needed from the owner
1. **A (pitch):** accept the young pure-Rust `timestretch` dep first (recommended), or go straight to zero-dep hand-rolled WSOLA?
2. **C (volume):** slider max = **3×** (keeps the boost he asked for, recommended) vs phone-style 0–100%?
3. **D (auto):** accept ~2× single-run time in auto mode (per-candidate progress shown) + cap 5 (<1h) / 3 (≥1h)? After a successful auto run, auto un-latches + stepper shows found N (recommended)?
4. **Packaging:** ship Phase-1 bugs as a quick **v0.29.1**, then player + auto-diar as **v0.30**? Or everything as one **v0.30**?

## Tunable constants (leave as `// ponytail:` ceilings, tune from owner retest)
`MIN_SHARE` (5%) · `MIN_WINS` (2) · `AUTO_CAP` (5/3) · `TIMELINE_TOL_MS` (5s) · timestretch preset (Vocal).
