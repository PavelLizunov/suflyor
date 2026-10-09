### 1. Native Gate Classification Bypasses
- **Finding / Hypothesis:** `scripts/git-gate-native.ps1` lines 73–81 treat *any* path ending in `.md`/`.html`/`.txt` as docs-only (`$docsOnly` → `$tier = 'docs'`), then lines 147–150 **exit 0 with no Cargo/clippy/tests**. That includes `overlay-backend/knowledge/*.md`, which CI comments in `.github/workflows/ci.yml` (lines 24–32) explicitly call **code inputs that must force full CI**.
- **Rationale:** Local pre-push can skip the real gate while GitHub still (or worse, *doesn’t*) see the same diff. Knowledge/markdown that the backend embeds or loads can change behavior with zero `cargo test`. `.html`/`.txt` are not docs in the CI classifier contract (`*.md` or `docs/` only), so local vs CI skip logic already disagrees.
- **Verification Method:** `git add overlay-backend/knowledge/whatever.md` (or `notes.txt` / `foo.html` only) → `powershell -File scripts/git-gate-native.ps1 push` and confirm `tier=docs` / `OK (docs-only; no Cargo work)`. Compare with `bash .github/scripts/is-docs-only.sh` on the same range (knowledge `.md` must *not* be docs-only).

- **Finding / Hypothesis:** Crate membership is only `overlay-backend/`, `slint-experiment/`, `suflyor-tts/`, `suflyor-teratts/`, `suflyor-wsola/` (lines 19–25, 67–71). On `push`, if `$tier -ne 'docs'` and `$affectedCrates` is empty, the `foreach` at lines 166–205 is a no-op and the script still prints `OK (targeted)` (lines 207–208). High-risk paths **never** select a crate: `.github/workflows/*`, `scripts/git-gate-native.ps1`, `scripts/slint-installer.nsi`, `scripts/ci.ps1`, `scripts/build-slint-release.ps1`, `deny.toml`, `.gitleaks.toml`, root `Cargo.toml`/`Cargo.lock`.
- **Rationale:** You can gut the gate, installer, or CI in one commit and local push-hook still “passes.” A self-modifying bypass is: rewrite `git-gate-native.ps1` to force `$tier = 'docs'` (commit stage only runs fmt on crates, lines 132–140, so the hook change itself is not clippy-tested).
- **Verification Method:** Change only `scripts/slint-installer.nsi` or `.github/workflows/ci.yml` → `powershell -File scripts/git-gate-native.ps1 push`. Expect `crates=` empty and `OK (targeted)` with no `clippy`/`test`. Then change the classifier to always set `$tier = 'docs'` and repeat.

- **Finding / Hypothesis:** `slintUiOnly` (lines 175–177) skips clippy and the full test suite whenever *no* changed path under `slint-experiment/` matches `\.(rs|toml|lock)$` or `/build\.rs$`. `.slint`, `.po`, assets, JSON/XML, committed DLLs, etc. only get `cargo check --bin overlay-host` plus a **hard-coded** `--test` allowlist (lines 181–198). New tests not in that list never run on UI-only diffs. `clippy` is skipped entirely.
- **Rationale:** High-risk UI/behavior can live in `.slint` or non-Rust sidecar files next to a shipped binary. The allowlist is a silent skip-list for any new `[[test]]` / integration test.
- **Verification Method:** Touch only `slint-experiment/ui/foo.slint` (or an asset) → push-gate. Confirm logs show `slint UI compile check` / `slint static guard tests` and **no** `slint-experiment clippy` / `slint-experiment test`. Add `tests/new_guard.rs` and a `.slint` change in the same commit vs `.slint`-only; `.slint`-only will not execute `new_guard`.

- **Finding / Hypothesis:** Push base fallback (lines 42–45): if `origin/master` is missing, `$Base = 'HEAD~1'` and the diff is a **single commit**. Combined with docs-only short-circuit, a two-commit push (`feat: rust` then `docs: readme`) classifies as docs-only.
- **Rationale:** First push of a branch, a `main`-not-`master` remote, or a shallow clone drops earlier high-risk files from `$changed`.
- **Verification Method:** On a repo with no `origin/master`, commit a `.rs` change, then a `.md` change, run `powershell -File scripts/git-gate-native.ps1 push`. Confirm only the last commit’s files are listed and `tier=docs`.

