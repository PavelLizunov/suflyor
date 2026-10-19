# Strata Vision UI audit (in progress)

Baseline: release candidate 564d9a739709b579875abd757e373eea7383334d,
product-equivalent to the current master before this feature. Windows software
renderer, light theme, scale 1, Settings 720x600, Vision tab, top scroll position.
Capture English and Russian before changing the UI, then repeat for the exact
committed candidate. Private native captures and task scripts live outside Git.

Acceptance includes download/resume/cancel and retained-window states, honest
unavailable status without a compatible Strata installation, image verification
before F8 activation, failure rollback, route persistence and hotkey dispatch.
No real Strata inference is claimed from mock fixtures or source inspection.
