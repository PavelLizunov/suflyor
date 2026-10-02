# Navigation gap ledger and evidence integrity

**Result:** all 298 selected source files have accepted bounded parser receipts, but only **156 have any precise feature/candidate source-range pointer**; **142 have none** in the registered ranges. This is reference bookkeeping, **not semantic review coverage**. [Machine ledger](navigation-gaps.json) records line interval unions/gaps per frozen source file; **semantically reviewed line count is not established**.

## What the numbers mean

- Selected syntax files: 298, 120,858 input source lines. This excludes protected/vendor/nonselected data/docs/assets explicitly; no whole-repository line denominator.
- Feature/candidate source references union: 38,545 lines, overlapping/adjacent intervals deduplicated. **Linked line is not an audited line**. A long function link doesn't mean every branch/caller/line was reviewed. No percentage published.
- 26 directory/unranged pointers preserved with zero line credit; they may be useful navigation, but never converted to entire-file/entire-directory completeness.
- Feature registry has 25 principal contracts/685 source ranges; original 119 claims source-triaged 39/75/5. Those artifacts represent bounded evidence, not exhaustive semantic acceptance.

## Prioritized remaining evidence, not mass fixes

The ledger exposes large unlinked inputs. Some are tests, experimental/support files or sources already mentioned only by file/directory; absence of exact registered range is not proof nobody read them. Next work should add genuinely inspected behavior/caller evidence, not broaden ranges to inflate counts.

| Slice | Current gap | Required bounded evidence |
|---|---|---|
| Overlay bar/tile/archive Slint | [Selected overlay/tile busy/gen/terminal contract added](../features/overlay-tile-state-and-stream-terminals.md); [selected archive confirmation/latch chain added](../features/archive-ui-confirmations-and-latches.md); remaining render/geometry/selection gaps remain | Actual remaining state/translation/callback/render consumers and terminal scheduling preconditions; physical UI acceptance separate |
| WSOLA playback | [Selected algorithm/stream/transports/clock contract added](../features/wsola-streaming-and-playback.md); no all-DSP numerical acceptance | Remaining exceptional branch/quality/performance/allocation/source finite-input and device/cancel listening tests; native audio remains separate |
| Tera engine/transports/textnorm | [Selected graph/text/cancel source contract added](../features/tera-graph-text-and-cancellation.md); runtime model/linguistic acceptance remains absent | Actual ORT graph bytes/inference/header/duration/tag preconditions, cancellation latency/work/memory and native audio; no source-only certification |
| Test modules | Large config/local-AI/runtime/journal/AI tests lack precise range registration | Describe tested invariants/fixtures versus unexecuted Rust tests; do not equate test lines with production review |
| UI math/state/audio controllers | Selected untouched/unlinked semantic chains | Formatting/escape/state lifetimes/device permission/routing callers and explicit known unknowns |
| Other Config keys/native callback indirection | Name candidates are not type-resolved graph | Manual concrete receiver/type/ownership/caller/source, avoid global full-graph claims |
| Hypotheses and independent acceptance | CI C04/C05/C06/C12 and TTS C03/local-AI C10 bounded extra evidence only | Remaining prerequisites proportionately verified; explicit Gemini/Opus route needed for real independent acceptance, no inherited-model substitution |

## Historical portable receipt integrity

[Receipt-integrity report](portable-receipt-integrity.json) checks **14 stored receipts through backend checkpoint**, all exact tested commit objects present; **11 recorded artifact hash/size checks match tested Git object bytes**. Baselines read from those exact commits. Four older schemas omit one boundary flag; omissions retained explicitly, **not acceptance**, and originals not rewritten.

This check is read-only Git object verification, **not rerunning historical test counts**, proving all prose, or independent/native acceptance. Fixtures test hash/size mismatch, legacy absent flags, false acceptance, unavailable Git objects/invalid SHA. Missing Git history in a portable archive is a concrete limitation: run receipt validator against task Git checkout, not an archive without objects.

## Reproduce

```bash
# Reference ledger needs only Python/committed inputs, no parser/native dependency.
python3 -B docs/agent-map/operations/coverage_gaps.py --output docs/agent-map/reconciliation/navigation-gaps.json
python3 -B docs/agent-map/operations/coverage_gaps.py --output docs/agent-map/reconciliation/navigation-gaps.json --validate
# Read-only exact historical artifact check requires Git checkout history.
python3 -B docs/agent-map/operations/verify_portable_receipts.py
```

Read generated artifacts before replacement under file-observation policy. Ledger hashes bind syntax/schema/register/feature inputs; union/gap fixtures reject invalid ranges and semantic credit. No new dependency, production edits, native operations or original status promotions. Full objective remains active/incomplete.
