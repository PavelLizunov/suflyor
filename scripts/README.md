# overlay-mvp scripts/

Workflow tooling for the testing methodology in `../CLAUDE.md`.

## Files

| File | Purpose | Layers covered |
|---|---|---|
| `ci.ps1` | Local CI runner — clippy + cargo test + copy_contract + tsc | 1, 2, 3 |
| `visual_check.ps1` | Launch overlay + screenshot primary display → PNG | 6 |
| `../docs/REVIEW_AGENT_PROMPT.md` | Template for `Agent(general-purpose)` review pass | 4 |

## Quick usage

```powershell
# Before every commit:
npm run ci                                # layers 1-3 (~30 s)
# (then in your Claude session)
# → Agent(subagent_type:"general-purpose", prompt = docs/REVIEW_AGENT_PROMPT.md)
# After build of any release:
npm run tauri build -- --bundles nsis     # layer 2 → build artifact
powershell scripts\visual_check.ps1 -Install
# (then in your Claude session)
# → Read C:\...\target\visual\overlay-{timestamp}.png   # layer 6 eyeballs
```

## Why this exists

After the 2026-05-26 marathon (29 releases in 6 h → user cut 64/68 from
GitHub by hand), we adopted the 6-layer methodology from `vpnctl`. See
`../CLAUDE.md § Methodology — six-layer testing`. Skipping any layer
historically produced a user-facing regression.
