# Spec: Optional Strata Vision encoder

## 1. Intent & Invariants
- Add a quick optional download of the compatible Qwen3.8-Flash-Next encoder.
- Reuse the selected Strata installation and its bundled image helper.
- Use CPU encoding by default; do not reserve extra GPU memory.
- Do not install another language model or another server.
- Downloading never edits Strata run configuration, saved providers or routes.
- Activation is explicit and requires the selected local text route to be Strata.
- Do not replace an unmanaged listener or interrupt unrelated model servers.
- Failed activation restores the previous run configuration and owned server.
- Preserve existing configured encoders and all unrelated configuration fields.
- Keep existing display reports and rc.14 evidence outside this task.

## 2. Interface / Data Contract
```rust
// local_ai Strata extension; blocking operations run outside the UI thread.
pub fn strata_vision_installable(root: &Path) -> bool;
pub fn strata_vision_downloaded(root: &Path) -> bool;
pub fn strata_vision_available(root: &Path) -> bool;
pub fn download_strata_vision(root: &Path, cancel: &AtomicBool, on: &dyn Fn(Progress)) -> Result<()>;
// Activation owns the lifecycle lock, verifies an image before succeeding,
// and restores the previous run configuration on failure.
pub fn enable_strata_vision(root: &Path, cancel: &AtomicBool, on: &dyn Fn(Progress), commit: &dyn Fn() -> Result<()>) -> Result<()>;
```
- Pin the upstream 907,543,008-byte BF16 encoder by immutable revision and SHA-256.
- Require a compatible original Qwen3.8-Flash-Next model and existing helper.
- Never attach this projector to Qwen3.6, Qwen3.5 or a text-only Coder variant.
- Vision Settings exposes Download / Resume, Cancel and Enable for F8 separately.
- A successful enable sets image capability and the explicit saved F8 route.
- Existing downloads survive hiding/reopening the retained Settings window.

## 3. Verification Checklist (Definition of Done)
- [x] Compatible/incompatible configurations and path resolution covered by tests.
- [x] Download preserves run configuration and saved text/Vision/STT routes.
- [x] Pinned integrity verification, cancellation and resumed download checked.
- [x] Failed start restores prior run configuration and keeps saved routes.
- [x] Native selected gate passes on immutable candidate 460ced5.
- [x] Matching EN/RU Vision baseline/after captures inspected through Slint MCP.
- [x] All 13 hotkey dispatches observed; F8 off-route behaves as saved.
- [x] Checkpoint branch pushed and exact remote SHA verified.
- [ ] Real successful Strata image input and saved enabled F8 route.
- [ ] Restart of a previously running real text server on activation failure.

Published evidence: `docs/release-evidence-v0.38.1-rc.15/README.md`. The Windows
worker has 16 GiB RAM and no NVIDIA GPU. The approved GPU peer was offline in
Tailscale and SSH timed out. Successful inference acceptance remains open.
The owner explicitly requested an experimental prerelease with this limitation
disclosed; v0.38.1-rc.15 is published without claiming those unchecked items passed.
Physical macOS is not tested; its existing managed MLX Vision path is unchanged.

Stable v0.38.1: the owner reports rc.15 works and authorized publication. This
is owner-reported acceptance, not an independent inference trace. Full physical
macOS compile-seam and isolated package start now pass; physical macOS UI and
real Strata inference items above remain unverified. See
`docs/release-evidence-v0.38.1/README.md`.
