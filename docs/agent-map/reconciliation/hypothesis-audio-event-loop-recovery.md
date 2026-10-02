# Original audio C03/C09: bounded event loop wait and capture start recovery evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_audio_event_loop_recovery_confirmed.py) inspect frozen WASAPI audio event loop waiting and capture startup orchestration logic. They do not initialize WASAPI audio devices or stream live audio. C03 and C09 remain confirmed mechanisms.

## C03 — `wait_for_event` timeout handling without recovery transition

In `audio.rs`, [capture_thread](<../../../overlay-backend/src/audio.rs#L410-L424>) waits on the audio buffer event using:
`if event.wait_for_event(250).is_err() { ... continue; }`.
When timeouts occur (e.g. device sleep, disconnected Bluetooth headphones that do not fire IMM notification callbacks), it emits a periodic 60-second heartbeat log (`audio_no_frame heartbeat idle_s=... action=wait`) and continues waiting.
The loop does not treat prolonged absence of events as a recovery trigger (`CaptureExit::Recover`) or attempt device re-initialization.

## C09 — `start_capture` immediate return and internal retry loop

In `audio.rs`, [start_capture](<../../../overlay-backend/src/audio.rs#L154-L199>) spawns background threads for `audio-system` and `audio-mic`, immediately returning `Ok((rx, CaptureHandle { stop }))` without waiting for initial hardware device acquisition or format verification.
Inside [capture_with_recovery](<../../../overlay-backend/src/audio.rs#L203-L263>), device opening errors or `capture_thread` failures are caught and retried in a loop sleeping 1 second between attempts while `!stop.load(Ordering::Acquire)`.
When the stop signal is set, `capture_with_recovery` terminates the loop and unconditionally returns `Ok(())`.

## Limits

No native Windows audio endpoints were disconnected, no Bluetooth audio sleep was simulated, and no WASAPI device invalidations were executed. Original statuses in `candidates.json` remain `confirmed`.
