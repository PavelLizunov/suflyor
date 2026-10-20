# Suflyor v0.38.1-rc.15

## Optional Strata Vision encoder (experimental, Windows)

Settings > Screenshots (F8) now offers **Add Vision to Strata** for a compatible
installed Qwen3.8-Flash-Next model. Select the Strata folder in AI providers
first; its bundled image helper must be present.

- **Download / Resume** fetches only the matching 907 MB image encoder, with
  an immutable revision and SHA-256 verification. No second language model
  or server is installed. Cancel and progress survive closing/reopening Settings.
- Downloading does **not** switch text, screenshot or speech-recognition routes,
  edit the Strata run configuration, or start the model.
- **Enable for F8** is separate. First use Strata for text answers. Activation
  configures CPU image encoding without an additional GPU memory reservation,
  starts the managed server, verifies a synthetic image and saves the F8 route
  only after success. On failure, the previous run configuration is restored.
- Existing configured encoders are retained rather than overwritten. An
  unsupported model family or missing image helper keeps installation unavailable.

## Verification and experimental limits

Native Windows checks cover the selected repository gate, English/Russian
Settings, actual encoder download/cancel/resume and failed-start rollback.
The complete encoder is 907,543,008 bytes; SHA-256:
`b1a82259702816a5330d7bd7607cd9676b11780e79ff7348c21103ff3ce49bd0`.

**Successful image inference with a real Strata model is not yet verified.**
The available Windows worker has 16 GiB RAM and no NVIDIA GPU; the approved
GPU peer was offline. Successful activation persistence and recovery of a
previously running real text server remain unverified. The owner explicitly
requested this experimental prerelease with those limits disclosed.

This does not make a text-only model compatible with an arbitrary projector.
The add-on is limited to the supported original Qwen3.8-Flash-Next quantizations.
It does not add Strata Vision to macOS; the existing MLX path is unchanged.
Physical macOS UI, actual speech/audio and the stable-only full test suite
are not claimed verified. The previously disclosed full overlay-bar geometry
failure and the owner's unverified transparent-rectangle/caption reports remain
separate; this prerelease does not claim to fix them.
