# macOS Settings RC2 fast visual audit

- Candidate: `7679a4cce2ccc609af7c8e690d5b42cdb5337de4`
- Worker: physical Apple Silicon macOS worker
- QA binary: debug build with `ui-mcp` and `SLINT_EMIT_DEBUG_INFO=1`
- MCP window: Settings at 720 × 600 logical pixels, scale factor 1.0
- Language: Russian
- Theme: default fresh-profile theme
- State: isolated empty QA profile; no credentials or personal data

The user approved a fast-path audit for this RC. The committed captures cover the four directly changed states only; the full 16-tab, English, baseline, and global-hotkey matrices were intentionally omitted.

Observed through MCP accessibility state:

- Backup contains `Версия: 0.38.0-rc.2 (suflyor / Slint)`.
- Text and Vision Managed MLX cards expose their model details and explicit Enable actions.
- The short read-aloud user label is 10 px high and its bordered bubble is 54 px high inside a 360 px tile.
- An independent image review passed all four captures with no clipping, overlap, unreadable text, missing MLX action, or residual bubble stretching.

## Capture hashes

- `after-backup-version-ru.png`: `878be0da5fcf23a1ebb28a88248321b96e572ace326fafd93794b5f0440eb47e`
- `after-ai-mlx-text-ru.png`: `5d17ff16c69cd64a7eb248f5d896f0ff43c63a482e0c8c923ef203395afd8133`
- `after-ai-mlx-vision-ru.png`: `746ca7a72ed0ed2595a1754631d60361b36a4bbb4abdcaf5592b0909a4c5bf4a`
- `after-read-aloud-short-text-ru.png`: `2fd4ddd01df5520dad605afd3858ce787353336b90b73468ce0297dd500c942e`
