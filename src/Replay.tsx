import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type SessionInfo = {
  path: string;
  filename: string;
  size_bytes: number;
  modified_unix: number;
};

// Every event in the JSONL has at minimum {kind, unix_ms, ...}.
type JournalEvent = {
  kind: string;
  unix_ms?: number;
  [k: string]: unknown;
};

function fmtClock(unix_ms?: number): string {
  if (!unix_ms || !Number.isFinite(unix_ms)) return "--:--:--";
  const d = new Date(unix_ms);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

function fmtBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(2)} MB`;
}

function fmtModified(unix: number): string {
  if (!unix) return "";
  const d = new Date(unix * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function preview(s: unknown, n: number): string {
  if (typeof s !== "string") return "";
  const t = s.replace(/\s+/g, " ").trim();
  return t.length > n ? t.slice(0, n) + "…" : t;
}

function asStr(v: unknown): string {
  return typeof v === "string" ? v : "";
}

function asNum(v: unknown): number | undefined {
  return typeof v === "number" ? v : undefined;
}

function asBool(v: unknown): boolean {
  return v === true;
}

// Map journal event kinds to the CSS variable used as the timeline
// row's left-border accent color, so the filter chip border matches
// what the user sees in the rows below. Kinds without a clear color
// fall back to neutral var(--c-border-soft).
function chipAccentForKind(kind: string): string {
  switch (kind) {
    case "session_start":
    case "session_stop":
    case "session_summary":
      return "var(--c-mic, #34d399)";
    case "transcript_line":
      return "var(--c-text-mute, #6b7280)";
    case "ai_request":
    case "ai_response":
      return "var(--c-ai, #818cf8)";
    case "tile_spawn":
      return "var(--c-auto, #f472b6)";
    case "detector_decision":
      // No on/off split at the chip level (we don't know which subset),
      // pick the "on" color so it still reads as a meaningful kind.
      return "var(--c-mic, #34d399)";
    case "rate_limited":
      return "#facc15"; // yellow, matches the overlay rate-limited chip
    case "error":
      return "#f87171"; // red, matches the timeline error row
    default:
      return "var(--c-border-soft)";
  }
}

// Journal stores cost as `cost_microcents` (u64 microcents = 10⁻⁸ USD)
// to avoid f64 drift. Legacy entries may also carry `cost_usd` directly.
function eventCost(e: JournalEvent): number {
  const micro = e["cost_microcents"];
  if (typeof micro === "number") return micro / 100_000_000;
  const usd = e["cost_usd"];
  return typeof usd === "number" ? usd : 0;
}

export default function Replay() {
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [selected, setSelected] = useState<string>("");
  const [events, setEvents] = useState<JournalEvent[]>([]);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string>("");
  // v0.0.11: filter chips. Set of EXCLUDED event kinds (click to toggle).
  // Default empty = show all. Stored per-session in state, not localStorage —
  // simpler model + each session has potentially different kinds anyway.
  const [hiddenKinds, setHiddenKinds] = useState<Set<string>>(new Set());

  useEffect(() => {
    document.body.classList.add("settings");
    return () => document.body.classList.remove("settings");
  }, []);

  useEffect(() => {
    invoke<SessionInfo[]>("list_sessions")
      .then((s) => {
        setSessions(s);
        // Auto-load the newest session for convenience.
        if (s.length > 0) {
          setSelected(s[0].path);
        }
      })
      .catch((e) => setErr(`list_sessions: ${e}`));
  }, []);

  useEffect(() => {
    if (!selected) {
      setEvents([]);
      return;
    }
    setLoading(true);
    setErr("");
    invoke<JournalEvent[]>("load_session", { path: selected })
      .then((es) => setEvents(es))
      .catch((e) => {
        setErr(`load_session: ${e}`);
        setEvents([]);
      })
      .finally(() => setLoading(false));
  }, [selected]);

  const totalCost = useMemo(() => {
    let sum = 0;
    let count = 0;
    for (const e of events) {
      if (e.kind === "ai_response") {
        sum += eventCost(e);
        count += 1;
      }
    }
    return { sum, count };
  }, [events]);

  // v0.0.11: distinct event kinds + count per kind. Sorted so common ones
  // (transcript_line, detector_decision) appear first. Used for filter chips.
  const kindCounts = useMemo(() => {
    const m = new Map<string, number>();
    for (const e of events) {
      const k = String(e.kind || "unknown");
      m.set(k, (m.get(k) || 0) + 1);
    }
    return Array.from(m.entries()).sort((a, b) => b[1] - a[1]);
  }, [events]);

  const visibleEvents = useMemo(() => {
    if (hiddenKinds.size === 0) return events;
    return events.filter(e => !hiddenKinds.has(String(e.kind || "unknown")));
  }, [events, hiddenKinds]);

  // Reset filter when switching session — different sessions have different
  // event kinds and stale filters would silently hide events the user just
  // loaded.
  useEffect(() => {
    setHiddenKinds(new Set());
  }, [selected]);

  const back = () => {
    window.location.search = "";
  };

  return (
    <main className="replay-root" aria-label="Session journal replay viewer">
      <div className="replay-header" role="banner">
        <h2 style={{ margin: 0 }}>📊 Session Replay</h2>
        <div className="replay-controls">
          <select
            value={selected}
            onChange={(e) => setSelected(e.target.value)}
            className="replay-select"
            aria-label="Choose a session to replay"
          >
            <option value="">— pick a session —</option>
            {sessions.map((s) => (
              <option key={s.path} value={s.path}>
                {s.filename} · {fmtBytes(s.size_bytes)} · {fmtModified(s.modified_unix)}
              </option>
            ))}
          </select>
          <button className="btn secondary" onClick={back} aria-label="Return to overlay">
            ← Back to overlay
          </button>
        </div>
      </div>

      {err && <div className="replay-error">{err}</div>}
      {loading && <div className="replay-status">Loading…</div>}
      {!loading && !err && selected && events.length === 0 && (
        <div className="replay-status">Empty session (no events).</div>
      )}
      {!loading && !selected && sessions.length === 0 && (
        <div className="replay-status">
          No sessions yet. Start a session from the overlay to populate this list.
        </div>
      )}

      {events.length > 0 && kindCounts.length > 1 && (
        <div
          style={{
            display: "flex",
            flexWrap: "wrap",
            gap: 6,
            padding: "6px 0",
            borderBottom: "1px solid var(--c-border-soft)",
            marginBottom: 6,
          }}
        >
          <span style={{ fontSize: 11, color: "var(--c-text-dim)", alignSelf: "center", marginRight: 4 }}>
            Filter:
          </span>
          {kindCounts.map(([kind, count]) => {
            const hidden = hiddenKinds.has(kind);
            // Color-code chip border to match the timeline row's accent color
            // for each kind. Makes visual scanning faster: a glance at the
            // chip strip shows the same colors you see in the timeline below.
            // Kinds not in this map fall back to neutral --c-border-soft.
            const accent = chipAccentForKind(kind);
            return (
              <button
                key={kind}
                onClick={() => {
                  const next = new Set(hiddenKinds);
                  if (hidden) next.delete(kind);
                  else next.add(kind);
                  setHiddenKinds(next);
                }}
                style={{
                  padding: "2px 8px",
                  fontSize: 11,
                  borderRadius: 12,
                  border: `1px solid ${hidden ? "var(--c-border-soft)" : accent}`,
                  background: hidden ? "transparent" : "var(--c-bg-2)",
                  color: hidden ? "var(--c-text-dim)" : "var(--c-text)",
                  textDecoration: hidden ? "line-through" : "none",
                  cursor: "pointer",
                }}
                title={hidden ? `Включить ${kind}` : `Скрыть ${kind} (${count} событий)`}
              >
                {kind} · {count}
              </button>
            );
          })}
          {hiddenKinds.size > 0 && (
            <button
              onClick={() => setHiddenKinds(new Set())}
              style={{
                padding: "2px 8px",
                fontSize: 11,
                borderRadius: 12,
                border: "1px solid var(--c-accent, #6366f1)",
                background: "transparent",
                color: "var(--c-accent, #6366f1)",
                cursor: "pointer",
                marginLeft: 4,
              }}
              title="Show all events"
            >
              ↺ reset
            </button>
          )}
        </div>
      )}

      {visibleEvents.length > 0 && (
        <div className="replay-timeline">
          {visibleEvents.map((e, idx) => (
            <ReplayRow key={idx} event={e} />
          ))}
        </div>
      )}

      {events.length > 0 && (
        <div className="replay-footer">
          <span>
            {events.length} events · {totalCost.count} AI responses
          </span>
          <span>
            {totalCost.sum > 0
              ? `Total cost: $${totalCost.sum.toFixed(4)}`
              : "Total cost: — (not tracked in journal yet)"}
          </span>
        </div>
      )}
    </main>
  );
}

function ReplayRow({ event }: { event: JournalEvent }) {
  const time = fmtClock(asNum(event.unix_ms));
  const kind = event.kind;

  switch (kind) {
    case "session_start": {
      const ctxChars = asNum(event.meeting_context_chars) ?? 0;
      const model = asStr(event.ai_model);
      const prepModel = asStr(event.prep_model);
      const lang = asStr(event.response_language);
      return (
        <Row time={time} cls="session-start" label="SESSION START">
          <span className="replay-meta">model={model}</span>
          {prepModel && <span className="replay-meta">prep={prepModel}</span>}
          <span className="replay-meta">ctx_chars={ctxChars}</span>
          {lang && <span className="replay-meta">lang={lang}</span>}
        </Row>
      );
    }
    case "session_stop":
      return (
        <Row time={time} cls="session-stop" label="SESSION STOP">
          <span />
        </Row>
      );
    case "session_summary": {
      const dur = asNum(event.duration_ms) ?? 0;
      const lines = asNum(event.transcript_lines) ?? 0;
      const mic = asNum(event.transcript_mic) ?? 0;
      const sys = asNum(event.transcript_system) ?? 0;
      const trig = asNum(event.detector_triggered) ?? 0;
      const skip = asNum(event.detector_skipped) ?? 0;
      const reqs = asNum(event.ai_requests_total) ?? 0;
      const errs = asNum(event.ai_errors) ?? 0;
      const tiles = asNum(event.tiles_spawned) ?? 0;
      const rl = asNum(event.rate_limited) ?? 0;
      const cost = (asNum(event.total_cost_microcents) ?? 0) / 100_000_000;
      const durMin = (dur / 60_000).toFixed(1);
      return (
        <Row time={time} cls="session-summary" label="SUMMARY">
          <span className="replay-meta">{durMin} min</span>
          <span className="replay-meta">
            {lines} lines ({mic}🎤 · {sys}🗣)
          </span>
          <span className="replay-meta">
            detector: {trig} / {trig + skip}
          </span>
          <span className="replay-meta">
            {reqs} AI · {tiles} tiles
            {rl > 0 ? ` · ${rl} rate-limited` : ""}
            {errs > 0 ? ` · ${errs} errors` : ""}
          </span>
          <span className="replay-meta">${cost.toFixed(4)}</span>
        </Row>
      );
    }
    case "transcript_line": {
      const source = asStr(event.source);
      const text = asStr(event.text);
      const icon = source === "mic" ? "🎤" : "🗣";
      return (
        <Row time={time} cls="transcript" label={`${icon} ${source}`}>
          <span className="replay-text">{text}</span>
        </Row>
      );
    }
    case "detector_decision": {
      const triggered = asBool(event.triggered);
      const text = asStr(event.text);
      const trigKind = asStr(event.trigger_kind);
      const cls = triggered ? "detector-on" : "detector-off";
      const reason = triggered ? `→ ${trigKind || "trigger"}` : "no trigger";
      return (
        <Row time={time} cls={cls} label={triggered ? "DETECT ✓" : "detect"}>
          <span className="replay-text">{preview(text, 200)}</span>
          <span className="replay-meta">{reason}</span>
        </Row>
      );
    }
    case "ai_request": {
      const purpose = asStr(event.purpose);
      const model = asStr(event.model);
      // Journal now stores the FULL user_prompt (was user_prompt_preview).
      // Fall back to legacy field name if reading an older journal.
      const userPrompt =
        asStr(event.user_prompt) || asStr(event.user_prompt_preview);
      const tokensEst = asNum(event.input_tokens_est);
      const hasShot = asBool(event.attached_screenshot);
      return (
        <Row time={time} cls="ai-req" label={`AI REQ · ${purpose}`}>
          <span className="replay-meta">{model}</span>
          {tokensEst !== undefined && (
            <span className="replay-meta">~{tokensEst} in-tok</span>
          )}
          {hasShot && <span className="replay-meta">📎 screenshot</span>}
          <span className="replay-text">{preview(userPrompt, 240)}</span>
        </Row>
      );
    }
    case "ai_response": {
      const purpose = asStr(event.purpose);
      const latency = asNum(event.latency_ms);
      const finish = asStr(event.finish_reason);
      const text = asStr(event.text);
      const cost = eventCost(event);
      return (
        <Row time={time} cls="ai-resp" label={`AI RESP · ${purpose}`}>
          {latency !== undefined && <span className="replay-meta">{latency} ms</span>}
          {finish && <span className="replay-meta">finish={finish}</span>}
          {cost > 0 && <span className="replay-meta">${cost.toFixed(4)}</span>}
          <span className="replay-text">{preview(text, 400)}</span>
        </Row>
      );
    }
    case "tile_spawn": {
      const question = asStr(event.question);
      const answer = asStr(event.answer);
      return (
        <Row time={time} cls="tile" label="TILE">
          <span className="replay-text"><strong>{preview(question, 80)}</strong></span>
          <span className="replay-text">{preview(answer, 100)}</span>
        </Row>
      );
    }
    case "rate_limited": {
      const what = asStr(event.what);
      const text = asStr(event.text);
      return (
        <Row time={time} cls="rate-limited" label={`RATE LIMITED · ${what}`}>
          <span className="replay-text">{preview(text, 240)}</span>
        </Row>
      );
    }
    case "error": {
      const module = asStr(event.module);
      const message = asStr(event.message);
      return (
        <Row time={time} cls="error" label={`ERROR · ${module}`}>
          <span className="replay-text">{message}</span>
        </Row>
      );
    }
    default: {
      // Unknown event — render JSON for debugging.
      return (
        <Row time={time} cls="unknown" label={kind || "unknown"}>
          <span className="replay-text">{JSON.stringify(event)}</span>
        </Row>
      );
    }
  }
}

function Row({
  time,
  cls,
  label,
  children,
}: {
  time: string;
  cls: string;
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className={`replay-row ${cls}`}>
      <span className="replay-time">{time}</span>
      <span className="replay-label">{label}</span>
      <span className="replay-body">{children}</span>
    </div>
  );
}
