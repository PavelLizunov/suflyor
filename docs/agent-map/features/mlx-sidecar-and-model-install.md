# MLX sidecar and model installation: source-linked contract

**Evidence:** Rust/Swift source inspection at `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`; no Swift/Cargo build, Apple GPU inference, model install, TCC interaction or benchmark executed. Swift package is an additional platform sidecar, not a root Cargo workspace member.

## Catalog and file verification

[MLX catalog](../../../overlay-backend/src/mlx_install.rs#L24-L219) records three reviewed model IDs: LFM text, Qwen text/vision and Gemma text/vision, exact revisions, filenames/bytes/SHA and declared license strings. Catalog `license` is source metadata, not independently verified grant for every upstream weight/template file.

[Role filter/path](../../../overlay-backend/src/mlx_install.rs#L221-L285) distinguishes text/image role and managed snapshot directory. `installed_snapshot` performs fast marker/presence checks; `installed_snapshot_verified` hashes full files. [Runtime start](../../../overlay-backend/src/mlx_runtime.rs#L417-L421) uses the fast form, not a new full-byte audit before every load.

[Installer](../../../overlay-backend/src/mlx_install.rs#L287-L418) is macOS-only, validates catalog/safe filenames, rejects symlink state, checks disk space, resumes staging partials, verifies each downloaded file and publishes marker then directory rename. Wrong partials are removed. Existing verified final snapshot returns early. Resource/timeout/cancellation behavior under huge model transfer still needs native test; atomic rename does not establish power-loss durable install success.

## Owned child lifecycle and request lease

[Runtime state](../../../overlay-backend/src/mlx_runtime.rs#L15-L112) holds actual child/stdin, owned endpoint, generation, active_requests and model load timing. Start/stop share lifecycle mutex; [start_locked](../../../overlay-backend/src/mlx_runtime.rs#L149-L183) rejects non-macOS. [stop_if_idle/acquire_request](../../../overlay-backend/src/mlx_runtime.rs#L209-L250) avoid replacing a model used by active request and return generation-scoped request lease. Normal lease Drop decrements matching generation; old lease does not count against newer child.

[start_macos](../../../overlay-backend/src/mlx_runtime.rs#L398-L539) verifies selected catalog/model readiness, stops old exact child, mints OS-random bearer, launches adjacent `suflyor-mlx` with pipes and sends startup JSON through stdin. It drains structured/sanitized stderr, waits up to 180 seconds for bounded READY, validates exact version/model/nonzero port, probes authenticated health and models, then installs endpoint only if intent still current/deep lock off. It does not bind a fixed external listener or read live model credentials from command line.

[Ready parse](../../../overlay-backend/src/mlx_runtime.rs#L286-L308) and [probe](../../../overlay-backend/src/mlx_runtime.rs#L542-L588) establish owned startup protocol/liveness shape. Full real generation/cancel/no-memory-pressure acceptance remains separate. Stopping an owned child closes stdin, then waits/kills as required; never apply generic port sweep to arbitrary macOS service.

## Swift protocol and server

[Package manifest](../../../suflyor-mlx/Package.swift) pins Swift dependencies and macOS 14.2+; [entrypoint](../../../suflyor-mlx/Sources/SuflyorMLX/main.swift) is arm64-only/no args and reads bounded startup line, emitting generic failure if invalid. [Startup config](../../../suflyor-mlx/Sources/SuflyorMLXCore/Protocol.swift#L15-L76) requires version1, selected supported model, absolute snapshot and exactly 64 lowercase hex bearer; host fixed loopback, ephemeral port0.

[Request parser](../../../suflyor-mlx/Sources/SuflyorMLXCore/Protocol.swift#L78-L118) validates role/model/messages/temperature/max output tokens (1..32768). [Image decoding](../../../suflyor-mlx/Sources/SuflyorMLXCore/Protocol.swift#L159-L192) accepts inline JPEG/PNG only and caps decoded image 16MiB; remote URLs rejected. These are selected fields/decoded image caps, not a universal HTTP request/body/messages/token memory budget.

[GenerationGate](../../../suflyor-mlx/Sources/SuflyorMLXCore/Server.swift#L204-L247) serializes model access and removes canceled waiters. Its waiting array itself has no shown count limit; serializing active inference is not bounded incoming request queue memory. [ModelEngine](../../../suflyor-mlx/Sources/SuflyorMLXCore/Server.swift#L254-L394) caches one selected container, handles no-think/filter, cancellation checks, completion metadata, clears MLX cache and releases gate. Its AsyncThrowingStream buffering/cancel propagation are native behavior checks, not benchmarked source claims.

[Routes/server](../../../suflyor-mlx/Sources/SuflyorMLXCore/Server.swift#L401-L516) require bearer on health/models/chat, enforce requested selected model, emit OpenAI-compatible JSON/SSE and READY after preloading before service starts. stdin EOF races service lifetime through task group cancellation. Preload warmup failure can be swallowed after container load; READY is not proof every future inference succeeds.

## Host routing and platform constraints

[AI managed MLX resolver](../../../overlay-backend/src/ai/control.rs#L100-L128) substitutes real owned URL/model/bearer and holds request lease; [Config](../../../overlay-backend/src/config.rs#L746-L759) uses placeholder only when model catalog exists. [deep lock](../../../overlay-backend/src/deep_lock.rs#L34-L66) blocks managed MLX route and host can stop owned sidecar on deep lock when safe.

The macOS worker policy requires no concurrent heavy/native model processes, >=40% free memory and jobs/tests=2 before build; [gate](../../../scripts/git-gate-macos.sh#L6-L103) covers Swift and Rust seams. Those commands were not run on DSH; isolated Linux research tests do not prove Apple GPU behavior.

[Swift source tests](../../../suflyor-mlx/Tests/SuflyorMLXCoreTests) and Rust inline runtime/install tests are declarations only here. Acceptance must exercise large text/image, model alias, readiness mismatch, active-lease model switch, queued/active cancellation, stdin EOF/forced-parent teardown, permission/path/symlink/install errors and bounded memory on exact candidate. See [managed local contract](managed-local-ai.md) for distinct Windows llama lifecycle.
