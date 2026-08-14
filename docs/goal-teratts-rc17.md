# RC17: experimental TeraTTSv2 read-aloud sidecar (first releasable integration)

Status: branch `codex/teratts-sidecar-rc17`, based on `v0.36.1-rc.16`
(`cae818f`). Version bump: `0.37.0-rc.1` in `slint-experiment/Cargo.toml`
and `scripts/slint-installer.nsi` (guarded by `version_guard`).

## Outcome

Read-aloud gains an opt-in EXPERIMENTAL engine — TeraSpace/TeraTTSv2 ONNX
graphs (44.1 kHz, RU+EN, ten voice styles) running in a NEW standalone
sidecar `suflyor-teratts.exe` through `ort`. Piper stays the default engine
and the automatic fallback: whenever Tera is not installed, not ready, or
crashes, speaking keeps working through Piper. Speaker diarization is
untouched and ALWAYS stays in `suflyor-tts.exe`.

NOT introduced: no Python/transformers/`trust_remote_code` at runtime, no
model weights in the NSIS installer, no downloads from unofficial mirrors,
no GitHub release/tag actions.

## Pinned upstream contract

- Source of truth: `https://huggingface.co/TeraSpace/TeraTTSv2`, immutable
  revision `f05ea799094571a3553904a555df3834fb0b963b`.
- Download URLs: `resolve/<revision>/<path>` per file (never a mutable branch
  or the generic HF cache).
- Graph contract orchestrated by `suflyor-teratts` (mirrored from upstream
  `teratts.py` at that revision):
  - `text_encoder.onnx`: `text_ids i64[1,N]`, `style_ttl f32[1,50,256]`,
    `text_mask f32[1,1,N]` → `text_emb`.
  - `duration_predictor.onnx`: `text_ids` (duration text without `+`),
    `style_dp f32[1,8,16]`, `text_mask` → seconds via
    `raw * duration_scale / 1.05`.
  - `sampler_distilled_cfg3_8step.onnx`: `initial_latent f32[1,144,L]`
    (seeded Gaussian), `text_emb`, `style_ttl`, `latent_mask f32[1,1,L]`,
    `text_mask`, `guidance f32[1]=3.0` → latent; `L = ceil(sec*44100/3072)`.
  - `vocoder.onnx`: `latent f32[1,144,F]` → waveform; streamed with the
    reference causal overlap-save decode (20-frame context, 16-frame chunks,
    3072 samples/frame), trimmed to `round(sec*44100)` samples.
  - `unicode_indexer.json` (65,536-entry BMP table) tokenizes NFKD text;
    text normalization ports the reference pipeline (NFC → punctuation/digit
    spacing → vocabulary filter → balanced `<ru>/<en>` tag validation →
    per-language number expansion → NFKD).
- Integrity: every manifest file pins SHA-256 (LFS files) or the git blob
  SHA-1 (regular files) exactly as published by the Hub tree API for that
  revision. One JSON manifest (`suflyor-teratts/manifest/teratts-v2.json`) is
  compiled into BOTH the sidecar and the installer so they cannot drift.

## Known compatibility gaps (deliberate for RC17)

1. **RUAccent automatic Russian stress is NOT orchestrated yet.** The bundled
   RUAccent NN subtree (4 graphs + dictionaries + HF tokenizers) is excluded
   from the RC17 manifest and download (~370 MB core instead of ~873 MB full
   release). Manual `+` stress markers are honoured; unmarked Russian text is
   synthesized as-is. Porting RUAccent is follow-up work.
2. Number expansion covers the common num2words grammar (RU/EN cardinals +
   short decimals); fractions longer than 4 decimal digits are spelled
   digit-by-digit instead of the exact num2words wording.

## Security contract

- The sidecar never reads `config.json`/secrets; stdin carries base64 text
  only, stdout carries id/token status lines only, logs carry counts — never
  spoken text or credentials.
- Invalid base64/commands are answered with `REJECTED reason=…` (observable
  by host tests), never executed.
- Downloads verify-before-use; a hash or size mismatch wipes the staging dir.
  Publish is ONE directory rename after a `manifest.json` marker is written
  inside staging. Cancel between files wipes staging; a published release
  dir is immutable (`teratts-v2-<revision>`).
- Every spawned process keeps `CREATE_NO_WINDOW` (the `no_window_guard`
  integration test covers the new spawns).

## Smallest implementation map

### 1. New crate `suflyor-teratts` (sidecar binary)

Modules: `protocol` (stdin commands + stdout event lines), `textnorm`
(reference normalization), `num2words` (RU/EN), `indexer`, `npy` (style
assets), `rng` (seeded latent noise), `tera` (ORT orchestration),
`playback` (WASAPI, adapted from suflyor-tts incl. the stereo-duplicate
fix), `manifest` (pinned contract + installed check), `chunk` (350-char
sentence-aware splitting). Clippy denies `unwrap_used`/`expect_used`/`panic`
like the sibling crates.

### 2. overlay-backend

- `teratts_install.rs`: manifest parse/validate, streaming SHA-verify,
  staging + resumable downloads, generation-friendly cancel flag, atomic
  publish, `installed_state()` for the UI.
- `tts.rs`: `EngineKind` (`piper|tera`), namespaced `VoiceRef`
  (`piper:<dir>` / `tera:<style>`, bare legacy ids → Piper), Tera READY
  handshake parser, dual sidecar clients, crash-count bypass
  (`TERA_CRASH_LIMIT = 3`), Piper fallback inside `speak()`,
  pause/resume/stop routed to the last accepting engine.
