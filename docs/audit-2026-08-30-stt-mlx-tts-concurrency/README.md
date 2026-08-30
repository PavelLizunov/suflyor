# macOS STT + MLX + TTS bounded concurrency evidence

Date: 2026-08-30

## Decision question

Can the production GigaAM CPU, managed MLX text, and Piper read-aloud seams run
at the same time on the 16 GiB Apple Silicon worker without a crash, timeout,
swap, thermal warning, or unbounded process retention?

This was a reliability/resource screen, not a model-quality or sustained-load
comparison.

## Provenance

- Code commit: `89fc0e0503a029a1eff91c771150c02a411488b6`
- Tree: `64e0eceb29927be52dd3f8323f13a0c3a849951d`
- GigaAM: production CPU backend and the committed five-cycle lifecycle probe
- MLX model: `LiquidAI/LFM2.5-8B-A1B-MLX-4bit`, pinned revision
  `2e92b640a63d47ad4dcf81a19a366b902356b3bc`
- MLX weights SHA-256:
  `3cc15631acc1894b3584ac11fb4122beee50b604e1a0be575da686cee87aa3a4`
- TTS: production Piper sidecar with the installed Irina medium voice

No model, package, or benchmark corpus was downloaded for this run.

## Production paths

1. The MLX sidecar was started with its authenticated parent startup envelope.
   Identity was checked from `READY`, then non-streaming requests used
   `/v1/chat/completions` with the selected model and server chat template.
2. The Piper process used its normal `SPEAK` stdin protocol and emitted
   `READY`, `STARTED`, and `DONE`.
3. GigaAM ran the production one-shot/live shared-cache lifecycle probe over the
   known 16 kHz fixture, including five load/start/stop/cache-reset cycles.
4. A supervisor sampled direct child RSS through the concurrent window and
   verified clean process exit afterward.

## Corrected smoke

The first MLX prompt was invalid for this reasoning model because a 48-token
output cap ended at `finish_reason=length` before visible content. A one-case
correction with a 512-token cap completed at `finish_reason=stop`, returned a
non-empty answer in 1.510 seconds, and used 114 completion tokens.

A virtual BlackHole output did not advance Piper playback to completion. It was
excluded as a device-clock artifact. The same production TTS command on the
normal output emitted first audio in 370 ms, accepted `STOP`, emitted `DONE`,
and exited cleanly.

## Concurrent result

| Check | Observed result |
|---|---:|
| Total bounded run | 39.857 s |
| MLX request | non-empty, `stop`, 114 completion tokens, 1.424 s |
| GigaAM lifecycle | exit 0, five cycles, no context-leak warning |
| Piper lifecycle | `READY` → `STARTED` → `DONE`, clean exit |
| Peak MLX RSS | 3031.0 MiB |
| Peak Piper RSS | 230.6 MiB |
| Peak GigaAM RSS | 699.6 MiB |
| Peak combined child RSS | 3961.0 MiB |
| Swap after run | 0 MiB |
| System memory free after run | 82% |
| Thermal/performance warning | none recorded |
| Remaining MLX/TTS/STT processes | 0 / 0 / 0 |

## Decision

**Supported for this bounded screen:** the three production model seams can be
resident and make progress concurrently within the observed resource envelope.
No product-code fix was required.

## Limits

- This is a roughly 40-second bounded concurrency screen, not a multi-hour soak.
- It does not compare model quality, streaming/non-streaming TPS parity, or
  alternate MLX weights/KV configurations.
- GigaAM input was the synthetic known-answer fixture, not live TCC capture.
- RSS values cover the three direct model processes, not total system RSS.
