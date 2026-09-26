# Error Architecture & Failure Recovery Catalog

## 1. Error Philosophy & Clippy Enforcements
Suflyor strictly denies panics and unwrap calls in production code via crate-level lints:
- `clippy::unwrap_used = "deny"`
- `clippy::expect_used = "deny"`
- `clippy::panic = "deny"`

Every error path is handled explicitly via:
1. `anyhow::Result<T>` with rich context attachments (`.context("...")`) for high-level domain modules.
2. Direct `std::result::Result<T, WsolaError>` for core audio mathematical routines in `suflyor-wsola`.
3. Process-level sidecar isolation for third-party ONNX models (`sherpa-onnx` and `ort`).

## 2. Dedicated Error Enums

### `WsolaError` (`suflyor-wsola/src/error.rs`)
The time-stretching engine declares a structured enum:
- `InvalidSampleRate`: Raised when sample rate is zero or out of bounds.
- `InvalidSegmentSize`: Raised when window size is non-positive or exceeds maximum.
- `InvalidOverlap`: Overlap window exceeds segment duration.
- `InvalidSpeed`: Time-stretch factor <= 0.0 or exceeds practical bounds [0.1 .. 10.0].
- `BufferTooSmall`: Destination scratch buffer cannot accommodate window crossfade.
- `BufferOverflow`: Streaming buffer capacity exceeded.

## 3. Dynamic Error Patterns & Bailout Sites (461 Mapped Sites)
The repository uses structured `anyhow::bail!` and `.context()` markers:
- **Audio Capture Failure**: Recovers via automatic polling of the Windows multimedia notification client (`IMMNotificationClient`) or CoreAudio route change listeners.
- **AI Stream Transport Failures**: Maps HTTP connection drops to generic, screen-safe status messages ("AI connection error"), protecting internal LAN IPs from leaking into visible UI tiles.
- **SQLite Database Corruption**: Trapped at open time; triggers non-destructive VACUUM into pre-flight backups with zero data dropping.
