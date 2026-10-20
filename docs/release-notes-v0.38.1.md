# Suflyor v0.38.1

Stable Windows release after the owner's successful rc.15 retest and explicit
request to publish. The optional Strata Vision add-on remains experimental;
stable publication does not broaden its model compatibility.

## Changes since v0.38.0

- Local-model downloads are separate from activation. Closing and reopening
  Settings retains progress and Cancel. Downloading does not change text,
  screenshot or speech-recognition routes, and cloud startup does not load
  downloaded local weights. Explicit Use starts and verifies the selected model
  before saving its route, including the actual fallback when necessary.
- Accelerated local-engine updates prefer a complete compatible release rather
  than a newer partial CPU-only upload.
- The OpenAI Settings test button no longer overlaps the download card in
  English or Russian. Narrow-window geometry has regression coverage.
- Buffered OpenAI Responses answers remain visible when a bridge sends the
  completed answer without preceding text deltas. Ordinary streamed answers
  are not duplicated.
- Deferred presentation respects hidden windows, and retained Settings can
  recover from an early close during native presentation. The owner's separate
  multi-monitor transparent-rectangle report is not claimed independently
  reproduced or resolved.
- Memory telemetry shares one footer line with the active model inside the
  fixed-height overlay bar. The existing strict EN/RU geometry test is retained.
- The Windows-only Strata setup constant is gated with its Windows-only caller
  so the macOS full Clippy gate does not report it as unused. Already-cancelled
  macOS push-to-talk returns before starting native capture.

## Optional Strata Vision, Windows

Settings > Screenshots (F8) offers **Add Vision to Strata** for a compatible
original Qwen3.8-Flash-Next installation and its bundled image helper.
**Download / Resume** fetches only the matching 907 MB encoder. It does not add
a second language model or server, change routes, edit the run configuration,
or activate the model. Progress and cancellation survive Settings close/reopen.

First use Strata for text, then choose **Enable for F8**. Activation configures
CPU image encoding without an additional GPU memory reservation, verifies a
synthetic image, and only then saves the screenshot route. Failure restores the
previous run configuration. Existing custom encoder settings are preserved.
The encoder is pinned to an immutable revision and verified by SHA-256.

## Evidence and limits

The owner reports rc.15 works. This is owner-reported acceptance, not an
agent-captured real Strata inference trace. The native-worker evidence covers
real encoder bytes, cancellation/resume, download-only semantics and deliberate
failed-start rollback using synthetic prerequisites. Independent real-model
inference, successful activation persistence and recovery of a previously
running real Strata server remain unverified on the available Windows worker.

The add-on does not support arbitrary projectors or model families and does not
add Strata Vision to macOS. Physical macOS UI, audible selected-text playback,
and the owner's actual provider wire behavior are not claimed verified by
synthetic fixtures or shortcut dispatch alone. Windows builds remain unsigned.

Exact stable gate, installer and publication results are recorded separately in
`docs/release-evidence-v0.38.1/README.md`.
