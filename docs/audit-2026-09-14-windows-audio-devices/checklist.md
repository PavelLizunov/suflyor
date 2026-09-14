# Audio settings acceptance checklist

Baseline: `2ab0dee01dda8632830aee1792543bc0d434237b`.
Tested implementation: `749813a8ed368d2585b7fc86be9e6636d928bce5`.

The implementation and the available Windows checks are recorded below.
Overall acceptance remains incomplete: physical endpoint switching, production
rendering, independent review and the full paired-state matrix are unavailable.
No release, installer or merge is included.

| Requirement | Observed evidence | Result / boundary |
| --- | --- | --- |
| Microphone and system selectors | Real host shows both; fixture selects USB microphone, USB headphones and A50 Stream Out | Passed UI; physical devices unavailable |
| Preserve missing headset | Both synthetic saved names remain selected with unavailable notices | Passed EN/RU |
| Default removes pin | Unit tests assert `None`; actual ComboBox input selects the localized default and removes the unavailable row | Passed tests and EN/RU fixture |
| Refresh and reopen | Final host Refresh and close/reopen repopulate both defaults; fixture refresh retains new choices | Passed available runtime checks; hotplug not exercised |
| Loading, empty, enumeration failure | Both languages inspected; loading/error disable selectors, error leaves Refresh enabled and hides availability notices, empty state can clear old pins | Passed fixture; no real driver failure injected |
| Save failure | Full-config regression test; both live ComboBoxes return to their previous displayed names and show the error | Passed EN/RU fixture; real filesystem failure not injected |
| Next-capture timing | EN/RU notice visible at 720x600; capture engine unchanged | Passed copy/UI; capture timing not physically tested |
| Preserve macOS/TTS/schema | Unchanged capture engine, manifests, lockfiles and config schema; seven macOS source guards passed | No macOS runtime test or TTS playback test |
| Native targeted gate | Exit 0; backend 732 passed / 3 ignored; Slint 283 passed / 3 ignored | Passed exact SHA; 1015 total, 0 failures |
| Focused verification | Host Clippy, six audio tests, 15 focused guards and both MCP builds returned 0 | Passed exact SHA |
| Paired visual evidence | EN real-host before/after at 720x600, scale 1, Glacier, software renderer, same profile and scroll position | Passed EN pair; RU/state-specific baseline pairs unavailable |
| Normal-size layout | 720x600 EN/RU states visually inspected; recording/storage reachable by vertical scroll | Passed affected Audio surface; no shared primitive/layout was changed |
| Minimum-size layout | RU 480x320 has horizontal overflow and clipped content until horizontal scrolling | Not accepted; baseline at this size was not recorded, regression status unknown |
| Global hotkeys | F1 opens/closes Help; F4 opens Palette; F7 opens Archive; distinct Windows-input dispatch logs and window geometry | Safe subset passed on final host; remaining keys listed below |
| Default-output switching during capture | Existing five route-policy tests pass | Not run: no active physical audio endpoint pair |
| Independent/security review | Coordinator differential and security review, no confirmed security regression | Not independent acceptance |

## Unverified hotkeys

`F3`, `F6`, `F8`, `Shift+F8`, `Ctrl+F8`, `F9`, `Shift+F9`,
`Shift+Alt+1`, `Shift+Alt+2`, `Shift+Alt+3` were not exercised. The normal host
loads a real profile with credential presence; no safe isolated production
profile was established. Avoiding AI requests, screen/clipboard capture and
read-aloud side effects takes priority over manufacturing a full smoke pass.
All 13 registration log entries exist, but registration is not dispatch proof.
Vision-route persistence and diagnostics-tab registration were not live-tested.

## Evidence boundaries

The ignored fixture uses the compiled Settings window, synthetic in-memory
config and an injected save result. The MCP driver sends actual ComboBox input;
it does not set the selected value directly. All ten language/state processes
closed normally with one ignored-test invocation passing each. These process
exits are lifecycle evidence; interaction assertions and screenshots provide
the UI evidence. They do not prove WASAPI hotplug, durable filesystem writes or
host global-hotkey dispatch.

The worker's default renderer failed before the patch with a missing OpenGL
entry point. Software-renderer evidence is not production-renderer acceptance.
Only privacy-reviewed Audio screenshots are included. Raw trees, process
receipts and logs remain in ignored task artifacts.
