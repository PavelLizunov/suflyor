# Stable 0.38.1 audit (completed)

Baseline is rc.15 source at master 698b2cac603c9820b40ae22f52e5ef9b4b47d4b1.
Native Windows software renderer, scale 1. Bar 1280x64 and compact 680x64,
EN/RU. Capture baseline before any geometry edit and compare the same states.
The prior full-suite footer failure is evidence to replay, not a passed check.
Screenshots/logs stay in private task evidence outside Git.

Final candidate77708af4caf3ba7d4413097c4221cf5075de4557; tree30515c632df4322628107358f488b15cb6cc9b5e. Full native Windows/macOS and GitHub gates pass. Matching EN/RU idle full-bar captures, compact bar and Vision/Settings reopen inspected. Memory-visible permutations pass headless; native Windows telemetry is empty. All13 shortcuts dispatched; audible audio and real Strata inference remain unverified. Release/installer details in docs/release-evidence-v0.38.1/README.md.
