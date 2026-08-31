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
