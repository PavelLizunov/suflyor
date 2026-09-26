# Pinned Nemotron V3 Windows runtime (RC4)

These Windows x64 CPU CLI files are copied from a native Windows build of NVIDIA
[NeMo-Speech.cpp](https://github.com/NVIDIA/NeMo-Speech.cpp) at source commit
`97a15afa5caa9bce5baaa86c1184103877af4101`. Build with upstream
`scripts/windows/build.ps1 -Backend cpu -AsrOnly -Jobs 4`, then verify every
file against `scripts/build-slint-release.ps1` SHA-256 pins. The binary was
exercised against a local 90-second WAV on `windows-worker`; it emitted valid
RTTM and requires adjacent ggml and NeMo DLLs.

The GGUF model **is not included**: Suflyor downloads the official
`nvidia/Nemotron-3-Diarization` Q8 artifact on demand, verifies SHA-256 and
uses a separate on-disk sentinel. NeMo runtime code is Apache-2.0; this
folder includes its LICENSE and NOTICE, THIRD_PARTY_NOTICES.md, the ggml MIT
license, and the separate OpenMDW-1.1 model license for the later model pull.
Do not substitute pre-V3 upstream v0.1.0 binaries without validation.
