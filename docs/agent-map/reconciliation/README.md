# Research reconciliation evidence

## Frozen scope

[Snapshot](snapshot.json) records source commit `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`, tracked non-map file hashes, and hashes of fourteen preexisting Grok-labeled reports. The original Grok model/backend and exact original source revision are not independently authenticated by filenames. The raw reports remain unchanged and untracked; their private-network examples prevent verbatim public publication. [Redacted copies](grok-redacted/) preserve the claims with placeholders; [transformation provenance](grok-redaction-provenance.json) links both original and copied hashes.

[Candidate register](candidates.json) assigns stable IDs to all 119 `Finding / Hypothesis` entries:

| Lane | Candidates | Expected output |
| --- | --- | --- |
| Data, memory, config | 36 | `data.json`, `data.md` |
| Audio, STT, TTS, local AI | 28 | `audio.json`, `audio.md` |
| Host UI, windows, tiles, settings | 33 | `ui.json`, `ui.md` |
| Privacy, installers, CI | 22 | `security-build.json`, `security-build.md` |
| Speech-model history | Separate integration inventory | `speech-models.json`, `speech-models.md` |

Missing lane files are pending work, not accepted evidence. The first four Opus lanes returned null with partial topic-misaligned files; eight exact-claim follow-up lanes returned null without files. Both receipts are retained. Those proposals are not accepted as independent review.

The coordinator has now inspected the exact original text for all 119 candidates: 40 source mechanisms confirmed, 74 hypotheses and 5 rejected. Read [summary](SUMMARY.md), [coordinator evidence](coordinator-checks.json) and [remediation queue](REMEDIATION.md). No native application reproductions were run; full-project completeness remains unaccepted.

## Evidence statuses

- `confirmed`: claimed mechanism is established from source; reproduction execution is recorded separately.
- `hypothesis`: plausible consequence still needs a missing precondition or runtime check.
- `rejected`: current source or caller behavior contradicts the claim.
- `duplicate`: the same issue is already tracked by a referenced candidate.
- `obsolete`: a historical issue is fixed or no longer applies at this snapshot.
- `unresolved`: evidence or analysis is insufficient.

Source inspection is not a native reproduction. Neither an existing test declaration nor a report's suggested test counts as an executed passing test.

## Historical review boundary

The old map used regex extraction and blanket completeness labels. Original bodies are retained in Git, with explicit provenance warnings. The historical final reviewer returned `changes_required` (visible chat tool receipt); its complete original response was not preserved as a standalone accepted evidence file. We do not reconstruct a favorable verdict or perform a new Astra call.

The source/model review now uses only explicit Gemini and Opus routes. See [operations](../operations/README.md) for checkpoint mechanics and known automation limits.
