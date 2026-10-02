# Original audio C01/C07: bounded stateless resampling and mix format negotiation evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_audio_resample_format_confirmed.py) inspect frozen audio capture and decimator sources. They do not initialize WASAPI/CoreAudio devices or capture live audio. C01 and C07 remain confirmed mechanisms.

## C01 — stateless decimator remainder dropping and per-chunk phase reset

In `audio.rs`, [resample_and_quantise](<../../../overlay-backend/src/audio.rs#L803-L834>) is a per-call stateless average-decimator taking `&[f32]` and `ratio: f64` without an accumulator for fractional phase.
In the 3:1 fast path, `full_chunks = input.len() / 3` drops trailing samples (`len % 3`).
In the main capture loop of [capture_thread](<../../../overlay-backend/src/audio.rs#L446-L454>), `f32_buf.clear()` is called immediately after emitting each ~200ms chunk, discarding any fractional residual.

## C07 — WASAPI autoconvert request without read-back of negotiated client format

In `audio.rs`, [capture_thread](<../../../overlay-backend/src/audio.rs#L348-L365>) and [record_source_until_stop](<../../../overlay-backend/src/audio.rs#L540-L555>) call `client.initialize_client(&desired, &init_dir, &mode)` requesting 32-bit float mono at native rate with `autoconvert: true`.
Neither function queries the audio client's actual negotiated format after initialization.
In `capture_thread`, [byte processing](<../../../overlay-backend/src/audio.rs#L425-L445>) clears `byte_q` at the beginning of each iteration, so any non-multiple-of-4 byte read from the device queue is discarded.

## Limits

No native Windows audio drivers or loopback endpoints were opened, and no live acoustic stream phase was measured. Original statuses in `candidates.json` remain `confirmed`.