- **Finding / Hypothesis:** `git diff --name-only` without `-z` (line 55). Unusual filenames (quotes, non-ASCII, `\"` quoting from Git) are not normalized beyond `\` → `/`.
- **Rationale:** A crafted path can fail `StartsWith("$crate/")` and also fail the docs regex, yielding empty `$affectedCrates` + targeted OK, or be dropped from classification.
- **Verification Method:** `git add` a file with spaces/quotes under `overlay-backend/src/` and print `$changed` / `crates=` in `classify` stage.

---

### 2. NSIS Installer Security
- **Finding / Hypothesis:** `scripts/slint-installer.nsi` does **not** install a Windows service, and `UninstallString` is quoted (`"$\"$INSTDIR\uninstall.exe$\""`, lines 136–137). Unquoted *service* paths are N/A. Residual issue: `Page directory` (line 32) lets the user set `$INSTDIR` to any writable path; install is `RequestExecutionLevel user` (line 22) into `$LOCALAPPDATA\suflyor-slint` by default (lines 16, 26) with **no** ACL tightening, **no** `SetOverwrite ifnewer` policy beyond default, and **no** Authenticode/`!uninstfinalize` signing.
- **Rationale:** Per-user `LOCALAPPDATA` is always attacker-writable *as that user* (classic DLL plant next to `overlay-host.exe`). A custom `INSTDIR` on a shared/weak ACL directory enables cross-user replace of `overlay-host.exe`, `suflyor-tts.exe`, `suflyor-teratts.exe`, `DirectML.dll` (lines 80–96). Shipping `DirectML.dll` beside the exe is the right *search-order* fix for DML, but it also makes the app-dir the first hijack point for any other implicit DLL loads (MSVC/UCRT/app DLL search).
- **Verification Method:** Install to a world-writable dir (directory page). Procmon the launched `overlay-host.exe` for `NAME NOT FOUND` DLL probes in `$INSTDIR`. Replace `DirectML.dll` with a dummy and relaunch. Confirm no `icacls` / `AccessControl` in the `.nsi`.

- **Finding / Hypothesis:** Process-stop helper is dropped under `$PLUGINSDIR` (lines 47–50) then invoked as  
  `nsExec::ExecToStack 'powershell.exe ... -File "$PLUGINSDIR\stop-installed-suflyor.ps1" -InstallDir "$INSTDIR"'`  
  (lines 51, 57) with **`-ExecutionPolicy Bypass`**. `$INSTDIR` is interpolated into a single command string; the directory page can contain `"` and other CreateProcess-breaking characters. `%TEMP%`/`$PLUGINSDIR` is a user-writable temp tree (NSIS random subdir, but still a temp-file pattern).
- **Rationale:** Quote break in `$INSTDIR` is argument injection into the `powershell.exe` command line. Even if `-File` treats trailing tokens as *script* args, a hostile `-InstallDir` can break `stop-installed-suflyor.ps1` (not in this dump) or confuse `nsExec` parsing. Temp extract + Bypass is expected for NSIS, but there is no `GetTempFileName`-style exclusive create and no signature check on the extracted `.ps1`.
- **Verification Method:** Run `makensis` and install with INSTDIR=`C:\Users\Public\x" -WhatIf` (and a path with `&`). Inspect nsExec / PowerShell command line. Confirm `$PLUGINSDIR\stop-installed-suflyor.ps1` exists only under `%TEMP%\ns*.tmp`.

- **Finding / Hypothesis:** Uninstall `RMDir /r` of `$APPDATA\suflyor`, `$APPDATA\overlay-mvp`, `$PROFILE\suflyor-local-ai` (lines 167–169) with no junction/reparse-point guard.
- **Rationale:** Same-user junction at those paths can redirect recursive delete (NSIS `RMDir /r` follows links). Opt-in MessageBox does not mitigate TOCTOU.
- **Verification Method:** Before uninstall, `mklink /J %APPDATA%\suflyor C:\Users\<you>\Desktop\canary` and choose “Yes” on the data-delete prompt.

---

### 3. CI/CD Security & Flakiness
- **Finding / Hypothesis:** `.github/workflows/ci.yml` uses **unpinned** actions: `actions/checkout@v4` (lines 54, 80, 123, 176), `dtolnay/rust-toolchain@stable` (lines 83–86, 126–129) — third-party **and** floating toolchain tag — and `Swatinem/rust-cache@v2` (lines 91–95, 131–135). There is **no** top-level `permissions:` block (workflow defaults may still be `write` on older repos).
- **Rationale:** A moved tag on `dtolnay/rust-toolchain` or `Swatinem/rust-cache` runs with `GITHUB_TOKEN` in a required Windows job. Cache restore without `save-if: github.ref == 'refs/heads/master'` is a known PR cache-poisoning pattern (dependency objects reused on `master`).
- **Verification Method:** `gh api repos/:owner/:repo/actions/permissions/workflow`; confirm absence of `permissions: contents: read`. Replace action tags with SHAs in a fork PR and diff lock/cache keys. Check cache entries created from `pull_request` vs `master`.

