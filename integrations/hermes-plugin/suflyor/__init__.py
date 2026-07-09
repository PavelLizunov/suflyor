"""suflyor Hermes plugin — bridge to the local suflyor interview overlay.

Registers 9 tools + a `/suflyor` slash command. All I/O is a tiny stdlib
`urllib` client against suflyor's loopback bridge (no third-party deps).

Config (env, read at call time so toggling the bridge doesn't need a Hermes
restart):
- SUFLYOR_BRIDGE_URL   (default http://127.0.0.1:8654)
- SUFLYOR_BRIDGE_TOKEN (required; from suflyor → Настройки → Hermes)

Boundaries mirror the bridge: reads are text-only; `suflyor_suggest_memory`
does NOT approve memory (it queues a suggestion the human approves in
suflyor's «Память»); `suflyor_set_profile` writes a call-prep profile.
"""

from __future__ import annotations

import json
import os
import urllib.error
import urllib.parse
import urllib.request

_DEFAULT_URL = "http://127.0.0.1:8654"
_TIMEOUT = 15


def _base_url() -> str:
    return os.environ.get("SUFLYOR_BRIDGE_URL", _DEFAULT_URL).rstrip("/")


def _request(method: str, path: str, params: dict | None = None, body: dict | None = None) -> dict:
    """Call the bridge; return a dict (parsed JSON) or {"_error": "..."}.

    Never raises — a handler must return a string, so all failure modes
    (bridge off, wrong token, timeout, bad JSON) collapse to a message.
    """
    token = os.environ.get("SUFLYOR_BRIDGE_TOKEN", "").strip()
    if not token:
        return {"_error": "SUFLYOR_BRIDGE_TOKEN не задан (см. suflyor → Настройки → Hermes)"}
    url = _base_url() + path
    if params:
        url += "?" + urllib.parse.urlencode(params)
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Authorization", f"Bearer {token}")
    if data is not None:
        req.add_header("Content-Type", "application/json")
    # Loopback only — never route a localhost bridge call through a proxy.
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    try:
        with opener.open(req, timeout=_TIMEOUT) as resp:
            raw = resp.read().decode("utf-8", "replace")
        return json.loads(raw) if raw else {}
    except urllib.error.HTTPError as e:
        if e.code == 401:
            return {"_error": "мост отклонил токен (401) — проверь SUFLYOR_BRIDGE_TOKEN"}
        return {"_error": f"мост вернул HTTP {e.code}"}
    except urllib.error.URLError:
        return {"_error": "мост недоступен — включён ли он в suflyor → Настройки → Hermes?"}
    except (ValueError, TimeoutError) as e:
        return {"_error": f"ошибка запроса к мосту: {e}"}


def _pretty(obj) -> str:
    return json.dumps(obj, ensure_ascii=False, indent=2)


# ── tool handlers ───────────────────────────────────────────────────────────
# Each takes the tool-args dict and returns a human/LLM-readable string.

def _h_status(_args: dict) -> str:
    r = _request("GET", "/health")
    if "_error" in r:
        return f"suflyor: НЕДОСТУПЕН. {r['_error']}"
    return f"suflyor подключён (версия {r.get('version', '?')})."


def _h_recent(args: dict) -> str:
    limit = int(args.get("limit", 10) or 10)
    r = _request("GET", "/sessions", params={"limit": limit})
    if "_error" in r:
        return r["_error"]
    sessions = r.get("sessions", [])
    if not sessions:
        return "Созвонов пока нет."
    return _pretty(sessions)


def _h_transcript(args: dict) -> str:
    sid = str(args.get("session_id", "")).strip()
    if not sid:
        return "Нужен session_id (возьми из suflyor_recent_sessions)."
    params = {}
    if args.get("max_chars"):
        params["max_chars"] = int(args["max_chars"])
    r = _request("GET", f"/sessions/{urllib.parse.quote(sid)}", params=params or None)
    if "_error" in r:
        return r["_error"]
    lines = r.get("transcript", [])
    if not lines:
        return f"Сессия {sid}: транскрипт пуст."
    who = {"mic": "Я", "system": "Собеседник"}
    text = "\n".join(f"[{who.get(l.get('source'), l.get('source'))}] {l.get('text', '')}" for l in lines)
    if r.get("truncated"):
        text += "\n… (обрезано — увеличь max_chars)"
    return text


def _h_summary(args: dict) -> str:
    sid = str(args.get("session_id", "")).strip()
    if not sid:
        return "Нужен session_id."
    r = _request("GET", f"/sessions/{urllib.parse.quote(sid)}/summary")
    if "_error" in r:
        return r["_error"]
    return r.get("summary") or f"У сессии {sid} нет саммери."


def _h_search(args: dict) -> str:
    q = str(args.get("query", "")).strip()
    if not q:
        return "Нужен query."
    limit = int(args.get("limit", 10) or 10)
    r = _request("GET", "/search", params={"q": q, "limit": limit})
    if "_error" in r:
        return r["_error"]
    hits = r.get("hits", [])
    if not hits:
        return f"По запросу «{q}» ничего не найдено."
    return _pretty(hits)


