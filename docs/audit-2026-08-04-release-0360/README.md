# v0.36.0 release UI audit

Conditions for the before/after pair: Settings at 720×600, 100% scale,
Light Frost theme, Updates tab, no update request in progress. The English
baseline is the verified v0.35.3 release screenshot; the v0.36.0 screenshots
were captured from the exact `ui-mcp` build of this release branch.

Checked:

- English: version is `0.36.0`; all labels and controls are visible.
- Russian: version is `0.36.0`; translations fit without clipping.
- The language was restored to English after the audit.
- The installed normal release bar was captured at startup and after five
  seconds. Both captures are 1200×64 at `(360, 24)` with stable geometry and
  no missing glyphs.
- The two-step Quit action stopped the installed host and TTS sidecar in
  1,145 ms. The normal installed binary was then relaunched for owner review.

Only package/version metadata and release documentation changed on this
branch. Shared Settings primitives and layouts were not modified, so the
16-tab geometry pass from the underlying master build remains applicable.
