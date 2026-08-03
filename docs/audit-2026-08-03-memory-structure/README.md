# Memory structure preservation audit — 2026-08-03

## Tester-visible failure

An explicitly saved four-row DRP schedule was later shown as a shortened prose
fact. The original tester screenshot is not committed because it contains
private desktop/email context.

## Root cause

All explicit Tile and Transcript saves shared `insert_approved_note`. That
function first collapsed whitespace and newlines, then asynchronously replaced
the approved text with at most three AI-selected facts (or two heuristic
clauses). A grounded rewrite could therefore still delete valid table rows by
design. Boot and Memory-tab sweeps retried the same rewrite for legacy pending
rows.

## Source matrix

| Source | Before | After |
| --- | --- | --- |
| Tile marked block | whitespace collapsed, then 1–3 facts | outer trim only |
| Tile selected span | whitespace collapsed, then 1–3 facts | outer trim only |
| Multiple Tile blocks | joined with `; `, then condensed | joined with newlines, stored verbatim |
| Transcript marked/selected text | condensed after approval | stored verbatim |
| Settings typed fact | stored verbatim | unchanged |
| Extracted candidate | reviewed before approval | unchanged |
| Legacy normalized item | raw shown only as provenance | explicit **Restore original** action |
| Legacy pending item | retried at boot/tab open | never rewritten automatically |

## Synthetic before/after fixture

Input and expected stored text (four lines):

```text
BI-Портал | ВС | Финансы | 03.08.2026 – 09.08.2026
АльфаОтчетность | ВС | Финансы | 07.09.2026 – 13.09.2026
АльфаРепликация | ВС | Финансы | 14.09.2026 – 20.09.2026
Финансовое хранилище данных | ВС | Финансы | 14.09.2026 – 20.09.2026
```

The previous AI path had a hard maximum of three facts; the heuristic fallback
kept two clauses. The regression test now asserts byte-for-byte preservation of
the four inner lines (only surrounding whitespace is removed).

## Checks

- `approved_note_preserves_structured_schedule_verbatim`
- `join_marked_text_combines_marked_only_in_order`
- `restore_memory_item_source_is_explicit_and_lossless`
- Exact Slint MCP build SHA-256:
  `01AB437EF36BFB778498556215271C1D93E46AC767535EF0B103D39E9AE798A0`.
- Embedded server identified itself as `slint-mcp-embedded` and exposed 14
  tools. Settings was checked at 720x600; the Memory tab had no clipping,
  overlap, or unreachable controls.
- Existing Memory entries did not contain restorable provenance. The private
  real-data screenshot was inspected but deliberately not committed, and no
  synthetic row was injected into the user's database. The restore state is
  covered by the storage regression test above.
- All 13 global shortcuts registered and produced a distinct dispatch result:
  `F1`, `F3`, `F4`, `F6`, `F7`, `F8`, `Shift+F8`, `Ctrl+F8`, `F9`,
  `Shift+F9`, `Shift+Alt+1`, `Shift+Alt+2`, and `Shift+Alt+3`.
  `Shift+Alt+1` copied 28 characters from a disposable Notepad selection.
- Read-only Qwen reviews used `qwen3.8-max-preview`: approve, confidence 0.90,
  no blockers. Its final non-blocking suggestion was also applied: a manually
  edited item (`norm_status='none'`) cannot be overwritten by Restore.
- Full repository gate: `All gating layers green` (2026-08-03).
