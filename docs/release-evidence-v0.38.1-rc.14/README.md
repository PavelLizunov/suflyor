# RC.14 verification evidence

Published prerelease: https://github.com/PavelLizunov/suflyor/releases/tag/v0.38.1-rc.14

Installer: https://github.com/PavelLizunov/suflyor/releases/download/v0.38.1-rc.14/suflyor-slint-setup.exe

- Tested candidate:564d9a739709b579875abd757e373eea7383334d.
- Merge/tag:92438f4eb9bc7e0957dc5128706a0df729009de0.
- Identical source tree:682e189e10a5b581f8894b538777fd8ecd2223ad.
- Installer:38333299bytes; SHA-2565d982a36b982ac54d4f8bbc3289a46d7a7ba37d68a09cc8962f09a2b15a6352a. The published asset was downloaded and matched exactly.
- QA executable SHA-256ff1c7d02426149872f6cb29c8beab73a35c361445489baf9c3a39f8df29d47c5.

## Native Windows gate

Selected gate exit0:798backend unit tests,94Slint library tests,153host tests,all selected guards including new provider geometry and retained Settings lifecycle. Stable-only full suite was not run.

GitHub required gate,Windows Rust,translation/classifier validation,secret scan and all3dependency checks passed at publication. macOS CI was still running; physical macOS acceptance was not performed.

## Actual UI consumer

Windows software renderer,Light Frost,100% scale,720x600Settings,460x360tile. Matching before/after captures and MCP trees are retained privately outside Git.

- OpenAI overlap before3pxEN/15pxRU; after8pxgap. Width720/600/480headless geometry passes.
- All16Settings tabs visited before/after in EN/RU. Native IsWindowVisible=false after close; reopen displays on screen. Retained MCP rectangle alone is not visibility evidence.
- Native synthetic Responses completed-only and ordinary delta cases in EN/RU keep the auto tile for10seconds,render answer exactly once,offer follow-up and disappear only after explicit close. Fast-empty case leaves count0.
- Backend regression930673147ef9eb3df5d750d145c7c8c9e6b081ad fails with empty text; fixed selected gate covers terminal-only,streamed,empty and incomplete output.
- All13global shortcut dispatches observed. Registration13,no failures. F8family capture surfaces cancelled with Esc; OpenAI Vision route persisted after Settings close. Actual selected-text audio is unverified: dispatch occurred but clipboard copied0characters. External route completion/audio is not implied by dispatch.
- Install/start/uninstall exit0,version0.38.1-rc.14in executable and registry,13registered hotkeys,0registration failures/panics; config restored byte-identically. Normal non-MCP release left running.

## Limits and cleanup

The owner Gemini wire trace was not captured. Large transparent outline not reproduced on the single-monitor worker; guards address a source-backed delayed-reveal risk,not verified multi-monitor compositor repair. Local-capability caption clipping not reproduced after end-scroll. Early-close-before33ms behavior is source-guarded,not directly timed natively. Previously disclosed full overlay-bar geometry failure remains.

Cleanup preview and apply both exit0. Oldrc.13release/tag and merged task remote branches removed; stable Latest remainsv0.38.0. Five unrelated non-ready PRs(#205,#286,#288,#289,#296),unproven branches and active worker checkout preserved. Original integration checkout HEAD,raw index and13unrelated dirty/untracked file hashes unchanged.

A historical23:59once-scheduled job unexpectedly restarted oldrc.13during QA. Its exact process tree was stopped,the task disabled and XML preserved; source restored to the tested candidate and QA binary hash rechecked. Native RU scenarios were rerun after recovery. No build remains active.
