# ADR-003 — Local model layer: engine contract, profiles, and agent harness

**Status:** Accepted. The direction below was chosen by the owner on 2026-10-14.
P4 (Strata) started on 2026-10-15 at the owner's request, before P1-P3; see
"P4 as built". Each phase needs its own verification gate (see "Phases").

**Date:** 2026-10-14

## Context

- The app talks to a local model through a single OpenAI-compatible request:
  `POST {base}/chat/completions`, streamed, with `messages` only. There is no
  `tools` field and no agent loop anywhere in `overlay-backend/src/ai/`
  (`completion.rs`, `stream.rs`, `provider.rs`). This works today.
- The bundled engine is llama.cpp. `update_llama_engine` pulls the latest
  `ggml-org/llama.cpp` release. The managed model catalog is closed: Gemma 4
  12B QAT, 26B-A4B and E4B, each with a pinned URL and SHA-256.
- A custom GGUF path exists. It only checks the `GGUF` magic bytes and launches
  with the `Unknown` hardware profile, so it gets no `-ncmoe` placement and
  relies on llama.cpp auto-fit. There is no memory-fit estimate.
- Target machine for the main user: 64 GiB RAM + 16 GiB VRAM (NVIDIA). The
  owner's DSH research (`homelab-infra-docs`, branch
  `origin/task/local-model-selection-20261007`) names Swift 27B IQ3_S for the
  small model and Swift Flash-Next IQ2_XS through Strata for the large model.
  Those numbers are not yet measured on the target.
- Qwen3.8-Flash-Next is a Qwen Community License 1.0 model. The license text
  does not address a free desktop app that downloads the weights at the user's
  request. The text names "AI Work Assistant" as a category that may need a
  separate license. Suflyor is close to that category. This ADR does not settle
  the question. Asking Qwen (model-business@notice.qwencloud.com) is the
  reliable route.

## Decisions

1. **License.** Suflyor stays GPL-3.0 (`LICENSE`, README). ADR-002 still says
   "already MIT-licensed at the source level". That line is stale and should
   be corrected when that document is next touched.

2. **Engine contract.** Only the OpenAI-compatible `/v1` contract is used. Any
   compliant server can be the engine. Default engine: llama.cpp, auto-updated
   as today. Optional engine: Strata, for Qwen3.8-Flash-Next profiles only, after
   a Windows test and a weights license check.

3. **Port.** Strata's documented base URL is `127.0.0.1:8080/v1`, the same port
   as the managed llama.cpp. Two servers on one port do not work together. The
   app's owner-aware `free_llama_port` only kills listeners under our root. A
   foreign listener on 8080 makes the switch fail with `PortBusy`. An optional
   engine must use its own port.

4. **Model profiles replace the closed catalog.** A profile holds: engine id,
   model source (local path, or HF `repo` + `revision` + `file` + `sha256`),
   context size, placement (`-ngl`, `--n-cpu-moe` / `-ot`), optional mmproj,
   thinking budget, and sampling parameters from the model card. Gemma stays as
   the default profile, so behaviour does not change until a profile is chosen.
   Show the model name and a link to its license in the UI.

5. **Derived weights.** Swift, Huihui, SC117, Heretic and similar fine-tunes
   carry their own cards and licenses. Do not add them to a default catalog
   until their license is checked.

6. **Harness = agent loop.** New module `overlay-backend/src/ai/agent.rs`. It
   reuses the SSE parser in `stream.rs` and adds `tools` plus streamed
   `tool_calls` deltas.
   - Tool set in v1, read-only: `kb_search` (wraps `kb::search`), `memory_recall`
     (approved memory items only, same bounds as `context_builder`), and screen
     or OCR context on explicit request. No shell and no file write in v1. This
     is deliberately smaller than `pi`'s read/bash/edit/write set.
   - Speed rules: short fixed system prompt; tool schemas kept identical across
     requests so the engine's prompt cache reuses the prefix; at most 4 tool
     steps per answer; a wall-clock budget per answer; each tool result capped
     (about 2 KB); parallel tool calls allowed.
   - Thinking: off for tool-selection steps. The final answer follows
     `ai_local_thinking`, which stays the user's switch.
   - Cancellation stops the loop. `check_deep_lock` still applies.
   - Capability probe: before the loop runs, send one request with `tools` and
     check that a valid `tool_calls` response parses. If the probe fails, fall
     back to today's plain single-shot path.

