# Release evidence — v0.36.0

Base: `origin/master` at `7fe425c`. The release branch changes only version
metadata, release documentation, and the attached visual evidence.

## Automated gates

- `scripts/ci.ps1`: `All gating layers green`.
- Qwen `qwen3.8-max-preview`: v0.36.0 is justified rather than v0.35.4;
  confidence 0.90.
- Worker artifact:
  `C:\Users\x3d_mutant\Natively\ai-worker-results\suflyor-20260804\release-0360-scope-review\stream.jsonl`.
- Final Qwen review: `APPROVE`, no blocking findings, confidence 0.92. Artifact:
  `C:\Users\x3d_mutant\Natively\ai-worker-results\suflyor-20260804\release-0360-final-review\stream.jsonl`.

## Built artifacts

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `overlay-host.exe` | 69,749,760 | `368E2EAFD58995C78F23B799BE3AADA626124FCD1F122362CC12B5A23C60AA99` |
| `suflyor-tts.exe` | 18,761,728 | `AD33E54A130CD62B93B7F8B18403BC8A065D25B9872C16A3E3794A9CE79B6110` |
| `suflyor-slint-setup.exe` | 23,522,696 | `2E6A161DD86D19F4BFCD3694EAC42182D4AC041596D986255A2DA2E779AB16C8` |

`overlay-host.exe` reports file version 0.36.0 and product version 0.36.0.
Silent installation completed successfully; installed host and sidecar hashes
match the release bundle.

## Live UI and runtime checks

- Slint MCP: exact `ui-mcp` binary, Settings > Updates, 720x600, Light Frost,
  100% scale, English and Russian. Version 0.36.0 was visible without clipping;
  English was restored afterward.
- Before/after images are in
  [`../audit-2026-08-04-release-0360/`](../audit-2026-08-04-release-0360/).
- Exact QA binary: F1 opened Help, F4 opened Knowledge palette, and F7 opened
  Session archive through real Windows global-hotkey input.
- Not repeated on this version-only branch: F3, F6, F8, Shift+F8, Ctrl+F8,
  F9, Shift+F9, Shift+Alt+1, Shift+Alt+2, and Shift+Alt+3. Their handlers were
  unchanged and passed the underlying master audit.
- Installed normal binary starts and responds with MCP port 9123 closed. The
  bar and Settings HWNDs both have `WS_EX_TOOLWINDOW` set and
  `WS_EX_APPWINDOW` clear.
- Installed bar captures at launch and five seconds later stayed 1200×64 at
  `(360, 24)` with no missing glyphs or resize loop.
- The two-step Quit action stopped both the host and TTS sidecar in 1,145 ms;
  the normal installed binary was relaunched for owner acceptance.

No tag or GitHub Release was created while collecting this evidence.
