# O7: Distribution, Packaging, Model Manifests & Gate Integrity (Reconciled)

## 1. Release Gating Architecture (`scripts/git-gate-native.ps1`)
- **Default Tier is Targeted**: Diff classification by default executes **Targeted** checks (`cargo fmt`, `cargo clippy`, and unit tests on the single affected crate).
- **Explicit Full Promotion**: The gate script promotes to **Full** (running across all five crates + NSIS packaging verification) ONLY when explicitly invoked with the `-Full` flag or during official release automation (`scripts/ci.ps1`). Multi-crate changes or high-risk paths require manual invocation of `-Full` rather than an automatic unprompted promotion.
- **Docs-Only Fast Path**: Diffs strictly affecting Markdown, HTML, and documentation text bypass Cargo builds entirely via `git diff --cached --check`.

## 2. NSIS Installer & Binary Topology (`scripts/slint-installer.nsi`)
- **Version Parity Invariant**: `slint-experiment/Cargo.toml` and `scripts/slint-installer.nsi` (`PRODUCT_VERSION`) must match identically.
- **Static ONNX Linkage (No `ort.dll`)**:
  - As verified directly in `scripts/slint-installer.nsi:83–88`: ONNX Runtime is statically linked into the binaries. **No dynamic `ort.dll` is bundled or shipped.**
  - DirectML (`directml.dll`) and C++ runtimes are bundled as required by Windows GPU acceleration.
- **Nemotron CLI Packaging**:
  - The installer explicitly bundles the Nemotron CLI binary (`suflyor-nemotron-cli.exe`), accompanying support DLLs, and associated notices as part of the native Windows audio stack (`scripts/slint-installer.nsi:68–81`).
- **Process Isolation**:
  - `overlay-host.exe` (main Slint application + in-process GigaAM STT)
  - `suflyor-tts.exe` (Piper TTS + sherpa-onnx diarization sidecar)
  - `suflyor-teratts.exe` (TeraTTSv2 experimental sidecar)

## 3. Model Weight Manifests & On-Demand Lifecycle
- **Zero Heavy Bundling**: The installer setup executable never bundles heavy model weights:
  - **TeraTTSv2 (~370 MB)**: Pinned by SHA-256 in `suflyor-teratts/manifest/teratts-v2.json`. Downloaded on demand via Settings -> AI.
  - **Piper Voices**: Downloaded into `%APPDATA%/suflyor/voices/` on demand.
  - **Tesseract OCR**: Language packs (`rus.traineddata`, `eng.traineddata`) are downloaded post-install.
- **Licensing Boundaries**:
  - `suflyor-teratts/NOTICE.md` maintains the upstream licensing compliance gate because upstream TeraTTS historically provides no standard LICENSE file.
  - Process isolation ensures that sidecar runtime dependencies do not contaminate the licensing of the core overlay crates.