## Evidence from the owner's DSH campaign (read from `homelab-infra-docs`, not live)

- DSH 0.2.0-rc.2 runs a native agent loop with real read, write, edit, glob
  and grep tools against restricted workspaces. In the bounded "All downloaded
  Qwen" comparison, all 13 local candidates completed 4/4 tasks each (52/52).
  The two expanded finalists completed 8/8 at seeds 42 and 43 (32/32). The
  finalists were Flash IQ2_XS on Strata and 27B Uncensored. Both runs produced
  no recorded runtime or tool errors. This shows the tool-call path works for
  these serving stacks: llama.cpp/Prism, Strata and EXL3.
- The screen measures operational correctness only. It does not establish a
  quality winner. Quality campaign (SWE-bench Verified, Terminal-Bench 2.0,
  MMLU-Pro) was stopped on the owner's request on 2026-10-10 after one candidate:
  MMLU-Pro subset 115/140, coding 6/12, agent 2/2 of 16 completed.
- Recorded X3D state (2026-10-10): Swift Flash IQ3S served by Strata
  (262,144 context, 32,768 output, int8 KV, MTP 4), with CPU-only native vision.
- Live X3D state (checked 2026-10-15 over Tailscale, read-only): the served model
  is `swift-1.5-qwen3.8-27b-exl3-sc-3.00bpw-heretic` on `100.77.14.34:8080`
  (context 163,840, cache quant 4, MTP draft). `swift-flash-iq3s.service` is
  inactive and nothing listens on 8081. The switch back happened after
  2026-10-10 and is not recorded in the repository.
- Measured on live X3D logs (from `/models/hard-bench-20261008` and
  `/models/flash-next-bench-20261008`, not the repository):
  - 27B Heretic EXL3, short Russian answers: 43–45 decode tok/s, TTFT
    0.24–0.29 s.
  - 27B Heretic, 64,503-token prompt: TTFT 139.9 s. Long prompts dominate
    latency, so an agent loop must keep its prompt short and cached.
  - Context tests `context_nested_storage` and `context_approval`: 27B passed
    both. The IQ2 run passed `context_nested_storage` and failed
    `context_approval`. Some wall-clock timeouts are present, so these runs are
    incomplete.

## Open questions (must be checked, not assumed)

- Does tool use hold up in Russian, on the Swift fine-tunes, at 2–3 bit
  quantization? The DSH screen used English and Russian fixtures on the same
  stack. Its quality campaign is not finished, so there is no quality score.
- Does llama.cpp's `--jinja` template for Qwen3.8-Flash-Next produce parseable
  `tool_calls` on the pinned build? DSH ran llama.cpp/Prism. This ADR's
  implementation must still confirm it for Suflyor's own client.
- Reuse versus rewrite: DSH already contains a working agent loop with tools.
  Porting that design, or sharing its code, may be cheaper than writing
  `agent.rs` from scratch. This is an open decision for the owner.

## Phases

- **P0 — spike (no product code).** One Qwen3.8 GGUF, one `tools` request, one
  tool round-trip. Record the tool-call format, time to first token, and
  prefix-cache hit on repeat. Run on the DSH host or the Windows worker. Needs
  the owner's go-ahead, because the model download is about 68 GB.
- **P1 — profile layer.** Replace the hardcoded catalog with profiles. Gemma
  stays the default. No behaviour change. Verification: unit tests and the
  Windows gate.
- **P2 — agent loop behind `ai_local_agent` (default off).** Mock
  OpenAI-compatible server in tests: `tool_call` → tool result → final answer,
  plus the failure paths (bad JSON in arguments, step cap, timeout, cancel).
- **P3 — tools.** `kb_search` and `memory_recall`, with a bounded result size.
- **P4 — Strata as an optional engine profile.** Own port, pinned release,
  license check on the weights.
- **P5 — evaluation and default decision.** On target hardware: a fixed set of
  about 20 tool-use tasks (valid call rate, answer correctness), time to first
  token, total latency. Turn the agent on by default only with those numbers in
  hand.

