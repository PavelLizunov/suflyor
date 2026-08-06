# Suflyor v0.36.1-rc.1

This release candidate adds a persistent deep lock for Suflyor's managed local
AI and fixes language-dependent labels in Settings.

## Highlights

- The bar lock now has a third state for the app-managed local AI: one click
  enables listening mode, the second unloads the managed model to free
  RAM/VRAM, and the third starts it again before unlocking.
- Deep lock survives restart, blocks managed-local requests and lifecycle
  operations, and stays locked if the model cannot be restarted safely.
- Cloud AI, external Ollama servers, local STT, and TTS keep working while the
  managed local AI is deep-locked. OCR remains available when routed through
  cloud or an external endpoint; OCR routed through the managed model is
  blocked with it.
- Component names, install hints, TTS voice names, and installer progress now
  follow the selected English or Russian interface language, including a live
  language switch.
- The installer now includes the matching DirectML runtime so the app starts on
  supported older Windows 10 systems whose built-in DirectML lacks the required
  API.

## Release-candidate note

This is a test build. Use the accompanying retest checklist before approving a
final release.
