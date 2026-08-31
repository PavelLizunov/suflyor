# `scripts/` Agent Guide & Script Maintenance

This guide documents the maintenance, risk classification, inspection vs. execution policies, remote worker model integrity, and verification protocols for all scripts in the `scripts/` directory.

## 1. Overview & Architecture Policy

The Suflyor project (Overlay MVP) uses PowerShell, Bash, Python, and NSIS scripts for quality gating, build automation, release packaging, repository hygiene, local AI environment setup, and visual UI QA automation.

Key architectural rules when working with scripts in this directory:
- **No hardcoded secrets or environment endpoints:** Never embed API keys, user secrets, homelab IP addresses, Tailscale node names, or private credentials in script sources, defaults, or output logs.
- **Exact-SHA remote-worker & release safety model:** Maintain deterministic, cryptographically verifiable remote operations (e.g. Winbrat remote worker builds, git ancestor SHAs via `git merge-base --is-ancestor`, exact HEAD commit matching for PR merges/branch deletion, and SHA-256 / byte-size integrity validation for DirectML DLLs and downloaded AI models).
- **Inspection vs. Execution distinction:** Clearly separate non-destructive inspection, classification, and dry-run modes (`classify`, `-ListOnly`, `-CheckOnly`, preview mode without `-Apply`, syntax parsing) from mutating execution modes (`push`, `-Apply`, `-Installer`, process termination, file system modifications).

---

## 2. Script Taxonomy & Risk Classification

All scripts in `scripts/` are classified below by domain, inspection/execution modes, and operational risk tier:

| Script | Purpose | Inspection Mode | Execution Mode | Risk Tier |
|---|---|---|---|---|
| `git-gate-native.ps1` | Native selective gate (docs/targeted; explicit stable full) | `classify`, `-ListOnly` stage modes | `commit`, `push`, `manual` stage modes | Low (Inspection) / Medium (Gate Execution) |
| `ci.ps1` | Stable-release full CI across all 5 crates | None (explicit stable publication only) | Full execution (`powershell scripts/ci.ps1`) | Medium-High (Heavy Cargo build, RAM/Disk impact) |
| `git-gate-macos.sh` | macOS arm64 compile-seam gate | None | Bash script execution | Medium (Cargo/Swift compile & test) |
| `build-slint-release.ps1` | Release build for host + sidecars + DirectML DLL + NSIS | Standard build (no `-Installer`) | `-Installer` flag (runs `makensis.exe`) | High (Release compilation & installer generation) |
| `slint-installer.nsi` | NSIS installer definition script | `makensis /V2` dry compile | Execution of resulting installer EXE | High (Modifies `%LOCALAPPDATA%` & Windows Registry) |
| `post-release-cleanup.ps1` | Post-release PR, branch, worktree & cache hygiene | Default preview mode (no `-Apply`) | `-Apply` flag | **HIGH / CRITICAL** (Mutates GitHub repo, closes PRs, deletes branches/tags/worktrees) |
| `stop-installed-suflyor.ps1` | Process termination guard for Suflyor binaries | `-CheckOnly` flag | Default mode (terminates `overlay-host`, `suflyor-tts`, `suflyor-teratts`) | High (Force-kills running processes) |
| `sync-release-docs.ps1` | Syncs `README.md` latest release marker block | Pattern check | Mandatory `-Tag` and `-ReleaseUrl` execution | Low-Medium (Modifies `README.md`) |
| `setup-local-ai.ps1` | Downloads llama/whisper/gigaam binaries + models & launches servers | `-NoLaunch`, `-Skip*` flags | Downloads model weights & launches local background processes | Medium (Downloads multi-GB network assets, launches local servers) |
| `qwen-parallel-audit.ps1` | Parallel read-only static audit runner via Qwen LLM | Read-only static review | Background process execution | Low (Read-only repository audit) |
| `macos-mcp-interactive-qa.py` | Python harness for Slint MCP visual/interactive QA | `list_windows`, property inspection | `click_element`, `take_screenshot` | Low (QA inspection & test driver) |
| Window UI Helpers (`probe_bar.ps1`, `get_hwnd_rect.ps1`, `screenshot_region.ps1`, etc.) | Win32 window geometry probes & synthetic input helpers | Read/probe scripts (`get_hwnd_rect.ps1`, `list_overlay_windows.ps1`) | Input simulation scripts (`sim_click.ps1`, `set_opacity.ps1`) | Low (Window inspection) / Medium (Synthetic input) |

