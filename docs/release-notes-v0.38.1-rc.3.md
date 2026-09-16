# Suflyor v0.38.1-rc.3

This release candidate adds Windows system-audio capture device selection in Settings > Audio, allowing separate configuration of microphones and system audio capture endpoints (headphones/speakers), and resolves an upstream security advisory in `suflyor-tts`.

## Features and fixes

- **System audio endpoint selection**: Settings > Audio exposes separate selectors for microphone input and system audio capture (headphones/speakers).
- **Windows default & pinned endpoints**: Choosing 'Windows default' unpins the device, dynamically following the active Windows default endpoint; choosing an explicit endpoint pins it.
- **Unavailable device preservation**: Saved but disconnected devices remain visible with an unavailable warning notice instead of silently falling back or clearing the configuration.
- **Background refresh**: Audio endpoints refresh asynchronously when opening Settings and via the 'Refresh devices' button without freezing the interface.
- **Honest status indicators**: Explicit UI states for loading, empty device list, enumeration failure, and save failure rollback.
- **Headset capture sources**: System capture selector includes headset loopback/capture endpoints such as Astro A50 Stream Out.
- **Dependencies**: Updated `rustls` to 0.23.45 in `suflyor-tts` to resolve advisory `RUSTSEC-2026-0285`.

## Installers

- Windows 10/11: `suflyor-slint-setup.exe`.

The Windows installer is unsigned.
