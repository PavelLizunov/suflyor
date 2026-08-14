# v0.37.0-rc.2: intelligible Tera speech and current Codex login

Status: implementation branch `codex/teratts-codex-fixes-rc2`, based on
`v0.37.0-rc.1` (`de277c4`). Version is `0.37.0-rc.2` in both guarded version
locations.

## Outcome

This RC closes two regressions found during owner testing:

1. TeraTTSv2 produced burbling audio because a `[1, 144, L]` sampler tensor
   was sliced as if frames, rather than channels, were contiguous. Vocoder
   windows now preserve the upstream channel-first layout. End-of-stream
   playback drains instead of continually refilling silence, and the sidecar
   exits when its host pipe closes.
2. The current official Codex app-server rejected suflyor's filesystem-deny
   override because the `:root` map key was quoted using syntax no longer
   accepted by the local Codex version. The override now uses the current
   official configuration form while retaining the same no-filesystem,
   no-network policy. Child stderr is reduced to bounded diagnostic classes;
   raw lines are never surfaced or logged.

No private ChatGPT endpoint, token-file import, Hermes credential, broader
permission, or bundled model weight is introduced.

## Evidence

- Tera unit suite: 65 tests green, including channel-first window and EOS
  regressions.
- Real owner-PC synthesis: `READY`, `STARTED`, `DONE`, natural process exit.
  A loopback recording of the fixed phrase was transcribed locally by GigaAM
  exactly as spoken.
- Current official local Codex app-server: account state signed in, Pro plan,
  seven safe account models and rate limits returned after the compatibility
  fix. No credential or config file was read or printed.
- Full `scripts/ci.ps1` is green (single-job rerun after a first owner-PC run
  exhausted RAM while the live 26B server was also active). Installer smoke,
  Winbrat checks, PR and prerelease remain release gates.
- Live Settings verified signed-in Pro/model/rate-limit loading, then a real
  Disconnect cleared the account surface. Connect reached the official
  AwaitingUser device flow; final reauthorization remains an explicit manual
  retest because both later one-time flows expired before browser confirmation.

## Explicitly unchanged

- Tera remains experimental and Piper remains the default fallback.
- Codex still owns ChatGPT login and credential storage.
- Codex inference remains inside the existing fail-closed text-only profile.
- Old release pages and tags are not deleted.