- `diarize.rs`: diarization resolves `suflyor-tts.exe` via its OWN
  `diarization_exe_path()` — the read-aloud/diarization sidecar paths are
  split and cannot be conflated.
- `config.rs`: `tts_engine` (serde default; machine-local, not carried by
  server-settings import), `tts_voice` documented as namespaced.

### 3. Host (slint-experiment)

- `overlay_host.rs`: `tts::init(engine, voice, rate, lang)` with
  `response_language` as the Tera language tag.
- Settings → Read aloud: engine chooser (Piper / Tera (experimental)),
  per-engine voice lists, Tera model status + Install model / Cancel with
  phase text. New transient props (`tera-model-status`,
  `tera-install-phase/-label`, `tera-installing`) are reset in
  `populate_token_status` (satisfies `settings_reset_guard`).
- i18n: 7 new `@tr` strings + RU pairs in `slint-replay.po`
  (satisfies `i18n_guard`); ASCII markers only (no tofu glyphs).

### 4. Installer / CI

- NSIS ships `suflyor-teratts.exe` (binary only — NEVER weights); build +
  delete lines added. `build-slint-release.ps1` builds the crate into the
  shared target dir. `scripts/ci.ps1` gates fmt/clippy/test on the crate.

### 5. Licensing

`suflyor-teratts/NOTICE.md`: upstream publishes NO LICENSE; the owner's
Telegram "MIT" statement is sufficient to develop, NOT sufficient to claim
verified public redistribution. The notice defines the release gate
(archived author grant covering code, weights, styles, dictionaries,
commercial redistribution) and preserves the upstream RUACCENT_NOTICE text
verbatim. RC17 ships the sidecar + integration code only; weights reach a
user's machine solely by the user's own on-demand download from upstream.

## Verification gates

1. Focused: `cargo test` for `suflyor-teratts` (protocol/normalization/
   num2words/npy/manifest/rng), overlay-backend `teratts_install` + `tts`
   modules (hermetic: injected downloader, tempdir staging, no network).
2. Full `scripts/ci.ps1` green with `CARGO_INCREMENTAL=0`,
   `CARGO_BUILD_JOBS=2`.
3. Owner visual acceptance of the Settings → Read aloud surface using
   `docs/retest-v0.37.0-rc.1.html`; Winbrat quality benchmarks decide when
   Tera may leave experimental/fallback status (out of scope here).

## P1 hardening (post-audit; version stays 0.37.0-rc.1)

Four independently-audited P1 findings were closed on the same branch, with
no protocol/format break and no release action:

1. **Self-healing install** (`overlay-backend/src/teratts_install.rs`): on
   Windows `fs::rename` cannot replace an existing directory, so a broken
   release (missing marker, corrupt/short files, nonempty junk) made every
   re-install fail at publish forever. `install_with` now validates the
   existing release; an invalid one is moved to a uniquely named
   `teratts-v2-<rev>.broken-<ms>-<pid>-<n>` quarantine INSIDE the managed
   tts root (guarded by a component-wise containment check that rejects
   drive-prefix neighbours and `..` traversal) before the atomic publish.
   Valid installs early-return untouched; quarantines are swept best-effort
   after a successful publish; cancel before quarantine leaves the broken
   dir for the next run.
2. **Strict ONNX schema** (`suflyor-teratts/src/tera.rs`): each pinned graph
   must declare EXACTLY ONE output (checked at load), and runtime outputs
   are selected by that exact declared name — never positional iteration.
   Every output shape/data length is validated BEFORE slicing (encoder/
   duration products, exact `[1,144,L]` sampler latent, `[1,S]` vocoder
   waveform with the overlap-save minimum), so a mismatched model yields a
   generic `synth` FAILED token — never a panic, never user text (reason
   tokens are additionally sanitized to protocol-safe ASCII).
3. **Cancellation generations** (`suflyor-teratts/src/main.rs`): synthesis
   moved to a dedicated worker thread; the active utterance id is the
   generation. STOP stops playback immediately (never waits for CPU
   synthesis) and a newer SPEAK supersedes in-flight synthesis; stale
   results are dropped before playback. The worker loop is fully
   event-driven (stdin commands, synth results, and playback-exit
   notifications on one channel — no polling sleeps). Deterministic unit
   tests drive a fake player/dispatcher: stop-during-synthesis, newer-speak
   supersession, exactly-one-terminal-event-per-STARTED, stale-result drop.
4. **Host fallback hygiene** (`overlay-backend/src/tts.rs`): the STT
   suppression window is marked ONLY after the SPEAK line is actually
   accepted by the sidecar stdin; a Tera write failure falls back to Piper
   within the same `speak()` call; PAUSE/RESUME/STOP deliver only to a live
   child (a dead sidecar is never respawned just to receive a control line).
   Piper fallback and the independent diarization path
   (`diarize::diarization_exe_path` → `suflyor-tts.exe`) are unchanged.

`docs/retest-v0.37.0-rc.1.html` gained three matching checklist items
(self-healing install, STOP/interrupt during synthesis, dead-engine
suppression hygiene).

## Explicitly out of scope

- RUAccent neural stress orchestration + its 525 MB asset subtree.
- Teacher sampler (only the distilled 8-step graph is wired).
- Streaming-first synthesis (chunks synthesize whole, then stream to WASAPI).
- Any download/inference on the owner workstation during development.
- Publishing releases/tags, pushing branches, merging to master.
