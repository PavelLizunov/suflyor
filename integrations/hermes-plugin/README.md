# suflyor ↔ Hermes integration

Two-way bridge between **suflyor** (the local Windows interview overlay) and a
**local Hermes agent**. Design + rationale: `../../docs/goal-hermes-integration-2026-07-09.md`.

```
Hermes agent ──(HTTP, loopback, bearer)──▶ suflyor bridge :8654   (Hermes reads calls/memory/profiles)
suflyor      ──(HTTP, /v1/chat/completions)──▶ Hermes API :8642   (suflyor asks Hermes to prep a call profile)
```

Both directions are **local-only** and **off by default**. suflyor never exposes
audio/screenshots/secrets; Hermes writes are narrow (a memory *suggestion* that
you still approve, and a call-profile upsert).

---

## Direction 1 — Hermes reads suflyor (the plugin)

### Install
```powershell
# in the suflyor repo:
integrations\hermes-plugin\install.ps1
```
This copies `suflyor/` → `~/.hermes/plugins/suflyor/`. Then follow the two config
steps it prints:

1. **Enable** in `~/.hermes/config.yaml`:
   ```yaml
   plugins:
     enabled:
       - suflyor
   ```
2. **Token** in `~/.hermes/.env` (get it from suflyor → Настройки → Hermes, after
   enabling «Мост для Hermes»):
   ```
   SUFLYOR_BRIDGE_URL=http://127.0.0.1:8654
   SUFLYOR_BRIDGE_TOKEN=<token from suflyor settings>
   ```
Restart Hermes. Verify: `hermes plugins list` shows `suflyor` enabled; `/suflyor`
in chat prints “suflyor подключён”.

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
In `~/.hermes/config.yaml`:
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
- suflyor bridge: `127.0.0.1` only, bearer required (blank token ⇒ refuses to
  start), text-only reads, body-capped, generic errors, bodies never logged.
- Hermes API server can drive terminal/file tools — keep it on loopback with a
  strong key; do **not** bind it to the LAN.
- The plugin has zero third-party deps (stdlib `urllib`), reads its token at call
  time, and forces no-proxy for the localhost bridge.
