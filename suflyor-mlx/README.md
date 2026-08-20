# suflyor-mlx

Native arm64 macOS 14.2+ loopback sidecar for one host-selected local MLX
snapshot. It never downloads models and keeps exactly one model resident.

The parent starts the executable without arguments and writes one bounded JSON
line to its owned stdin:

```json
{"version":1,"bearer":"<64 lowercase hex characters>","model":"<supported model ID>","snapshot":"<absolute local path>"}
```

The process validates and preloads that exact snapshot before binding
`127.0.0.1` on an ephemeral port. It then writes one JSON line to stdout:
`{"event":"READY","version":1,"port":12345,"model":"<selected model ID>"}`.
Every endpoint requires `Authorization: Bearer ...`. The parent keeps stdin
open for the child's lifetime; closing it shuts the server down.

Supported paths are `/health`, `/v1/models`, and `/v1/chat/completions`.
Only the selected model ID is accepted. Vision accepts inline JPEG and PNG data
URLs; remote image URLs are rejected. Requests are serialized, and cancellation
removes a queued request without allowing concurrent access to the model.
