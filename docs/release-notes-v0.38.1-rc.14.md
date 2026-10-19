# Suflyor v0.38.1-rc.14

## Changes

- Keep buffered OpenAI Responses answers. If a bridge provides the answer only in its terminal response snapshot, the text reaches the auto tile instead of being treated as an empty reply and closing it. Normal text-delta streams are not duplicated.
- Size the OpenAI provider block using its wrapped content. The connection-test button no longer overlaps the local-model download card in English or Russian.
- Skip delayed native window setup after a window has been closed. Retained Settings uses the existing native hide fallback and recovers a cancelled off-screen first reveal on reopening. Active downloads still retain their window and callbacks.

## Verification scope

The release uses the selected Windows native gate, not the stable-only full gate. Regression checks cover buffered, ordinary streamed, empty and incomplete Responses output; English/Russian provider geometry at720,600 and480px; retained Settings download lifecycle.

Native UI acceptance uses the feature-enabled QA executable and synthetic transcript/Responses fixtures. These fixtures reproduce the disappearing auto-tile path, but are not a wire trace of the owner's Gemini bridge.

## Known limits

- The owner's large transparent background rectangle is not reproduced on the single-monitor software-rendered Windows worker. The closed-window guards address a source-backed delayed-reveal risk, not a claim of verified multi-monitor compositor repair.
- The reported local-capabilities caption clipping is not reproduced after scrolling to the end; it is not presented as fixed by the OpenAI layout change.
- The previously disclosed full overlay-bar geometry failure remains outside these repairs. The full suite and physical macOS UI acceptance are not claimed green.
- External model availability, provider refusals and actual speech/audio output are distinct from local UI and dispatch checks.
