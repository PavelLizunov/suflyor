# Original audio C02/C04: bounded clock and route evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_audio_clock_route_hypotheses.py) use pure arithmetic and frozen source. They do not open devices, capture audio, or execute Rust. C02/C04 remain hypotheses.

## C02 — timestamps are not sample positions

[Windows capture](<../../../overlay-backend/src/audio.rs#L440-L470>) stamps chunks with `Instant::elapsed`, and the macOS worker does the same. [Resample](<../../../overlay-backend/src/audio.rs#L803-L834>) floors non-integer ratios and drops a remainder in the exact 3:1 path. The numerical model shows a 2.999 ratio changes output length; it does not measure a real device clock.

## C04 — only console default changes recover

[Recovery decision](<../../../overlay-backend/src/audio_route.rs#L160-L205>) requires `RouteRole::Console` for default-device recovery. [Role mapping](<../../../overlay-backend/src/audio_route.rs#L70-L90>) sends every non-console role to `Other`, and [device added](<../../../overlay-backend/src/audio_route.rs#L213-L220>) does not send a notification. Pinned selections ignore default changes. The retry loop has no attempt ceiling.

## Limits

No WASAPI/CoreAudio device, native notification, or long-running capture was tested. Original statuses and 39/75/5 remain unchanged.
