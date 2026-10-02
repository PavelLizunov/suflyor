# Original local AI C06/C07: bounded download verification and HuggingFace branch evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_local_ai_download_artifacts_hypotheses.py) inspect frozen local AI asset download and installation routines. They do not execute network requests, download model weights, or extract archives. C06 and C07 remain hypotheses.

## C06 — server binary host-pinning vs missing Authenticode/SHA

In `local_ai.rs`, [is_trusted_release_url](<../../../overlay-backend/src/local_ai.rs#L2444-L2453>) enforces an allowlist check on the download URL (restricting to `https://github.com/`, `https://objects.githubusercontent.com/`, and `https://release-assets.githubusercontent.com/`).
However, [download_and_extract](<../../../overlay-backend/src/local_ai.rs#L2450-L2480>) downloads the archive via `curl_resumable` and unpacks it with `extract_zip` without verifying a cryptographic hash or Authenticode signature on `llama-server.exe` / `whisper-server.exe` / DLLs.
In [curl_resumable](<../../../overlay-backend/src/local_ai.rs#L2563-L2620>), the download loop checks `if cur >= expected` and terminates when the file size meets or exceeds the expected length.

## C07 — mutable `/main` HuggingFace branch URLs vs pinned SHA verification

In `local_ai.rs`, auxiliary models are fetched from mutable `/resolve/main/` endpoints:
- [MMPROJ_URL](<../../../overlay-backend/src/local_ai.rs#L81-L86>): `.../resolve/main/mmproj-F16.gguf`
- [WHISPER_URL](<../../../overlay-backend/src/local_ai.rs#L92-L97>): `.../resolve/main/ggml-large-v3-turbo-q8_0.bin`
- [GIGAAM_MODEL_URL](<../../../overlay-backend/src/local_ai.rs#L100-L105>): `.../resolve/main/v3_e2e_ctc.int8.onnx`

While the branch references are mutable, the codebase defines explicit SHA-256 constants (`MMPROJ_SHA256`, `WHISPER_SHA256`, `GIGAAM_SHA256`).
During [install](<../../../overlay-backend/src/local_ai.rs#L363-L600>), each downloaded asset is verified via `verify_sha256(&dest, PINNED_SHA256, ...)` before use.

## Limits

No malicious network response was fed, no corrupted model weights were staged, and no server binary execution was tested. Original statuses in `candidates.json` remain `hypothesis`.
