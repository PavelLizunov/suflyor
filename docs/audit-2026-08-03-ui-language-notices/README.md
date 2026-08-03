# UI language vs AI response language audit

## Scope and fixed invariant

The reported state used English application UI with Russian AI response
language. Deterministic application chrome, notices, statuses, and errors must
follow `ui_language`; prompts and generated answers must continue to follow
`response_language`.

The private reporter screenshot was inspected but is not committed. Repository
evidence uses only empty-transcript, missing-token, and quiet-microphone states.

## Reproduction conditions

- Suflyor 0.35.3 debug QA build with `--features ui-mcp`
- `SLINT_EMIT_DEBUG_INFO=1`, MCP port 9123
- Light Frost, 100% scale, 460×360 tiles
- English UI with Russian AI response language for the reported mismatch
- Empty transcript, cloud token unavailable, and quiet microphone

## Before / after evidence

| State | Before | English after | Russian after |
| --- | --- | --- | --- |
| F6 empty transcript | [`before-f6.png`](before-f6.png) | [`after-f6-en.png`](after-f6-en.png) | [`after-f6-ru.png`](after-f6-ru.png) |
| Shift+F9 missing cloud auth | [`before-shift-f9.png`](before-shift-f9.png) | [`after-shift-f9-en.png`](after-shift-f9-en.png) | [`after-shift-f9-ru.png`](after-shift-f9-ru.png) |
| Push-to-talk, quiet mic | Code-audit finding: hardcoded Russian chrome | [`after-ptt-en.png`](after-ptt-en.png) | [`after-ptt-ru.png`](after-ptt-ru.png) |

The final PTT captures came from exact QA binary SHA-256
`5562C6C6CD435AD5C2E5CD7BC07037C2238026F476C4733923F10DC89783A972`.
The final F6 and Shift+F9 English captures came from exact QA binary SHA-256
`5C0ED44D36C60EC8788E8889B01EE73942C02FC8F69B39EAFAA7239100E09EB8`;
the later change only localized the PTT provenance chip and Russian empty-summary
wording and did not touch those two paths.

## Verification

- Slint MCP element trees and pixels agree for F6, Shift+F9, and PTT in both
  English and Russian; English was restored before handoff.
- The exact MCP-enabled QA binary dispatched all 13 registered shortcuts:
  F1, F3, F4, F6, F7, F8, Shift+F8, Ctrl+F8, F9, Shift+F9,
  Shift+Alt+1, Shift+Alt+2, and Shift+Alt+3. Capture surfaces were cancelled
  with Esc. Shift+Alt+1 copied 26 characters from disposable selected text.
- `scripts/ci.ps1`: `All gating layers green` after the final change.
- Qwen `qwen3.8-max-preview` first found the missed PTT path, then returned
  `APPROVE` on the completed semantic split. Worker artifacts:
  `C:\Users\x3d_mutant\Natively\ai-worker-results\suflyor-20260803\ui-language-notices\qwen-review.txt`
  and `qwen-final-review.txt`.

Generated summary/debrief content was not sent to an external AI during this
audit, so those end-to-end content states were not live-tested. Their
deterministic titles/notices are covered by unit tests; generated content still
uses the separate AI response language.
