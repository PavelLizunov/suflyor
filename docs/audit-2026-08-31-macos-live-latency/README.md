# macOS live latency and memory UI audit

## Scope

- Baseline: `d7949618b8dfe35f1b0473fc0900436d4a0bf59b`
- Candidate UI: `a7db6fd78936d6e5c10fdd9096df4cde305ad8dd`
- Capture path: exact candidate built with `--features ui-mcp` and `SLINT_EMIT_DEBUG_INFO=1`, captured through Slint MCP
- Environment: Apple Silicon macOS, Graphite scheme, system display scale, same saved configuration for before/after
- Locales: English and Russian
- Full bar: 1280×64; compact bar: 680×64

## Evidence map

| State | English | Russian |
|---|---|---|
| Baseline full | `before-en-full.png` | `before-ru-full.png` |
| Baseline compact | `before-en-compact.png` | `before-ru-compact.png` |
| MLX loading full | `after-en-loading-full.png` | `after-ru-loading-full.png` |
| MLX ready full | `after-en-ready-full.png` | `after-ru-ready-full.png` |
| MLX ready compact | `after-en-ready-compact.png` | `after-ru-ready-compact.png` |
| Completed request full | `after-en-perf-full.png` | `after-ru-perf-full.png` |
| Completed request compact | `after-en-perf-compact.png` | `after-ru-perf-compact.png` |

## Results

- Immediate background prewarm reached `ready` in both locales without blocking the bar.
- Full ready/request states show `App RAM` / `RAM приложения` and `MLX unified` / `MLX: единая память`; no VRAM wording is used.
- The earliest loading frame may omit MLX memory while no sidecar PID is measurable. This is intentional rather than a fabricated zero value.
- Completed-request captures show model load, first-token latency, total latency, decode tok/s, and end-to-end tok/s in both locales.
- Compact mode intentionally elides memory values while retaining the latency/throughput line without overlap.
- All 14 captures have the expected dimensions. No clipping, overlap, tofu, paths, URLs, usernames, secrets, or private transcript content was observed.
- The Russian baseline already contains the English dynamic labels `recording` and `local:`; the same inherited mixed-language labels remain after the change. The new memory and performance labels themselves are localized.
- Functional macOS hotkey smoke: 13/13 registered actions dispatched. F4 and F7 were retried with explicit key events after the first synthetic dispatch was not observed.

## Bounded production-path benchmark

The approved three-cell run used `overlay_backend::ai::stream_chat_endpoint` and the managed MLX lifecycle, then stopped without tuning:

| Cell | Load | TTFT | Total | Decode | End-to-end | MLX memory |
|---|---:|---:|---:|---:|---:|---:|
| Cold | 34.808 s | unavailable | 37.527 s | 89.663 tok/s | 2.558 tok/s | 4.71 GiB |
| Warm | resident | unavailable | 1.227 s | 87.709 tok/s | 78.196 tok/s | 4.71 GiB |
| Long synthetic context | resident | unavailable | 9.992 s | 79.884 tok/s | 9.607 tok/s | 6.09 GiB |

The benchmark stream reported completion-token metrics but no visible content delta, so TTFT is recorded as unavailable rather than inferred. Total GPU-active benchmark time was 50 seconds, below the approved 15-minute ceiling.

## Follow-up auto-tile output-budget screen

A bounded follow-up used the installed native Swift sidecar and its production `/v1/chat/completions` streaming API with the pinned LFM model, production chat template, temperature `0.2`, and one fixed concise-answer prompt. Python was used only as the benchmark HTTP harness; the measured inference process remained the Swift/Metal sidecar.

| Max output tokens | Visible characters | TTFT | Total | Finish |
|---:|---:|---:|---:|---|
| 512 | 0 | unavailable | 6.034 s | length |
| 1024 | 509 | 5.835 s | 7.034 s | stop |
| 2048 | 451 | 6.096 s | 7.237 s | stop |
| 4096 | 481 | 9.659 s | 11.523 s | stop |

The 1024-token cell was the smallest successful budget and the fastest successful total in this screen, so the auto-tile path uses 1024 rather than the earlier 512 cap or the manual ask path's 4096 cap. This single-prompt screen establishes truncation and latency for the selected production path; it does not claim general model quality or load capacity. Total measured generation time was about 32 seconds and remained below the approved 15-minute GPU ceiling.
