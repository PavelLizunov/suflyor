# Windows audio device settings audit

Status: in progress. No implementation or runtime acceptance is claimed yet.

## Identity

- Branch: `codex/windows-audio-devices`
- Baseline commit: `2ab0dee01dda8632830aee1792543bc0d434237b`
- Baseline tree: `d3b0fb8c3e6eedbe688a116f60b53e3c2250a4ed`
- Worker: `windows-worker`
- Candidate: pending

## Baseline job plan

- Scheduled task: `SuflyorAudioDevicesBaseline20260914`
- Script: `C:\suflyor-test-evidence\audio-devices-20260914\baseline.ps1`
- Log / exit / manifest: `baseline.log`, `baseline.exit`, `baseline-manifest.json`
  under the same task evidence directory.
- Checkout: `C:\suflyor-audio-devices`, detached at the exact baseline.
- Build cache: existing `C:\suflyor\slint-experiment\target`, sequential use only.
- No existing task wrapper is replaced.

## Capture conditions

Target paired conditions: Settings 720x600, scale 1.0, same theme, language,
renderer, audio device configuration and scroll position for each before/after
pair. Record actual conditions before accepting images. Screenshots must contain
no secrets, private transcript/session titles, paths, or network endpoints.

Previous worker evidence warned that overriding APPDATA does not isolate
Windows known-folder configuration. Do not assume isolation or modify the live
profile based on an environment override. Hardware availability and production
renderer must be established; software rendering is not production acceptance.

## Check results

Pending. Native tests and live checks run on the worker, never on DSH.
Independent model review is not available through an authorized explicit-Gemini
route in this session; coordinator source review is not independent acceptance.