def _h_get_memory(_args: dict) -> str:
    r = _request("GET", "/memory")
    if "_error" in r:
        return r["_error"]
    items = r.get("items", [])
    if not items:
        return "Одобренная память пуста."
    return "\n".join(f"- {it.get('text', '')}" for it in items)


def _h_suggest_memory(args: dict) -> str:
    text = str(args.get("text", "")).strip()
    if not text:
        return "Нужен text факта."
    reason = str(args.get("reason", "")).strip() or "Предложено Hermes"
    r = _request("POST", "/memory/suggest", body={"text": text, "reason": reason})
    if "_error" in r:
        return r["_error"]
    return "Факт добавлен в ОЧЕРЕДЬ предложений памяти suflyor — владелец одобрит его в разделе «Память»."


def _h_get_profiles(_args: dict) -> str:
    r = _request("GET", "/profiles")
    if "_error" in r:
        return r["_error"]
    active = r.get("active")
    profiles = r.get("profiles", [])
    if not profiles:
        return "Профилей нет."
    head = f"Активный профиль: {active or '(нет)'}\n"
    return head + "\n".join(
        f"— {p.get('name')}{'  ← активный' if p.get('name') == active else ''}\n  {p.get('context', '')[:400]}"
        for p in profiles
    )


def _h_set_profile(args: dict) -> str:
    name = str(args.get("name", "")).strip()
    context = str(args.get("context", "")).strip()
    if not name or not context:
        return "Нужны name и context профиля."
    set_active = bool(args.get("set_active", True))
    r = _request("POST", "/profiles", body={"name": name, "context": context, "set_active": set_active})
    if "_error" in r:
        return r["_error"]
    verb = "создан" if r.get("created") else "обновлён"
    tail = " и сделан активным" if r.get("active") else ""
    return f"Профиль «{name}» {verb}{tail} в suflyor."


# ── schemas (OpenAI function-calling shape) ─────────────────────────────────

def _schema(name: str, desc: str, props: dict, required: list[str]) -> dict:
    return {
        "name": name,
        "description": desc,
        "parameters": {"type": "object", "properties": props, "required": required},
    }


_STR = {"type": "string"}
_INT = {"type": "integer"}

_TOOLS = [
    (_schema("suflyor_status", "Проверить, что мост suflyor доступен, и узнать версию.", {}, []),
     _h_status, "🩺"),
    (_schema("suflyor_recent_sessions",
             "Список недавних созвонов suflyor (id, время, статус, число реплик). Начни отсюда, чтобы получить session_id.",
             {"limit": {**_INT, "description": "сколько сессий (по умолчанию 10)"}}, []),
     _h_recent, "🗂"),
    (_schema("suflyor_get_transcript",
             "Полный текстовый транскрипт одного созвона (реплики Я/Собеседник). Аудио НЕ отдаётся.",
             {"session_id": {**_STR, "description": "id из suflyor_recent_sessions"},
              "max_chars": {**_INT, "description": "лимит символов (по умолчанию 60000)"}},
             ["session_id"]),
     _h_transcript, "📄"),
    (_schema("suflyor_get_summary", "Саммери (итоги) одного созвона, если оно есть.",
             {"session_id": _STR}, ["session_id"]),
     _h_summary, "📝"),
    (_schema("suflyor_search",
             "Полнотекстовый поиск по всем созвонам suflyor (реплики + вопросы/ответы). Возвращает совпадения с session_id.",
             {"query": {**_STR, "description": "поисковый запрос"},
              "limit": {**_INT, "description": "сколько совпадений (по умолчанию 10)"}},
             ["query"]),
     _h_search, "🔎"),
    (_schema("suflyor_get_memory", "Одобренная пользователем персональная память suflyor (факты о людях/проектах).", {}, []),
     _h_get_memory, "🧠"),
    (_schema("suflyor_suggest_memory",
             "Предложить факт в память suflyor. НЕ добавляет напрямую — кладёт в очередь на одобрение владельцем.",
             {"text": {**_STR, "description": "факт (кратко)"},
              "reason": {**_STR, "description": "почему это стоит запомнить (опц.)"}},
             ["text"]),
     _h_suggest_memory, "➕"),
    (_schema("suflyor_get_profiles", "Профили созвонов suflyor (вводный контекст) + какой активен.", {}, []),
     _h_get_profiles, "👤"),
    (_schema("suflyor_set_profile",
             "Создать/обновить профиль созвона в suflyor (например подготовленная справка о компании/роли) и опц. сделать активным.",
             {"name": {**_STR, "description": "имя профиля"},
              "context": {**_STR, "description": "вводный контекст (справка к созвону)"},
              "set_active": {"type": "boolean", "description": "сделать активным (по умолчанию true)"}},
             ["name", "context"]),
     _h_set_profile, "✍️"),
]


def _cmd_suflyor(_raw: str) -> str:
    """`/suflyor` — quick status line."""
    return _h_status({})


def register(ctx) -> None:
    """Register all suflyor tools + the /suflyor slash command."""
    for schema, handler, emoji in _TOOLS:
        ctx.register_tool(
            name=schema["name"],
            toolset="suflyor",
            schema=schema,
            handler=handler,
            emoji=emoji,
        )
    ctx.register_command(
        name="suflyor",
        handler=_cmd_suflyor,
        description="Статус моста suflyor (созвоны/память/профили).",
    )
