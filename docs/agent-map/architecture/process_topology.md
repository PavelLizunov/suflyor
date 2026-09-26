# Subsystem Architecture & Process Topology

## 1. Process Boundaries & IPC Architecture

Suflyor isolates CPU-intensive and native runtime dependencies across distinct OS processes:

```mermaid
graph TD
    A[overlay-host.exe<br>Slint UI + In-Process GigaAM STT] -->|Line Protocol: Stdin/Stdout<br>Base64 JSON Chunks| B[suflyor-tts.exe<br>Piper TTS + sherpa-onnx Diarization]
    A -->|Line Protocol: Stdin/Stdout| C[suflyor-teratts.exe<br>TeraTTSv2 ONNX Runtime]
    A -->|Direct Static Link| D[overlay-backend<br>AI, Audio, SQLite, Settings]
    A -->|Direct Static Link| E[suflyor-wsola<br>FFT Pitch-Preserving Stretch]
    D -->|Append-Only| F[(Journal Files<br>%APPDATA%/sessions/*.jsonl)]
    D -->|WAL Mode + FTS5| G[(SQLite Catalog<br>%APPDATA%/catalog.sqlite)]
```

## 2. Audio Capture & Resampling Pipeline

```mermaid
sequenceDiagram
    participant Mic as Microphone (WASAPI/CoreAudio)
    participant Sys as System Audio (Loopback)
    participant Dec as Decimator (16kHz Mono)
    participant VAD as RMS VAD & Noise Gate
    participant STT as GigaAM / Whisper STT
    participant UI as Slint Overlay Bar

    Mic->>Dec: Native Samples (48kHz/44.1kHz)
    Sys->>Dec: Native Samples (48kHz/44.1kHz)
    Dec->>VAD: Decimated i16 Samples (16kHz)
    Note over VAD: RMS >= 50, 800ms Hangtime
    VAD->>STT: Bounded Utterance (<=10s)
    STT->>UI: Tagged TranscriptLine [Mic]/[System]
```

## 3. Slint Asynchronous Dispatch & Reactivity

```mermaid
graph LR
    subgraph UI Thread
        SlintEventLoop[Slint Event Loop]
        SlintWindow[Component Handle]
    end

    subgraph Background Thread
        TokioTask[Tokio Worker / Stream]
        WeakHandle[slint::Weak Reference]
    end

    TokioTask -->|1. Upgrade Weak| WeakHandle
    WeakHandle -->|2. invoke_from_event_loop| SlintEventLoop
    SlintEventLoop -->|3. Safe Mutation| SlintWindow
```
