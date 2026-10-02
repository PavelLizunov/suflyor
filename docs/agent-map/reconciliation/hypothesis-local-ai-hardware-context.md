# Original local AI C08/C09: bounded hardware profile matching and context token limits evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_local_ai_hardware_context_hypotheses.py) inspect frozen hardware detection, VRAM normalization, and context configuration logic. They do not probe GPU VRAM, invoke llama.cpp, or allocate model weights. C08 and C09 remain hypotheses.

## C08 — unknown hardware profile and server argument defaults

In `local_ai.rs`, [llama_server_args](<../../../overlay-backend/src/local_ai.rs#L2058-L2132>) configures `-ngl`, `--no-mmap`, and `-np 1` only if `profile != HardwareModelProfile::Unknown || cfg!(target_os = "macos")` (or if `force_cpu` is explicitly set).
On Windows with `HardwareModelProfile::Unknown`, neither `-ngl` nor `-np` arguments are injected into the command invocation.
In [normalize_vram_gib](<../../../overlay-backend/src/local_ai/hardware_profile.rs#L98-L105>), only values within ±1 GiB of 8, 12, or 16 are snapped (e.g. 7..=9 -> 8); non-matching values (such as 6 GiB GPUs) pass through unchanged and map to `HardwareModelProfile::Unknown` in [select_hardware_model_profile](<../../../overlay-backend/src/local_ai/hardware_profile.rs#L75-L89>).

## C09 — context token limits and KV cache quantization

In `model_choice.rs`, [context_tokens](<../../../overlay-backend/src/local_ai/model_choice.rs#L100-L110>) accepts a `_prep: bool` parameter that is deliberately ignored (prefixed with `_`), computing `safe_live = profile.context_tokens(false)` regardless of whether `prep` mode is active.
In `llama_server_args`, when `prep` is true and the profile matches `Primary26Vram8` or `Primary26Vram12`, it appends `-ctk q8_0 -ctv q8_0` to quantize the KV cache to 8-bit, while keeping `-c` clamped to the live ceiling rather than expanding it.

## Limits

No GPU hardware queries were executed, no out-of-memory states were triggered, and no llama server was spawned. Original statuses in `candidates.json` remain `hypothesis`.
