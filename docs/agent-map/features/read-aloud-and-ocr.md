# Read-aloud and OCR: source-linked feature contract

**Evidence:** source inspection at `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`; no native speech, clipboard, screenshot, installer or inference run executed. This documents selection/OCR-to-speech chains and process boundaries, not recognition quality or UI acceptance.

## User surfaces and config

[Read-aloud hotkey dispatch](../../../slint-experiment/src/bin/overlay_host_windows.rs#L2687-L2809): Shift+Alt+1 copies current selection using native clipboard input; Shift+Alt+2/Ctrl+F8 starts region OCR; pause uses current speech target. Clipboard snapshot/restore here is text-only, not history scrape or preservation of all clipboard formats.

[Text tile](../../../slint-experiment/src/bin/overlay_host/read_aloud.rs#L20-L116) creates selectable/copyable/readable content and can retain a closed tile for restore. Marked or selected sensitive text remains in those UI/data structures by design; no password-detector/privacy guarantee is established. [OCR result fill](../../../slint-experiment/src/bin/overlay_host/read_aloud.rs#L193-L263) renders returned text, seeds minimal conversation state and auto-reads on success; empty text displays a generic no-text notice and disables dead speak/copy controls. OCR text is not rewritten by a model on this path.

[Voice Settings](../../../slint-experiment/src/bin/overlay_host/settings_voice.rs#L100-L184) stores namespaced `piper:<dir>`/`tera:<style>`, engine, synthesis rate and applies runtime change. Save-failure/runtime consistency remains separately checked. [TTS initialization](../../../slint-experiment/src/bin/overlay_host_windows.rs#L519-L535) creates global client at boot with saved engine/voice/language. A constructor alone does not guarantee sidecar preload; see implementation below.

## Sidecar routing and protocol

[Tts::spawn](../../../overlay-backend/src/tts.rs#L1071-L1124) constructs two sidecar handles, installed voice list and selection. [Global init](../../../overlay-backend/src/tts.rs#L1431-L1456) then **calls `tts.warm()` before storing the client**; [warm](../../../overlay-backend/src/tts.rs#L1404-L1428) starts selected usable engine with seeded voice/rate/speed. [ensure](../../../overlay-backend/src/tts.rs#L864-L940) also supports lazy respawn on commands. Earlier inference that constructor-only startup omitted warm was incorrect; successful model preload and actual first-speech latency still need native evidence.

[Tts::speak](../../../overlay-backend/src/tts.rs#L1236-L1282) converts markup to spoken text, Base64-encodes UTF-8, prefers ready Tera, then falls back to Piper if unavailable/not accepting the command. [pause/resume/stop/seek/speed/rate](../../../overlay-backend/src/tts.rs#L1284-L1428) operate against last target or engine fallback; selected engine and accepted last speech target are distinct state.

[Protocol write](../../../overlay-backend/src/tts.rs#L968-L1044) registers pending playback generation then writes/flushed one line while holding sidecar lock; [raw writer](../../../overlay-backend/src/tts.rs#L950-L961) clears stdin/process handles on write failure. [Tracker](../../../overlay-backend/src/tts.rs#L483-L562) maps sidecar event ID to FIFO pending generation; [speaking state](../../../overlay-backend/src/tts.rs#L285-L420) tracks load/playback/STT-suppression status. Missing reader/pipe stall/queue mismatch consequences require exact reachable preconditions and large-input tests; no global bounded-text guarantee follows from eventual sentence chunking.

[Piper stdin/events](../../../suflyor-tts/src/main.rs#L123-L352) reads a bounded-count command queue, loads selected engine/voice, synthesizes chunks and emits READY/STARTED/DONE/FAILED. It links sherpa-onnx only. [Tera controller](../../../suflyor-teratts/src/main.rs#L1-L25) additionally advertises engine/revision/voices/sample rate/ready state; its generation/controller logic is separate and does not imply identical Piper semantics.

[Child spawn](../../../overlay-backend/src/tts.rs#L1594-L1602) applies hidden process flags and **attaches Windows lifetime JobObject**. Earlier blanket no-attach report is disproved by code; best-effort assignment failure and non-Windows forced-parent exit remain native checks. [Sidecar log routing](../../../overlay-backend/src/tts.rs#L1566-L1591) appends stderr to data-root logs; response content/error safety must be reviewed per engine, not assumed by process isolation.

## Speech text and playback

[Piper engine](../../../suflyor-tts/src/engine.rs#L13-L96) wraps OfflineTts, voice/token assets and `text::chunk_text` in a synthesis worker feeding playback; selected voice/rate are model inputs, not just UI decoration. [Playback modules](../../../suflyor-tts/src/playback.rs) and [macOS transport](../../../suflyor-tts/src/playback_macos.rs) consume mono float PCM and WSOLA time stretch. Tera has its own [Windows](../../../suflyor-teratts/src/playback.rs)/[macOS](../../../suflyor-teratts/src/playback_macos.rs) transports. [WSOLA](../../../suflyor-wsola/src/lib.rs) is a Rust library, not another process.

[STT anti-feedback](../../../overlay-backend/src/stt.rs#L379-L445) drops buffers while speaking suppression is active. It is an intended safety boundary, not proof that false activity flags cannot suppress real meeting speech. [Tracker tests](../../../overlay-backend/src/tts.rs#L2087-L2123) declare stopped/failed/rejected/EOF handling but are not executed passes here.

## OCR paths

[Platform dispatch](../../../slint-experiment/src/bin/overlay_host/vision_capture.rs#L78-L100) uses Tesseract child on Windows and Apple Vision on macOS. [Host local OCR branch](../../../slint-experiment/src/bin/overlay_host/vision_capture.rs#L646-L690) schedules off-thread processing and fills its placeholder tile; this branch bypasses AI endpoint/HTTP/image request. [Normal entry](../../../slint-experiment/src/bin/overlay_host/vision_capture.rs#L153-L187) captures no endpoint while local OCR is ready. The later launch helper rechecks availability, so an endpoint-bearing alternate caller or availability change can follow generic vision routing; do not promise unconditional no-network behavior solely from mode label.

[Tesseract OCR](../../../overlay-backend/src/ocr.rs#L96-L139) checks zero dimensions/overflow/undersized BGRA, encodes an in-memory BMP, launches hidden child using stdin/stdout and writes image on a thread while draining output. It does **not** write an OCR image/text tempfile in this current path. [Spawn wait](../../../overlay-backend/src/ocr.rs#L132-L153) is blocking with no explicit elapsed timeout in this helper. Comments calling OCR deterministic/no-hallucination are not measured accuracy guarantees; dimension validation limits input shape, not OCR accuracy.

## Install and license boundaries

- [Piper voice packs](../../../overlay-backend/src/tts_install.rs#L48-L65) pin Irina/Ruslan archive SHA; [installer](../../../overlay-backend/src/tts_install.rs#L98-L198) independently installs missing packs and returns partial success if at least one is usable. Existing pack readiness is model/tokens presence, not every-use rehash.
- [Tera manifest validation](../../../overlay-backend/src/teratts_install.rs#L38-L95) validates paths/digests/revision. [Ready state](../../../overlay-backend/src/teratts_install.rs#L120-L154) uses marker plus exact sizes; [install publish](../../../overlay-backend/src/teratts_install.rs#L237-L289) hashes downloads then writes marker/staging and renames. Marker is not a recurring full byte hash audit.
- [OCR installer](../../../overlay-backend/src/ocr_install.rs#L24-L83) pins archive but extracts to live data root; executable/traineddata readiness is sentinel-based.
- Model weights are on-demand, not bundled in NSIS. [Tera NOTICE](../../../suflyor-teratts/NOTICE.md) preserves unresolved upstream license grant/release boundary. Process isolation prevents runtime collisions, not automatic code/weight/style license clearance.

Source-declared install/protocol/textnorm/playback tests do not substitute exact-SHA native playback, command cancellation, slow/broken pipe, clipboard restoration, own-window capture exclusion, no-TCC permission behavior, on-demand install failure and localized status/reset/functional hotkey QA. See [Grok register](../reconciliation/candidates.json) for explicit hypotheses and counterevidence.