- **Finding / Hypothesis:** PR title/body/branch are **not** interpolated into shell (classifier uses `env: BASE_SHA`/`HEAD_SHA`, lines 58–66) — no classic `github.event.pull_request.title` injection. Real bypass is **workflow self-rewrite + non-blocking validators**: `gate` `needs: [changes, rust]` only (lines 154–155); `macos` and `validate` are **not** required. `gate` treats `rust != true` as success (lines 168–173). A PR that sets `.github/scripts/is-docs-only.sh` to always print `true` skips the Windows rust job; if branch protection requires only `gate`, clippy/tests never run. Same PR can edit `test-docs-only-detection.sh` so `validate` also goes green.
- **Rationale:** CI comment (lines 24–32) claims fail-closed classification and that workflow YAML is a full-CI path, but the **required** job trusts classifier output from the **PR tree**. Local native gate is even skip-happier (Finding 1), so `--no-verify` + docs-only classifier poke lands on `master` with a green `gate`.
- **Verification Method:** PR that (1) changes `is-docs-only.sh` → `echo true`, (2) introduces a `clippy -D warnings` failure in `overlay-backend`. Confirm `rust` skipped and `gate` success. Repeat with `validate` still failing vs also patching `test-docs-only-detection.sh`.

- **Finding / Hypothesis:** Shipped `suflyor-teratts` (and `suflyor-wsola`) are in the native `$crateOrder` but **absent** from the `rust` job (ci.yml lines 97–117 only run overlay-backend, slint-experiment, suflyor-tts). Almost no `cargo --locked` (only slint `ui-mcp` check, line 110). `dtolnay/rust-toolchain@stable` + unlocked `cargo test`/`clippy` is flaky and not reproducible. `concurrency.cancel-in-progress: true` (lines 43–46) can hide flakes by canceling the run that would have failed.
- **Rationale:** Installer ships `suflyor-teratts.exe` (`slint-installer.nsi` lines 88–91; `build-slint-release.ps1` lines 62–80) with **zero** GitHub clippy/test. Lockfile drift / crates.io yank surfaces only on developer machines or later.
- **Verification Method:** PR touching only `suflyor-teratts/**` — `rust` job green without any `manifest-path suflyor-teratts`. Delete a lockfile entry and watch CI still resolve. Pin `1.xx.0` vs `@stable` and compare.

- **Finding / Hypothesis:** Task context says CI “enforces … security scans on PRs.” `ci.yml` has **no** gitleaks/cargo-deny/CodeQL job (only a comment that `security.yml` shares the docs classifier). `macos` failure cannot fail `gate`.
- **Rationale:** Security scanning is not part of the required `gate` context in this workflow; advisory jobs are merge-optional.
- **Verification Method:** Branch protection list vs job names `gate` / `validate` / `macos`. `rg` the workflow for `deny`, `gitleaks`, `codeql`.

---

### 4. Version Synchronization
- **Finding / Hypothesis:** `scripts/slint-installer.nsi` line 13 hardcodes `!define PRODUCT_VERSION "0.38.1-rc.3"` (also `Name`/`DisplayVersion`, lines 23, 135). `scripts/build-slint-release.ps1` invokes `makensis /V2 $nsi` (lines 148–154) with **no** `/DPRODUCT_VERSION=...` and no read of `Cargo.toml` / `Info.plist`. Native `version_guard` runs only inside `slint-experiment` tests (gate lines 181–198, and only on that crate’s targeted/full path). Changing the `.nsi` does not affect `$affectedCrates` (Finding 1), so **version_guard does not run**.
- **Rationale:** Release binary, macOS plist, crate semver, and Add/Remove Programs can diverge; users get an installer whose `DisplayVersion` does not match `overlay-host`’s embedded version. RC tags (`0.38.1-rc.3`) are especially easy to forget in one of three files.
- **Verification Method:** Diff `PRODUCT_VERSION` against `slint-experiment/Cargo.toml` `version`, `overlay-backend/Cargo.toml` if published, and `Info.plist` `CFBundleShortVersionString`. Bump only the `.nsi` and run `git-gate-native.ps1 push` + CI: expect no `version_guard` failure. `rg "0\.38\.1" -g "Cargo.toml" -g "*.nsi" -g "Info.plist"`.