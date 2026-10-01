# Error records: source-verified correction and remaining gaps

## WSOLA error enum

At reviewed commit `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`, [the declaration](../../../suflyor-wsola/src/error.rs#L5-L18) contains exactly four variants:

| Variant | Payload |
| --- | --- |
| `InvalidRatio` | `String` |
| `InputTooShort` | `provided: usize`, `minimum: usize` |
| `BufferOverflow` | `buffer: &'static str`, `requested: usize`, `available: usize` |
| `InvalidState` | `&'static str` |

[The JSONL entry](errors.jsonl) now matches that declaration. The old parser incorrectly treated a struct-variant field as an enum variant and stopped at a nested closing brace. The earlier prose also invented variants absent from the implementation. Both are corrected; call-site behavior still requires separate checks.

## Historical error-site candidates

[The 461 error-site matches](error_sites.jsonl) were generated from regex patterns for bailout and context operations. They are navigation candidates, not 461 independently audited recovery paths. The patterns miss other error construction/propagation forms and can truncate expressions.

No blanket claim is made that every failure recovers, that SQLite corruption causes a lossless `VACUUM`, or that every visible error redacts secrets. Those consequences require source and caller verification. See the [Grok reconciliation](../reconciliation/candidates.json) for per-candidate status.

## Verification boundary

This correction checked the enum declaration and display match against source. No native Cargo build, unit-test execution, application launch or malformed-input reproduction was performed here. Production lint declarations are policy checks, not evidence that runtime paths cannot panic or abort.
