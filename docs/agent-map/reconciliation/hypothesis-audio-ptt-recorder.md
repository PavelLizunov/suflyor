# Original audio C06/C08: bounded PTT accumulation and silence padding evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_audio_ptt_recorder_hypotheses.py) inspect frozen audio and recorder sources. They do not initialize WASAPI/CoreAudio, capture live microphone audio, or perform disk I/O. C06 and C08 remain hypotheses.

## C06 — PTT unconstrained buffer growth until stop signal

In Windows capture, [record_source_until_stop](<../../../overlay-backend/src/audio.rs#L502-L599>) pre-allocates an initial 30-second buffer `Vec::with_capacity((actual_rate as usize) * 30)` and continuously appends frames in a loop `while !stop.load(Ordering::Acquire)` with no maximum buffer size check.
In macOS capture, [record_source_until_stop](<../../../overlay-backend/src/audio_macos.rs#L516-L575>) similarly collects incoming audio samples into an unbounded `Vec<f32>` buffer until `stop` becomes true. In contrast, the continuous streaming live audio paths enforce bounded buffers (~200ms) and bounded mpsc queues that drop overflowing frames.

## C08 — silence gap padding and budget absorption

In `recorder.rs`, [constants](<../../../overlay-backend/src/recorder.rs#L80-L92>) define `MAX_PAD_SAMPLES` (10 minutes) and `MAX_TOTAL_PAD_SAMPLES` (30 minutes).
The [plan_pad](<../../../overlay-backend/src/recorder.rs#L368-L381>) function computes silence insertion between timestamps:
1. Gaps exceeding 10 minutes are truncated to `MAX_PAD_SAMPLES`, absorbing the excess gap into `skew` so subsequent timestamps remain contiguous.
2. Backwards or overlapping timestamps result in zero pad samples (`gap = 0`), preserving forward-only WAV writes without seeking backwards.
3. The total inserted pad across a session is clamped by `pad_budget` (`MAX_TOTAL_PAD_SAMPLES`).

## Limits

No live microphone audio was captured, no memory exhaustion test was run, and no multi-hour recording session was performed. Original statuses in `candidates.json` remain `hypothesis`.
