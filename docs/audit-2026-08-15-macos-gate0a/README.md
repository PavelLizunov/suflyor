# macOS Gate 0A visual evidence

This is a new disposable surface, so no matching pre-change application window
exists. The first successfully packaged build is the visual baseline for later
iterations. Every replacement screenshot must use the same Mac, display scale,
theme, synthetic text, and window state.

Evidence must contain only the synthetic Gate 0A UI. Do not capture account
names, network addresses, other application content, meeting content, or live
configuration.

Recorded results are in [RESULTS.md](RESULTS.md). The evidence set intentionally
keeps raw screenshots and automation logs outside Git because they are useful
only for this machine-local feasibility run.

Still pending:

- a second physical display, which was unavailable;
- Apple Development signing, which is blocked on certificate state in Apple's
  backend (the ad-hoc package is not an acceptance substitute);
- click-through from an inactive foreign app: the first click currently
  activates the Slint/Winit app and the control acts on the next click;
- a production decision for external-capture exclusion, which this prototype
  explicitly reports as unsupported.