---

## 3. Detailed Guide to Critical Maintenance Scripts

### A. Quality Gates & Verification Pipeline

#### `scripts/git-gate-native.ps1`
- **Purpose:** Agent-agnostic selective gate invoked by `.githooks/pre-commit`, `.githooks/pre-push`, and manual workflows.
- **Classification Rules:**
  - `docs`: Non-executable text/documentation changes (`.md`, `.html`, `.txt`). Runs git diff checks only; no Cargo compilation.
  - `targeted`: Every normal change and every prerelease. Runs checks only for each affected crate/surface; multi-crate or high-risk paths do not auto-escalate.
  - `full`: Selected only by explicit `-Full` while publishing an owner-authorized stable release. Delegates to `scripts/ci.ps1`.
- **Inspection vs Execution:**
  - `powershell scripts/git-gate-native.ps1 manual -ListOnly` or `classify`: Inspects changed files and outputs the selected tier without triggering syntax checks or Cargo builds.
  - `commit` / `push` / `manual`: Runs `git diff --check`, validates changed PowerShell syntax, then executes the selected formatting/test gate.
- **Maintenance Guidelines:**
  - Enforces `$env:CARGO_INCREMENTAL = '0'` and `$env:CARGO_BUILD_JOBS = '2'` to prevent disk bloat and OOM conditions during gate execution.

#### `scripts/ci.ps1`
- **Purpose:** Stable-publication full CI covering all five standalone crates (`slint-experiment`, `overlay-backend`, `suflyor-wsola`, `suflyor-tts`, `suflyor-teratts`). Do not run it for normal development or prereleases.
- **Invariants:**
  - Enforces `$env:CARGO_INCREMENTAL = "0"` and limits parallel rustc jobs (`$env:CARGO_BUILD_JOBS = "2"`).
  - Automatically stages matching `DirectML.dll` into both `slint-experiment/target/debug/deps/` and `overlay-backend/target/debug/deps/` so test executables do not crash on system DirectML symbol mismatches.
  - Verifies the QA-only `ui-mcp` feature build via `cargo check --locked --bin overlay-host --features ui-mcp`.

#### `scripts/git-gate-macos.sh`
- **Purpose:** Native Apple Silicon (arm64) gate verifying macOS compile-seams for backend, Slint UI binaries, Swift MLX sidecar (`suflyor-mlx`), TTS, WSOLA, and TeraTTS sidecars.

---

### B. Build, Packaging & Installer Pipeline

#### `scripts/build-slint-release.ps1`
- **Purpose:** Compiles release binaries for `overlay-host.exe`, `suflyor-tts.exe` (Piper sidecar), and `suflyor-teratts.exe` (TeraTTS sidecar), extracts DirectML redistributable DLL from `ort` build output, and packages the NSIS installer.
- **Invariants:**
  - Sidecar processes (`suflyor-tts.exe` and `suflyor-teratts.exe`) must be compiled into the shared target directory (`$env:CARGO_TARGET_DIR = Join-Path $crate "target"`) so binaries land beside `overlay-host.exe` and reuse cached native ONNX/sherpa libraries.
  - DirectML DLL resolution scans `ort-sys-*` build output for the non-zero redistributable `DirectML.dll` and materializes it at `slint-experiment/target/release/DirectML.dll`.
  - When `-Installer` is set, `makensis.exe` is located across standard install paths (Scoop, Winget, Tauri cache) and launched via `Start-Process` to avoid PowerShell argument-binding errors.

