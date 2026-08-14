# RC13 AI provider UI audit

Date: 2026-08-09  
Environment: Winbrat Windows VM, 1280 px desktop, Slint MCP build  
Baseline: RC12 `7025880e9452fcad07f10a1d625d4943465c421d`  
Candidate: RC13 `55c3a7fcbb6a40574f2961ec0b18b44a28656131`

## Result

- Settings remained exactly 720 x 600 at scale 1.0.
- The provider selector exposed four reachable modes: custom OpenAI-compatible,
  local OpenAI-compatible, OpenAI Responses, and Anthropic Messages.
- Local settings remained compatible with the RC12 layout and retained their
  existing values.
- OpenAI and Anthropic direct-provider pages fit without clipping at 720 x 600.
- Both direct providers were exercised with non-secret dummy values through
  `save -> stored securely -> blank save/delete -> not set`.
- Direct-provider inputs were blank again after save. No credential value was
  exposed in MCP properties, screenshots, configuration exports, or this audit.
- English and Russian pages were checked; the original English + Local selection
  was restored after the audit.
- The custom bridge page was inspected but its screenshot is intentionally not
  retained because the VM configuration contains a private endpoint.

## Evidence

- `before-rc12-ai-tab.png` — RC12 Local baseline.
- `after-rc13-local.png` — compatible RC13 Local page.
- `rc13-provider-popup.png` — all four provider choices.
- `after-rc13-openai-stored.png` — OpenAI credential stored status and page fit.
- `after-rc13-anthropic-stored.png` — Anthropic credential stored status and page fit.
- `after-rc13-openai-ru.png` — Russian direct-provider page.

The VM app was stopped after capture. No application instance was launched on
the owner workstation.
