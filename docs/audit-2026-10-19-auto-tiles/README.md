# Auto-tile stream and retained-window audit

Scope: preserve actual buffered Responses answers instead of dropping their auto tile; prevent delayed native setup from revealing a closed retained window. The owner also reports a large transparent outline. Its exact compositor symptom remains unverified on the single-monitor Windows worker.

Baseline native QA: c39142d8f4e7dbc0a69f6ba5adccdff4b873c9f5, executable SHA-256 0add46ec8d6713fe867a05361e6a9d785017f8d81df1668900d8952e392512fe. Windows software renderer, Light Frost, English,100% scale,460x360 logical tile; synthetic QA transcript and loopback Responses endpoint.

Red: a response.completed event containing non-empty output without preceding text deltas gives reason=stop/deltas=0. The auto placeholder appears and disappears. A response with normal text deltas remains open. Backend regression930673147ef9eb3df5d750d145c7c8c9e6b081ad fails with empty text instead of answer. Backend00c71d6ae3aa0ea5db44de09b9e169f73f2622bd passes terminal-only and non-duplication cases. These are fixtures, not a trace of the owner endpoint.

Native fast-empty baseline removes the placeholder and leaves open-tile count0. Settings native close reads IsWindowVisible=false; Slint MCP still lists the retained handle with its old geometry and therefore does not establish native visibility. Never equate that MCP listing with a ghost. The worker has one1024x768 monitor, so owner multi-monitor compositor residue cannot be accepted as fixed from this run.

Private evidence and scripts: ~/.hygiene/sufloyr/ui-geometry/. No owner transcript, endpoint trace, credentials, or desktop images enter Git.

Pending exact-candidate gate, live before/after buffered/delta/empty/close scenarios, independent UI review and release acceptance.
