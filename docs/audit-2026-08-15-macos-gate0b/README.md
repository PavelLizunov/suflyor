# macOS Gate 0B evidence policy

Gate 0B is an isolated local feasibility app for native capture and TCC. It is
not the production macOS port and does not use AI, STT, persistence, updater,
BlackHole, Tailscale, or the existing UAP STT service.

Only sanitized facts belong in Git:

- permission state names and generic error/status codes;
- capture frame counts, image dimensions, and lifecycle results;
- compiler, package, signature, and process-count results.

Do not commit sensitive runtime screenshots, captured audio, device names,
account identifiers, network data, TCC database contents, or unrelated
application/window content. A sanitized visual check of this isolated Slint
window is allowed when required by the project UI policy. The ScreenCaptureKit
probe captures only its own Gate 0B window and discards the image after
recording dimensions.

The owner does not plan to purchase Apple Developer Program enrollment. Local
Gate 0B work therefore uses an ad-hoc package that passes local
`codesign --verify --strict`, and records TCC persistence across rebuilds as a
limitation. Paid signing and notarization are not Gate 0B acceptance
requirements.

Recorded results are in [RESULTS.md](RESULTS.md).
