# STT question-boundary UI audit

Synthetic-only visual and timing evidence for the approved system-audio segmentation and auto-tile question extraction fix.

## Fixed conditions

- Worker: `mac-worker` (Apple Silicon macOS)
- Baseline source: `d17d849f09ae3c2ec1e4ee59c4f3201522441b32` (`v0.38.1-rc.1`)
- Candidate source: `95eab80623b670103ada55ac561d00a128e08bbe`
- Language: Russian
- Theme, display arrangement, scale, window placement: unchanged between captures
- Configuration: managed MLX, GigaAM STT, auto-tile every-line enabled, mic-trigger skip enabled
- Input: synthetic non-private Russian statement plus one question
- UI capture: exact feature-enabled Slint-MCP binary, 460×360 tile at scale factor 1
- Render fixture: the baseline title was submitted through Text Ask because the unbundled QA binary's system-audio tap produced zero-RMS chunks; deterministic runtime tests cover the auto-tile wiring

## Evidence

- `before-auto-tile.png`: baseline tile rendering the untrimmed synthetic statement and question.
- `after-auto-tile.png`: candidate tile rendering only the extracted question.
- `timing.txt`: source SHAs, production-safe aggregate timing, focused test results, and audit limitations.

No credentials, local paths, private session titles, or real transcript text are included.
