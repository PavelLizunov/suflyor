# GitHub Automation & CI Guide (`.github/`)

This directory contains Suflyor's GitHub Actions workflows, path-classification scripts, regression test suites, and Dependabot configuration.

---

## 1. Workflow Inventory & Ownership

| Workflow / Config | Trigger | Target Runner | Description / Key Steps |
|---|---|---|---|
| **`ci.yml`** (`CI`) | `push`, `pull_request` (`master`) | `ubuntu-latest`, `windows-latest`, `macos-26` | Main build & test gate. Classifies diff using `is-docs-only.sh`. Runs `cargo fmt`, `clippy -D warnings`, `cargo test`, and `ui-mcp` QA check on Windows. Runs advisory macOS build seam. Required status check: `gate` (`if: always()`). |
| **`package-windows.yml`** (`Package Windows`) | `workflow_dispatch` | `windows-latest` | Manual build workflow for Windows setup executable. Installs NSIS, runs `build-slint-release.ps1 -Installer`, writes `SHA256SUMS.txt`, and uploads artifact. |
| **`security.yml`** (`security`) | `push`, `pull_request` (`master`), schedule (`0 6 * * 1`) | `ubuntu-latest` | Supply-chain & secret scanning. Runs `gitleaks` and `cargo-deny` matrix for `overlay-backend`, `slint-experiment`, and `suflyor-tts`. Skips `cargo-deny` on docs-only PRs. |
| **`docs-after-release.yml`** (`Documentation after release`) | `release` (`published`), schedule (`23 6 * * 1`), `workflow_dispatch` | `windows-latest` | Automates README synchronization after non-prerelease releases via `sync-release-docs.ps1`. Opens PR or issue for post-release review. |
| **`dependabot.yml`** | Weekly schedule | N/A | Automated Cargo dependency updates for `/overlay-backend`, `/slint-experiment`, and `/suflyor-tts`. Groups `minor` and `patch` updates per crate. GitHub Actions ecosystem omitted by design. |

---

## 2. Least Privilege & Permissions Model

- **`package-windows.yml`**: Explicitly restricted to `permissions: { contents: read }`.
- **`docs-after-release.yml`**: Uses `permissions: { contents: write, issues: write, pull-requests: write }` to allow branch creation, commits, PR generation, and issue creation.
- **`ci.yml` & `security.yml`**: Rely on standard repository default read permissions for checkout and checks.
- **Separation of Concerns**: Security scans (`security.yml`) run independently of build gates (`ci.yml`) to prevent advisory database fetch latency from delaying fast CI feedback.

---

## 3. Release Boundaries & Fast Paths

- **Docs-Only Fast Path Classifier** (`.github/scripts/is-docs-only.sh`):
  - Single source of truth for path classification used by `ci.yml` and `security.yml`.
  - Classifies a diff as docs-only (`true`) if **every** modified file ends in `.md` or resides under `docs/`.
  - **Exception**: `overlay-backend/knowledge/*.md` files are compiled into the binary via `include_str!` and always force full CI (`false`).
  - **Fail-closed**: Any classification error (e.g. invalid git refs) exits non-zero and fails the gate.
  - **Conservative renames**: `--no-renames` flag ensures renaming a code file to `.md` triggers full CI.
- **Release Automation Boundaries**:
  - `gate` job in `ci.yml` serves as the mandatory PR merge block for `master`.
  - Packaging (`package-windows.yml`) is strictly manual (`workflow_dispatch`).
  - Documentation sync (`docs-after-release.yml`) runs only for published, non-prerelease releases.

---

## 4. Local Validation

Agents and developers must validate CI automation logic locally before committing changes to `.github/`:

```bash
# Run classifier end-to-end regression tests (throwaway git repository)
bash .github/scripts/test-docs-only-detection.sh

# Run gate and cargo-deny shell logic tests
bash .github/scripts/test-gate-logic.sh

# Test classification directly against local git commits
bash .github/scripts/is-docs-only.sh BASE_SHA HEAD_SHA
```

Local git hooks and `scripts/git-gate-native.ps1` provide a complementary native classifier. They are not byte-for-byte identical to the GitHub docs-only classifier, so update and test both when path policy changes.

---

## 5. Agent Constraints & Editing Rules

1. **No Unvalidated Workflow Edits**: Do not modify workflow files or shell scripts without executing local validation scripts (`test-docs-only-detection.sh` and `test-gate-logic.sh`).
2. **Preserve Fail-Closed Invariants**: Required jobs (e.g., `gate` in `ci.yml` and `cargo-deny` validation in `security.yml`) must maintain `if: always()` with explicit checking of classifier outputs.
3. **Single Source of Truth**: Keep path classification logic within `is-docs-only.sh`. Do not duplicate path regexes inside YAML files.
4. **Least Privilege**: Maintain minimum required `permissions:` declarations on all workflows.