#### `scripts/slint-installer.nsi`
- **Purpose:** NSIS script defining product installation to `%LOCALAPPDATA%\suflyor-slint\`.
- **Invariants:**
  - `PRODUCT_VERSION` must stay synchronized with `slint-experiment/Cargo.toml`.
  - Installs `overlay-host.exe`, `suflyor-tts.exe`, `suflyor-teratts.exe`, and `DirectML.dll`. AI model weights (TeraTTS, GigaAM, Gemma) are downloaded on demand by the app or setup scripts and MUST NOT be bundled into NSIS installers.

---

### C. Release Lifecycle & Repository Cleanup

#### `scripts/post-release-cleanup.ps1`
- **Purpose:** Post-release repository, PR, branch, worktree, and target disk hygiene script.
- **Safety & Risk Controls:**
  - **DEFAULT MODE IS PREVIEW:** Running `powershell scripts/post-release-cleanup.ps1` prints planned actions without mutating state. The `-Apply` flag is required to execute mutations.
  - **Exact-SHA Remote Worker & Ancestry Verification:**
    - Uses `Git-IsAncestor` (`merge-base --is-ancestor <SHA> origin/master`) before considering a commit or branch merged.
    - Closes redundant PRs only when exact head SHA is in `master`.
    - Merges green PRs only with `--match-head-commit` matching exact SHA.
    - Deletes remote/local branches only when exact SHA ancestry is proven in `master` or recorded on a merged PR (`$mergedExact`).
    - Removes worktrees only when clean (`git status --porcelain` is empty) AND head SHA is an ancestor of `master`.
    - Aborts local cleanup if `cargo` or `rustc` processes are running.
    - Never deletes stable or draft releases; preserves the newest `$KeepPrerelease` prerelease tag.

#### `scripts/sync-release-docs.ps1`
- **Purpose:** Synchronizes `README.md` with the published release tag and URL inside `<!-- latest-release:start -->` markers.

#### `scripts/stop-installed-suflyor.ps1`
- **Purpose:** Stops running Suflyor processes (`overlay-host`, `suflyor-tts`, `suflyor-teratts`) matching a specific installation directory.
- **Safety:** `-CheckOnly` exits with code 10 if matching processes are active, enabling non-destructive inspection before installation or cleanup.

---

### D. Environment, AI Setup & Audit Tools

#### `scripts/setup-local-ai.ps1`
- **Purpose:** Downloads and configures local llama.cpp, whisper.cpp, and GigaAM-v3 models with HuggingFace URL resolution and byte-length validation.
- **Safety:** Downloads models to `%USERPROFILE%\suflyor-local-ai` without hardcoding host IP addresses or local user secrets.

#### `scripts/qwen-parallel-audit.ps1`
- **Purpose:** Runs read-only static analysis across 11 codebase domains using Qwen.
- **Safety:** Invokes Qwen with `--safe-mode`, `--approval-mode plan`, and explicit read-only constraints. Does not execute builds, tests, or network calls.

---

## 4. Maintenance & Contribution Protocol

When modifying existing scripts or adding new ones to `scripts/`:
1. **Avoid homelab secrets and private endpoints:** Keep network parameters standard (`http://127.0.0.1:8080/v1`) or configurable via CLI flags/environment variables.
2. **Preserve exact-SHA safety checks:** Never replace `git merge-base --is-ancestor` or exact SHA comparisons with loose string or branch-name matches.
3. **Distinguish inspection from execution:** Provide dry-run/preview modes (`-ListOnly`, `-CheckOnly`, preview default) for any script that mutates git, GitHub, process, or disk state.
4. **Validate PowerShell syntax:** Ensure all `.ps1` files pass `[System.Management.Automation.Language.Parser]::ParseFile`.
5. **Verify documentation:** Run `git diff --check -- scripts/AGENTS.md` after editing this file.
