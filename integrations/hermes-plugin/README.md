# suflyor ↔ Hermes integration

Two-way bridge between **suflyor** (the Windows interview overlay) and a
**Hermes agent**. Design + rationale: `../../docs/goal-hermes-integration-2026-07-09.md`.

```
Hermes agent ──(HTTP, bearer)──▶ suflyor bridge :8654   (Hermes reads calls/memory/profiles)
suflyor      ──(HTTP, /v1/chat/completions)──▶ Hermes API :8642   (suflyor asks Hermes to prep a call profile)
```

Both directions are **off by default**. suflyor never exposes
audio/screenshots/secrets; Hermes writes are narrow (a memory *suggestion* that
you still approve, and a call-profile upsert).

---

## Direction 1 — Hermes reads suflyor (the plugin)

### Install — Hermes on the SAME machine (one click, no scripts)

In suflyor: **Настройки → Hermes** → enable «Мост для Hermes» → press
**«Установить плагин в Hermes»** → restart Hermes. The button (backed by
`overlay-backend/src/hermes_install.rs`, plugin sources embedded in the binary):

1. writes `plugin.yaml` + `__init__.py` into `<hermes home>/plugins/suflyor/`;
2. line-merges `SUFLYOR_BRIDGE_URL` / `SUFLYOR_BRIDGE_TOKEN` into `<hermes home>/.env`
   (nothing else in the file is touched);
3. adds `suflyor` to `plugins.enabled` in `<hermes home>/config.yaml` via a
   conservative text edit (comments preserved; exotic YAML shapes are left
   alone with a hint to run `hermes plugins enable suflyor`).

Hermes home = `HERMES_HOME` env var, else `%LOCALAPPDATA%\hermes` on Windows /
`~/.hermes` elsewhere — same resolution as hermes-agent itself.

Verify: `hermes plugins list` shows `suflyor` enabled; `/suflyor` in chat
prints “suflyor подключён”.

### Install — REMOTE Hermes (e.g. over Tailscale)

The in-app button installs into the *local* Hermes only. For a Hermes running
on another server:

1. In suflyor: Настройки → Hermes → set **«Хост привязки»** to this machine's
   Tailscale IP (`100.x.y.z`) or `0.0.0.0`, enable the bridge, copy the token.
2. Copy this directory's `suflyor/` folder to the server:
   `~/.hermes/plugins/suflyor/` (Linux default home).
3. On the server, append to `~/.hermes/.env`:
   ```
   SUFLYOR_BRIDGE_URL=http://<tailscale-ip-of-the-windows-machine>:8654
   SUFLYOR_BRIDGE_TOKEN=<token from suflyor settings>
   ```
4. `hermes plugins enable suflyor` (or add `suflyor` to `plugins.enabled` in
   `~/.hermes/config.yaml`), restart Hermes.

### Tools Hermes gets
| Tool | What |
|---|---|
| `suflyor_status` | bridge reachable + version |
| `suflyor_recent_sessions` | recent calls (id, time, status) — start here for an id |
| `suflyor_get_transcript` | full text transcript of one call (no audio) |
| `suflyor_get_summary` | a call's summary if present |
| `suflyor_search` | full-text search across all calls |
| `suflyor_get_memory` | approved personal memory (people/projects) |
| `suflyor_suggest_memory` | queue a fact for the owner to approve (does **not** add directly) |
| `suflyor_get_profiles` | call profiles + which is active |
| `suflyor_set_profile` | create/update a call profile (e.g. a researched company brief) |

Example asks to Hermes: *«что было на последнем созвоне?»*, *«найди во всех
созвонах где обсуждали зарплату»*, *«подготовь профиль для собеса в компанию X и
поставь его активным в suflyor»*, *«предложи в память suflyor, что Тимур —
тимлид»*.

---

## Direction 2 — suflyor asks Hermes (call-profile prep)

In suflyor → Настройки → Hermes: point **URL** at the Hermes API server (default
`http://127.0.0.1:8642/v1`), paste its **key** (`API_SERVER_KEY`). Then in the
profiles editor, type a seed line and press **«Подготовить профиль (Hermes)»** —
suflyor sends it to the agent (which can research), and writes the answer as a new
active profile. This is a *slow* agentic call — it never touches suflyor's live
answer path (that stays fast/local).

### Enable the Hermes API server (once)
In the Hermes `config.yaml` (`%LOCALAPPDATA%\hermes\config.yaml` on Windows):
```yaml
platforms:
  api_server:
    enabled: true
    extra:
      key: "<a strong secret>"      # == API_SERVER_KEY; also paste into suflyor
      # host: 127.0.0.1             # default; keep loopback
      # port: 8642                  # default
```
Restart Hermes. The brain model is your existing local Qwen (see the qwen-kit
handoff); no extra model config needed for suflyor.

---

## Security
- suflyor bridge: bearer required (blank token ⇒ refuses to start), text-only
  reads, body-capped, generic errors, bodies never logged. Binds to
  `127.0.0.1` by default; a non-loopback bind host (Tailscale scenario) shows
  a warning in Settings — keep it on a trusted network only.
- Hermes API server can drive terminal/file tools — keep it on loopback with a
  strong key; do **not** bind it to the LAN.
- The plugin has zero third-party deps (stdlib `urllib`), reads its token at call
  time, and forces no-proxy for a localhost bridge.