## P4 as built (2026-10-15)

- `overlay-backend/src/local_ai/strata.rs` pins the Strata source release
  `v0.1.44.1` (size and SHA-256 of the GitHub tag archive). Strata is not one
  binary: its `START-HERE.bat` installs Python, a private `.venv`, the engine
  that fits the GPU and the model files. Suflyor runs it once with
  `--yes --family qwen --model IQ2_XS --no-start --no-browser --port 8090`, then
  starts `serve/server.py` itself, as Strata's own `run-<model>.bat` does.
- Port 8090 (decision 3). Ready means `/health` reports `loaded: true` and a
  four-token completion answers. Only one engine runs: starting Strata stops the
  managed llama.cpp, the built-in engine stops Strata, and a non-Strata local URL
  stops it too.
- Only the original Qwen3.8-Flash-Next IQ2_XS is offered (decision 5). The
  Settings row links to the model page for its licence. The licence question in
  "Context" is still open.
- Settings: the local provider shows one engine at a time (built-in llama.cpp,
  Strata, another server); the model list is a fixed grid with one action per
  row; one status line and one progress bar serve every built-in job; the
  context window, engine update, reinstall and connection test sit under
  "More settings".
- Not covered yet: the deep lock treats Strata as an external server (the bar
  chip only hides tiles), Strata vision, Strata updates (a new pinned release
  installs into a new folder; Strata finds its existing model files).

## P2-P3 as built, and an existing Strata (2026-10-15)

- `overlay-backend/src/ai/agent.rs` runs the loop inside the streamed answer:
  each step offers the tools and streams text to the tile; `tool_calls`
  deltas are collected by index, the calls run, and the next step asks again.
  At most 4 tool steps and 60 s of tool time; the step after that offers no
  tools. Each tool has 20 s and its result is capped at 4000 characters. The
  first request is the capability probe: an HTTP error there marks the server
  and the answer goes through the plain path.
- It runs only for the local provider (`is_local` OpenAI-compatible
  endpoints) and only after the user turns on "Model tools" in Settings
  (`ai_agent_tools`, default off). Cloud providers keep the plain path.
- Tools (`overlay-backend/src/ai/tools.rs`): `kb_search`; with the second
  switch `ai_web_search` (default off) also `web_search` and `open_url`.
  Web search uses DuckDuckGo's lite page without a key: its full HTML page
  returned a captcha to scripted requests on 2026-10-15, the lite page did
  not. `open_url` opens only public http(s) hosts: names and addresses on this
  PC, the LAN or Tailscale's 100.64/10 range are refused, after redirects and
  after DNS too. `memory_recall` is not built yet.
- The tile's state label shows the running tool ("AI · ищет в интернете:
  <query>"); the answer text does not change.
- Thinking is not switched off for tool steps yet: one streamed request serves
  both the tool choice and the answer.
- A Strata installed before Suflyor: Strata's own setup lists the folders it
  ran from in `%APPDATA%\Strata\settings.json` (`installs`, `data_dir`).
  Settings reads that file and the `Strata*` folders next to them, and shows
  the newest usable model config (`.venv`, `serve/server.py`, a
  `strata-<model>.json` with `exe` and `args`). "Find my Strata folder…"
  picks one by hand. Choosing it saves `ai_strata_dir` and
  `ai_strata_config` (machine-local, not exported) and starts that server on
  :8090 with its own config. Nothing is downloaded. A model the user already
  runs may be a fine-tune; its licence is the user's (decision 5 covers only
  what Suflyor offers to download).

## Verification rules

- `cargo` does not run on the workstation. Build, lint and test run on the
  Windows worker (`docs/winbrat-recovery.md`).
- Any change that alters visible UI goes through the Slint MCP smoke test
  (`scripts/slint_mcp.py`) before it is shown to the owner.
- Speed targets in this ADR are goals, not measurements. Record measured
  numbers next to them when they exist.

## Consequences

- The app stays engine-agnostic. Users can point it at any OpenAI-compatible
  server.
- The agent loop adds per-step latency. It is gated behind a flag until P5
  shows the cost on target hardware.
- Writing and shell tools are out of scope. Adding them later would need a
  separate decision and a confirmation UI.
