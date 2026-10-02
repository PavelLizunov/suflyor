# Original STT C01/C02: bounded task queueing and timeout cascade evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_stt_queue_network_confirmed.py) inspect frozen speech-to-text task queueing, concurrency semaphore acquisition, and retry timeout logic. They do not send HTTP requests, query Groq/Whisper endpoints, or transcribe audio. C01 and C02 remain confirmed mechanisms.

## C01 — in-flight task spawn preceding semaphore permit acquisition

In `stt.rs`, [start_stt](<../../../overlay-backend/src/stt.rs#L305-L310>) initializes a semaphore with 6 permits:
`let stt_semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(6));`.
In the live audio loop for both GigaAM ([L513-L522](<../../../overlay-backend/src/stt.rs#L513-L522>)) and HTTP Whisper ([L552-L557](<../../../overlay-backend/src/stt.rs#L552-L557>)), the code executes `tokio::spawn(async move { ... })` **first**, capturing the `to_send.samples` audio vector into task memory, and only then awaits `sem.acquire_owned().await` inside the spawned task.
The outer VAD loop does not wait on the semaphore before spawning, so incoming speech flushes allocate new tasks in memory that queue on the semaphore.

## C02 — client timeouts, retry delays, and error classification

In `stt.rs`, the live streaming HTTP client is configured with a 30-second timeout ([start_stt](<../../../overlay-backend/src/stt.rs#L348-L352>)):
`reqwest::Client::builder().timeout(Duration::from_secs(30))`.
In contrast, [transcribe_once](<../../../overlay-backend/src/stt.rs#L776-L815>) sets a 60-second client timeout.
In [is_permanent_error](<../../../overlay-backend/src/stt.rs#L872-L878>), only HTTP 401, 403, 404, and 413 short-circuit retries; HTTP 429 rate limits, 5xx server errors, and network disconnects are retried up to 3 attempts with exponential delays `1000 * (1 << (attempt - 1))` (1s, 2s) without jitter or `Retry-After` header parsing.

## Limits

No cloud Groq API requests were made, no local Whisper servers were queried, and no network timeout cascades were simulated. Original statuses in `candidates.json` remain `confirmed`.
