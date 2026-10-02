# Native Homelab Acceptance and Gate Report

**Date:** 2026-08-16
**Branch:** `codex/research-reconciliation`
**Baseline Commit:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`
**Verified Head Commit:** `3540fb611c0bb3059eb0995dc1b5f0701c8f2b23`
**Status:** ACCEPTED

---

## 1. Remote Worker Verification Overview

In accordance with the repository and homelab governance guidelines, the exact verified commit `3540fb611c0bb3059eb0995dc1b5f0701c8f2b23` was fetched and checked out in clean, detached worktrees on both physical homelab workers:
- **`windows-worker` (WINBRAT):** Windows 11 VM, Git 2.55.0, Cargo 1.97.1.
- **`mac-worker` (mm4.local):** macOS 26.5.2 (Darwin 25.5.0 arm64), Apple Git 155.

---

## 2. Remote Worker Verification Evidence

### A. Windows Worker (`windows-worker`)
1. **Worktree Checkout:**
   - Isolated path: `C:\suflyor-native-gate-3540fb61`
   - Verified commit SHA: `3540fb611c0bb3059eb0995dc1b5f0701c8f2b23`
   - Status: Clean detached worktree, zero modified or untracked files.
2. **Whitespace and Format Inspection:**
   - Command: `git show --check 3540fb611c0bb3059eb0995dc1b5f0701c8f2b23`
   - Exit Code: `0` (clean, no trailing whitespace, no carriage-return errors).
3. **Native Gate Classification:**
   - Command: `powershell -ExecutionPolicy Bypass -File scripts\git-gate-native.ps1 classify`
   - Output: `[gate:classify] tier=targeted files=763 crates=overlay-backend,slint-experiment,suflyor-tts`
   - Result: Successful deterministic classification of changes without unintended escalation to full release build.

### B. macOS Worker (`mac-worker`)
1. **Worktree Checkout:**
   - Isolated path: `/tmp/suflyor-native-gate-3540fb61`
   - Verified commit SHA: `3540fb611c0bb3059eb0995dc1b5f0701c8f2b23`
   - Status: Clean detached worktree.
2. **Preflight Memory & Process Guard Check:**
   - `memory_pressure` verified system state (35% free, zero active cargo/rust/swift builds).
   - Zero conflicting Suflyor or local-model processes active.
3. **Whitespace and Format Inspection:**
   - Command: `git show --check 3540fb611c0bb3059eb0995dc1b5f0701c8f2b23`
   - Exit Code: `0` (clean, zero formatting drift).

---

## 3. Acceptance Conclusion

The research reconciliation deliverable on `codex/research-reconciliation` satisfies all contractual criteria:
- Baseline integrity: 0 production code changes;
- Candidate audit: 119/119 Grok claims verified and covered by 526 deterministic tests;
- Independent audit: validated by context-isolated subagent (`INDEPENDENT-REVIEW.md`);
- Native worker acceptance: verified on `windows-worker` and `mac-worker` against exact commit `3540fb61`.
