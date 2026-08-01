# 2026-07-31 audit delivery index

Status snapshot: 2026-08-01. The pull requests below are the independent
delivery branches used for review and integration. PR #50 is the pre-existing
context-preset fix and was checked before this delivery so its work was not
duplicated. No release or tag was created by this audit delivery.

| Audit findings | Delivery |
|---|---|
| K1–K9, K12, K13, A2, F8 comment | [#51 — release/docs alignment](https://github.com/PavelLizunov/suflyor/pull/51) |
| F1, F2, F7 | [#52 — TTS suppression lifecycle](https://github.com/PavelLizunov/suflyor/pull/52) |
| L1, L3–L7 | [#53 — runtime deduplication](https://github.com/PavelLizunov/suflyor/pull/53) |
| D2, D3, L2 | [#54 — visible cost-cap notices](https://github.com/PavelLizunov/suflyor/pull/54) |
| C2, G1–G3 | [#55 — persistence work off the UI thread](https://github.com/PavelLizunov/suflyor/pull/55) |
| C1, C3 | [#56 — ordered session stop on exit](https://github.com/PavelLizunov/suflyor/pull/56) |
| I1–I4 | [#57 — verified stealth state and failures](https://github.com/PavelLizunov/suflyor/pull/57) |
| H1–H3 | [#58 — diarization install/window lifecycle](https://github.com/PavelLizunov/suflyor/pull/58) |
| Handy-informed STT model choice follow-up | [#59 — guided STT model selection](https://github.com/PavelLizunov/suflyor/pull/59) |
| Screenshot/UX clarity follow-up | [#60 — help, archive, tiles, Diagnostics](https://github.com/PavelLizunov/suflyor/pull/60) |
| D1, D4 | [#61 — journal response semantics](https://github.com/PavelLizunov/suflyor/pull/61) |
| J1, J2 | [#62 — Slint i18n guard extension](https://github.com/PavelLizunov/suflyor/pull/62) |
| E1, E2, E5 | [#63 — audio-source observability](https://github.com/PavelLizunov/suflyor/pull/63) |

## Visual evidence rule

Every visual fix must keep matching before/after screenshots of the same
surface, dimensions, theme, language, and state at a stable artifact path and
link both from its PR. An after-only screenshot is not acceptance evidence.

The original reports predate this rule and arrived as differently sized crops;
their source evidence is preserved here rather than recreated:

| Surface | Before | After |
|---|---|---|
| Help | [before](visual/ux-clarity/before-help.png) | [after](visual/ux-clarity/after-help.png) |
| Archive | [before full](visual/ux-clarity/before-archive-full.png), [before metadata](visual/ux-clarity/before-archive-meta.png) | [after](visual/ux-clarity/after-archive.png) |
| Empty tile | [before](visual/ux-clarity/before-tile.png) | [after](visual/ux-clarity/after-tile.png) |

## Intentionally still open

The delivery does not claim that every audit item is fixed. Remaining or
conditional work includes B1/B2, C4, E3/E4, F3–F6, G4/G5, K11, L8–L10, and
runtime-only acceptance gaps recorded in the individual PR descriptions.
Those items need their own scoped branches if prioritised; they were not folded
into unrelated diffs.

The complete 2026-07-31 Qwen audit remains the source of truth for finding
descriptions and evidence. This file is only the public routing/index layer.
