import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

// Inline-toast + modal types. Replaces blocking window.prompt / alert /
// confirm — those break Tauri WebView focus and look like 1998 UX.
type ToastKind = "ok" | "err";
type Toast = { kind: ToastKind; text: string; ts: number };
type ModalState =
  | {
      kind: "prompt";
      title: string;
      placeholder?: string;
      initial: string;
      onSubmit: (v: string) => void;
      onCancel: () => void;
    }
  | {
      kind: "confirm";
      title: string;
      onYes: () => void;
      onNo: () => void;
    };

type Snippet = {
  key: string;
  title: string;
  body: string;
};

type KBEntry = {
  key: string;
  heading: string;
  body: string;
  source: "glossary" | "commands" | "patterns";
};

type KBStats = {
  total: number;
  glossary: number;
  commands: number;
  patterns: number;
};

type Config = {
  meeting_context: string;
  context_profiles: { name: string; context: string }[];
  active_profile: string | null;
  mic_device: string | null;
  system_audio_device: string | null;
  ai_base_url: string;
  ai_bearer: string;
  ai_model: string;
  prep_model: string;
  response_language: string;
  groq_api_key: string;
  stt_language: string | null;
  tile_monitor_name: string | null;
  trigger_keywords: string;
  auto_tiles_enabled: boolean;
  hotkey_ask: string;
  hotkey_screenshot: string;
  hotkey_toggle_visibility: string;
  hotkey_pause_audio: string;
  stealth_enabled: boolean;
  snippets: Snippet[];
  post_meeting_debrief_enabled?: boolean;
};

type DeviceList = { outputs: string[]; inputs: string[] };

export default function Settings() {
  const [cfg, setCfg] = useState<Config | null>(null);
  const [devices, setDevices] = useState<DeviceList>({ outputs: [], inputs: [] });
  const [monitors, setMonitors] = useState<string[]>([]);
  const [savedFlash, setSavedFlash] = useState(false);
  const [recState, setRecState] = useState<"idle" | "recording" | "structuring">("idle");
  const [recCountdown, setRecCountdown] = useState(0);
  const [recError, setRecError] = useState("");
  // Controlled state for the cost-indicator toggle. Seeded once from
  // localStorage; we write back on every change so the overlay's storage-
  // event listener picks it up.
  const [showCost, setShowCost] = useState<boolean>(() => {
    try { return localStorage.getItem("overlay.showCost") !== "false"; }
    catch { return true; }
  });
  // KB search state (embedded glossary + commands + patterns).
  const [kbStats, setKbStats] = useState<KBStats | null>(null);
  const [kbQuery, setKbQuery] = useState("");
  const [kbResults, setKbResults] = useState<KBEntry[]>([]);
  const [kbBusy, setKbBusy] = useState(false);
  // Inline toast + modal — replaces blocking window.alert/prompt/confirm.
  const [toast, setToast] = useState<Toast | null>(null);
  const [modal, setModal] = useState<ModalState | null>(null);
  const [promptValue, setPromptValue] = useState("");
  // Snippets section state: collapsed by default + filter, otherwise 57
  // entries make the Settings page scroll forever (live regression
  // 2026-05-25: "snippet бесконечно длинный список").
  const [snippetsExpanded, setSnippetsExpanded] = useState(false);
  const [snippetFilter, setSnippetFilter] = useState("");
  const toastTimerRef = useRef<number | null>(null);
  const mountedRef = useRef(true);
  // Pending modal resolver — captured so unmount can reject open prompts
  // instead of hanging awaiting callers forever.
  const pendingModalRejectRef = useRef<null | (() => void)>(null);
  const showToast = useCallback((kind: ToastKind, text: string) => {
    if (!mountedRef.current) return;
    if (toastTimerRef.current) {
      window.clearTimeout(toastTimerRef.current);
      toastTimerRef.current = null;
    }
    setToast({ kind, text, ts: Date.now() });
    toastTimerRef.current = window.setTimeout(() => {
      if (!mountedRef.current) return;
      setToast(null);
      toastTimerRef.current = null;
    }, kind === "err" ? 6000 : 3500);
  }, []);
  useEffect(() => {
    // RESET mountedRef on every mount — critical for React StrictMode
    // which mounts→unmounts→re-mounts the component in dev. Without this
    // reset, the cleanup from the first mount sets mountedRef=false, and
    // the second mount inherits the same ref (useRef preserves value), so
    // every showPrompt/showConfirm early-exits silently. CAUSED THE MODAL
    // CLICK BUG live-discovered 2026-05-25.
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      if (toastTimerRef.current) window.clearTimeout(toastTimerRef.current);
      // Resolve any pending modal so awaiting callers don't hang.
      if (pendingModalRejectRef.current) {
        pendingModalRejectRef.current();
        pendingModalRejectRef.current = null;
      }
    };
  }, []);
  const showPrompt = useCallback((title: string, placeholder?: string, initial = "") =>
    new Promise<string | null>((resolve) => {
      if (!mountedRef.current) { resolve(null); return; }
      pendingModalRejectRef.current = () => resolve(null);
      setPromptValue(initial);
      setModal({
        kind: "prompt",
        title,
        placeholder,
        initial,
        onSubmit: (v) => {
          pendingModalRejectRef.current = null;
          setModal(null);
          resolve(v);
        },
        onCancel: () => {
          pendingModalRejectRef.current = null;
          setModal(null);
          resolve(null);
        },
      });
    }), []);
  const showConfirm = useCallback((title: string) =>
    new Promise<boolean>((resolve) => {
      if (!mountedRef.current) { resolve(false); return; }
      pendingModalRejectRef.current = () => resolve(false);
      setModal({
        kind: "confirm",
        title,
        onYes: () => {
          pendingModalRejectRef.current = null;
          setModal(null);
          resolve(true);
        },
        onNo:  () => {
          pendingModalRejectRef.current = null;
          setModal(null);
          resolve(false);
        },
      });
    }), []);
  // Esc-anywhere-to-cancel for confirm modals (prompt input already handles it).
  useEffect(() => {
    if (!modal || modal.kind !== "confirm") return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") modal.onNo();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [modal]);
  useEffect(() => {
    invoke<KBStats>("kb_stats").then(setKbStats).catch((e) => console.warn("kb_stats:", e));
  }, []);
  useEffect(() => {
    const q = kbQuery.trim();
    if (!q) { setKbResults([]); return; }
    setKbBusy(true);
    let cancelled = false;
    const t = setTimeout(() => {
      invoke<KBEntry[]>("kb_search", { query: q, limit: 12 })
        .then((res) => { if (!cancelled) setKbResults(res); })
        .catch((e) => { if (!cancelled) { console.warn("kb_search:", e); setKbResults([]); } })
        .finally(() => { if (!cancelled) setKbBusy(false); });
    }, 100); // debounce
    return () => { cancelled = true; clearTimeout(t); };
  }, [kbQuery]);

  useEffect(() => {
    document.body.classList.add("settings");
    const fetchAll = () => {
      invoke<Config>("get_config")
        .then((c) => { if (mountedRef.current) setCfg(c); })
        .catch((e) => console.warn("get_config:", e));
      invoke<DeviceList>("list_audio_devices")
        .then((d) => { if (mountedRef.current) setDevices(d); })
        .catch((e) => console.warn("list_audio_devices:", e));
      invoke<string[]>("list_monitors")
        .then((m) => { if (mountedRef.current) setMonitors(m); })
        .catch((e) => console.warn("list_monitors:", e));
    };
    fetchAll();
    // CRITICAL UX/data-safety bug fix (caught live 2026-05-25): if the
    // Tauri binary restarts while the Settings webview survives (tauri dev
    // hot-reload, cargo rebuild, manual kill+respawn), the cached config
    // in React state goes stale. Saving in that state would PERSIST stale/
    // empty values to disk and wipe real secrets+devices. Re-fetch on every
    // window-focus event so the page heals itself the moment the user
    // tabs back in.
    const onFocus = () => fetchAll();
    window.addEventListener("focus", onFocus);
    return () => {
      document.body.classList.remove("settings");
      window.removeEventListener("focus", onFocus);
    };
  }, []);

  if (!cfg) return <div style={{ padding: 24 }}>Loading…</div>;

  const update = (patch: Partial<Config>) => setCfg({ ...cfg, ...patch });

  const save = async () => {
    await invoke("save_config", { newCfg: cfg });
    setSavedFlash(true);
    setTimeout(() => setSavedFlash(false), 1500);
  };

  const RECORD_SECONDS = 30;

  const recordPrep = async () => {
    if (!cfg || recState !== "idle") return;
    setRecError("");
    setRecState("recording");
    setRecCountdown(RECORD_SECONDS);

    const tick = setInterval(() => {
      setRecCountdown((c) => Math.max(0, c - 1));
    }, 1000);

    try {
      const text = await invoke<string>("prep_record", {
        durationSecs: RECORD_SECONDS,
      });
      clearInterval(tick);
      const appended = cfg.meeting_context
        ? cfg.meeting_context.trim() + "\n\n" + text.trim()
        : text.trim();
      update({ meeting_context: appended });
      setRecState("idle");
    } catch (e) {
      clearInterval(tick);
      setRecError(String(e));
      setRecState("idle");
    }
  };

  const structurePrep = async () => {
    if (!cfg || recState !== "idle") return;
    if (!cfg.meeting_context.trim()) {
      setRecError("Сначала запишите или впишите текст");
      return;
    }
    setRecError("");
    setRecState("structuring");
    try {
      const structured = await invoke<string>("prep_structure", {
        rawText: cfg.meeting_context,
      });
      update({ meeting_context: structured });
      setRecState("idle");
    } catch (e) {
      setRecError(String(e));
      setRecState("idle");
    }
  };

  const back = async () => {
    // Restore overlay compact size + clear ?settings query, all via backend
    // so the window resize happens atomically with the route change.
    try {
      await invoke("close_settings");
    } catch (e) {
      console.error("close_settings:", e);
      window.location.search = "";
    }
  };

  return (
    <div className="settings-root">
      {/* Header is the drag region — overlay window has decorations:false
       * so without an explicit drag-region the user can't move the window
       * at all (live regression 2026-05-25). The data-tauri-drag-region
       * attribute alone proved unreliable on WebView2 in release build
       * (2nd live regression: "натройки почему-то все равно не двигаются"),
       * so we ALSO wire an explicit onMouseDown → startDragging() handler
       * as a belt-and-suspenders fallback. */}
      <div
        className="settings-header"
        data-tauri-drag-region
        title="Перетащи за этот заголовок чтобы подвинуть окно"
        onMouseDown={(e) => {
          // Only initiate drag on primary (left) button + when the target
          // is the header itself (not a child button). The button check
          // prevents the ✕ Выйти click from accidentally starting a drag.
          if (e.button !== 0) return;
          const target = e.target as HTMLElement;
          if (target.closest("button, input, select")) return;
          // Fire-and-forget — drag latency matters more than error reporting.
          getCurrentWindow().startDragging().catch((err) => {
            console.warn("startDragging failed:", err);
          });
        }}
      >
        <h2 style={{ marginTop: 0, marginBottom: 0 }} data-tauri-drag-region>
          ⋮⋮  Settings
        </h2>
        <button
          className="btn settings-modal-danger"
          style={{ height: 28, padding: "0 12px", fontSize: 12 }}
          onClick={async () => {
            const ok = await showConfirm(
              "Выйти из приложения? Текущая сессия захвата завершится, journal сохранится.",
            );
            if (ok) {
              try { await invoke("quit_app"); }
              catch (e) { showToast("err", `quit failed: ${e}`); }
            }
          }}
          title="Полностью завершить suflyor (с подтверждением)"
        >
          ✕ Выйти
        </button>
      </div>

      <div className="settings-section">
        <h3>👥 Профили контекста</h3>
        <div className="field">
          <label>Активный профиль</label>
          <select
            value={cfg.active_profile ?? ""}
            onChange={(e) => {
              const name = e.target.value || null;
              if (!name) {
                update({ active_profile: null });
              } else {
                const p = cfg.context_profiles.find((x) => x.name === name);
                update({ active_profile: name, meeting_context: p?.context ?? cfg.meeting_context });
              }
            }}
          >
            <option value="">— нет —</option>
            {cfg.context_profiles.map((p) => (
              <option key={p.name} value={p.name}>{p.name}</option>
            ))}
          </select>
        </div>
        <div className="btn-row" style={{ justifyContent: "flex-start", gap: 8 }}>
          <button
            className="btn secondary"
            onClick={async () => {
              const raw = await showPrompt(
                "Имя нового профиля",
                "K8s interview, Backend SRE, …",
              );
              const name = raw?.trim();
              if (!name) return;
              const profiles = [
                ...cfg.context_profiles.filter((p) => p.name !== name),
                { name, context: cfg.meeting_context },
              ];
              update({ context_profiles: profiles, active_profile: name });
              showToast("ok", `Профиль «${name}» сохранён`);
            }}
          >+ Сохранить текущий как профиль</button>
          {cfg.active_profile && (
            <button
              className="btn secondary"
              onClick={async () => {
                const ok = await showConfirm(`Удалить профиль «${cfg.active_profile}»?`);
                if (!ok) return;
                const removed = cfg.active_profile;
                const profiles = cfg.context_profiles.filter((p) => p.name !== cfg.active_profile);
                update({ context_profiles: profiles, active_profile: null });
                showToast("ok", `Профиль «${removed}» удалён`);
              }}
            >× Удалить активный</button>
          )}
        </div>
      </div>

      <div className="settings-section">
        <h3>📝 Meeting context</h3>
        <div className="field">
          <label>
            Контекст которой AI видит при каждом запросе (резюме, описание проекта, термины…)
          </label>
          <textarea
            value={cfg.meeting_context}
            onChange={(e) => update({ meeting_context: e.target.value })}
            placeholder="Например: Это собеседование на Senior SRE в Acme. Мой опыт: 7 лет K8s, etcd, networking…"
          />
        </div>
        <div className="btn-row" style={{ justifyContent: "flex-start", gap: 8 }}>
          <button
            className="btn secondary"
            onClick={recordPrep}
            disabled={recState !== "idle"}
            title="Запишет с микрофона 30 секунд и добавит транскрипт в поле выше"
          >
            {recState === "recording"
              ? `🔴 Идёт запись… ${recCountdown}с`
              : `🎤 Записать голосом (${RECORD_SECONDS}с)`}
          </button>
          <button
            className="btn secondary"
            onClick={structurePrep}
            disabled={recState !== "idle"}
            title={`Отправит текст в ${cfg.prep_model} с промтом структурирования и заменит на чистый контекст`}
          >
            {recState === "structuring"
              ? "✨ Структурирую через Sonnet…"
              : `✨ Структурировать (${cfg.prep_model})`}
          </button>
        </div>
        {recError && (
          <div style={{ color: "#ef4444", fontSize: 11, marginTop: 6 }}>
            {recError}
          </div>
        )}
      </div>

      <div className="settings-section">
        <h3>🎤 Audio devices</h3>
        <div className="field">
          <label>Microphone (your voice)</label>
          <select
            value={cfg.mic_device ?? ""}
            onChange={(e) => update({ mic_device: e.target.value || null })}
          >
            <option value="">— default —</option>
            {devices.inputs.map((d) => (
              <option key={d} value={d}>{d}</option>
            ))}
          </select>
        </div>
        <div className="field">
          <label>System audio (what they say) — выбери loopback устройство (для Astro A50: "Line (A50 Stream Out)")</label>
          <select
            value={cfg.system_audio_device ?? ""}
            onChange={(e) => update({ system_audio_device: e.target.value || null })}
          >
            <option value="">— default render endpoint loopback —</option>
            {devices.inputs.map((d) => (
              <option key={"in:" + d} value={d}>{d}</option>
            ))}
            {devices.outputs.map((d) => (
              <option key={"out:" + d} value={d}>{d} (loopback)</option>
            ))}
          </select>
        </div>
      </div>

      <div className="settings-section">
        <h3>🤖 AI proxy (your Claude bridge)</h3>
        <div className="field">
          <label>Base URL</label>
          <input
            type="text"
            value={cfg.ai_base_url}
            onChange={(e) => update({ ai_base_url: e.target.value })}
            placeholder="http://192.168.0.142:18902/v1"
          />
          {cfg.ai_base_url.trim().toLowerCase().startsWith("http://") && (
            <div
              style={{
                fontSize: 11,
                color: "var(--c-warn)",
                marginTop: 4,
                padding: "4px 8px",
                background: "color-mix(in srgb, var(--c-warn) 12%, transparent)",
                border: "1px solid color-mix(in srgb, var(--c-warn) 35%, transparent)",
                borderLeft: "3px solid var(--c-warn)",
                borderRadius: "var(--r-1)",
              }}
            >
              ⚠ Plaintext HTTP — bearer token + prompts travel in clear. Use https:// (Caddy/Nginx in front) for any non-localhost deployment.
            </div>
          )}
        </div>
        <div className="field">
          <label>Bearer secret (BRIDGE_SECRET)</label>
          <input
            type="password"
            value={cfg.ai_bearer}
            onChange={(e) => update({ ai_bearer: e.target.value })}
          />
        </div>
        <div className="field">
          <label>Модель для живых ответов (нужна скорость)</label>
          <select
            value={cfg.ai_model}
            onChange={(e) => update({ ai_model: e.target.value })}
          >
            <option value="claude-haiku-4-5">claude-haiku-4-5 (быстро, default)</option>
            <option value="claude-sonnet-4-6">claude-sonnet-4-6 (умнее, медленнее)</option>
            <option value="claude-opus-4-7">claude-opus-4-7 (самый умный, медленный)</option>
          </select>
        </div>
        <div className="field">
          <label>Модель для подготовки контекста (нужно качество)</label>
          <select
            value={cfg.prep_model}
            onChange={(e) => update({ prep_model: e.target.value })}
          >
            <option value="claude-sonnet-4-6">claude-sonnet-4-6 (default, 30-50% быстрее 4-5)</option>
            <option value="claude-sonnet-4-5">claude-sonnet-4-5 (старая, ещё работает)</option>
            <option value="claude-haiku-4-5">claude-haiku-4-5 (быстро)</option>
            <option value="claude-opus-4-7">claude-opus-4-7 (максимум качества)</option>
          </select>
        </div>
        <div className="field">
          <label>Response language (forced via system prompt)</label>
          <select
            value={cfg.response_language}
            onChange={(e) => update({ response_language: e.target.value })}
          >
            <option value="ru">Русский (ru)</option>
            <option value="en">English (en)</option>
          </select>
        </div>
      </div>

      <div className="settings-section">
        <h3>🎨 Интерфейс</h3>
        <div className="field">
          <label>
            <input
              type="checkbox"
              checked={showCost}
              onChange={(e) => {
                const v = e.target.checked;
                setShowCost(v);
                try { localStorage.setItem("overlay.showCost", String(v)); }
                catch (err) { console.warn("localStorage write failed:", err); }
              }}
              style={{ marginRight: 6 }}
            />
            Показывать индикатор стоимости 💰 в overlay-баре
          </label>
          <div style={{ fontSize: 11, color: "var(--c-text-mute)", marginLeft: 22 }}>
            Скрытие не отключает учёт — деньги всё равно пишутся в журнал и cost:update event летает. Только убирает шильдик из бара.
          </div>
        </div>
      </div>

      <div className="settings-section">
        <h3>🎯 Stealth</h3>
        <div className="field">
          <label title="WDA_EXCLUDEFROMCAPTURE — overlay+tiles не появятся в Zoom/Meet/OBS screen share">
            <input
              type="checkbox"
              checked={cfg.stealth_enabled}
              onChange={async (e) => {
                const v = e.target.checked;
                update({ stealth_enabled: v });
                try {
                  await invoke("set_stealth", { enabled: v });
                } catch (err) {
                  console.warn("set_stealth:", err);
                }
              }}
              style={{ marginRight: 6 }}
            />
            Скрыть overlay+tiles от screen share (нужен restart? нет — применяется сразу)
          </label>
        </div>
      </div>

      <div className="settings-section">
        <h3>🎯 Coaching</h3>
        <div className="field">
          <label title="После Stop session AI шлёт mic-транскрипт в Sonnet и возвращает 3 коротких замечания о вашей речи (темп, паразиты, структура). +1 Sonnet-вызов на сессию.">
            <input
              type="checkbox"
              checked={cfg.post_meeting_debrief_enabled ?? false}
              onChange={(e) => update({ post_meeting_debrief_enabled: e.target.checked })}
              style={{ marginRight: 6 }}
            />
            Post-meeting auto-debrief — coaching tile после Stop (opt-in)
          </label>
          <div style={{ fontSize: 11, color: "var(--c-text-dim)", marginTop: 4 }}>
            Срабатывает только если сессия ≥30 сек и было ≥5 mic-реплик. Стоит ~1 Sonnet вызов (≈$0.005). Не забудьте Save.
          </div>
        </div>
      </div>

      <div className="settings-section">
        <h3>🪟 Auto-tiles</h3>
        <div className="field">
          <label>
            <input
              type="checkbox"
              checked={cfg.auto_tiles_enabled}
              onChange={(e) => update({ auto_tiles_enabled: e.target.checked })}
              style={{ marginRight: 6 }}
            />
            Включить авто-окошки при вопросах в транскрипте
          </label>
        </div>
        <div className="field">
          <label>Монитор для tiles (по умолчанию: первый не-primary дисплей; если монитор один — primary)</label>
          <select
            value={cfg.tile_monitor_name ?? ""}
            onChange={(e) => update({ tile_monitor_name: e.target.value || null })}
          >
            <option value="">— авто (предпочитать не-primary) —</option>
            {monitors.map((m) => (
              <option key={m} value={m}>{m}</option>
            ))}
          </select>
        </div>
        <div className="field">
          <label>
            Trigger-keywords (через пробел, case-insensitive). Срабатывают как whole-word match.
          </label>
          <textarea
            style={{ minHeight: 60 }}
            value={cfg.trigger_keywords}
            onChange={(e) => update({ trigger_keywords: e.target.value })}
            placeholder="kubernetes etcd istio terraform prometheus"
          />
        </div>
      </div>

      <div className="settings-section">
        <h3>
          📚 Knowledge Base
          {kbStats && (
            <span style={{ marginLeft: 8, fontSize: 11, color: "var(--c-text-dim)", textTransform: "none", letterSpacing: 0 }}>
              · {kbStats.total} entries ({kbStats.glossary} glossary · {kbStats.commands} commands · {kbStats.patterns} patterns)
            </span>
          )}
        </h3>
        <div className="field">
          <label>Поиск по встроенной базе (термины + команды + паттерны). Хит → Open as tile.</label>
          <input
            type="text"
            value={kbQuery}
            onChange={(e) => setKbQuery(e.target.value)}
            placeholder="kubernetes / dijkstra / saga / iptables / consistent hashing …"
            spellCheck={false}
            autoComplete="off"
          />
        </div>
        {kbBusy && (
          <div style={{ fontSize: 11, color: "var(--c-text-dim)" }}>searching…</div>
        )}
        {!kbBusy && kbQuery && kbResults.length === 0 && (
          <div style={{ fontSize: 11, color: "var(--c-text-dim)" }}>
            no matches for «{kbQuery}»
          </div>
        )}
        {kbResults.length > 0 && (
          <div style={{ display: "flex", flexDirection: "column", gap: 4, maxHeight: 280, overflowY: "auto" }}>
            {kbResults.map((e, i) => (
              <div
                key={e.source + ":" + e.key + ":" + i}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                  padding: "4px 8px",
                  background: "var(--c-bg-2)",
                  border: "1px solid var(--c-border-soft)",
                  borderRadius: 4,
                  fontSize: 12,
                }}
              >
                <span
                  style={{
                    minWidth: 70,
                    fontSize: 10,
                    color: "var(--c-text-mute)",
                    textTransform: "uppercase",
                    letterSpacing: "0.04em",
                  }}
                  title={`source: ${e.source}`}
                >
                  {e.source}
                </span>
                <kbd style={{ minWidth: 80 }}>{e.key}</kbd>
                <span style={{ flex: 1, color: "var(--c-text)" }}>{e.heading}</span>
                <button
                  className="btn secondary"
                  style={{ height: 22, padding: "0 8px", fontSize: 11 }}
                  onClick={async () => {
                    try {
                      await invoke("kb_spawn", { key: e.key });
                      showToast("ok", `Открыт тайл «${e.heading}»`);
                    } catch (err) {
                      showToast("err", `kb_spawn failed: ${err}`);
                    }
                  }}
                  title={`Открыть тайл с записью «${e.heading}»`}
                >
                  Open →
                </button>
              </div>
            ))}
          </div>
        )}
        <div style={{ fontSize: 11, color: "var(--c-text-dim)", marginTop: 6 }}>
          KB файлы embedded в бинарник (read-only). Источники: <code>src-tauri/knowledge/{"{glossary,commands,patterns}"}.md</code>.
        </div>
      </div>

      <div className="settings-section">
        <h3>
          📋 Snippets ({(cfg.snippets || []).length}) — pre-written answers (zero cost){" "}
          <button
            className="btn secondary"
            style={{ height: 22, padding: "0 8px", fontSize: 11, marginLeft: 8 }}
            onClick={() => setSnippetsExpanded(v => !v)}
            title={snippetsExpanded ? "Свернуть" : "Развернуть все снипеты"}
          >{snippetsExpanded ? "▲ свернуть" : "▼ показать"}</button>
        </h3>
        {snippetsExpanded && (
          <div className="field">
            <label>
              Шаблонные ответы, разворачиваются мгновенно (без AI-вызова, $0). Нажми «Expand →» — карточка появится на tile-мониторе (см. секцию Auto-tiles).
            </label>
            <input
              type="text"
              value={snippetFilter}
              onChange={(e) => setSnippetFilter(e.target.value)}
              placeholder={`Фильтр (${(cfg.snippets || []).length} всего)…`}
              style={{ marginTop: 4 }}
            />
          </div>
        )}
        {!snippetsExpanded ? (
          <div style={{ fontSize: 11, color: "var(--c-text-dim)" }}>
            Свёрнуто чтобы Settings не превращался в портянку. Жми «показать» сверху · или используй F4 (KB palette) во время сессии — там же доступны.
          </div>
        ) : (cfg.snippets || []).length === 0 ? (
          <div style={{ fontSize: 12, color: "var(--c-text-mute)" }}>
            Нет снипетов. Конфиг сбросит дефолтные при следующем перезапуске.
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 6, maxHeight: 320, overflowY: "auto" }}>
            {cfg.snippets
              .filter(s => !snippetFilter.trim() || s.key.toLowerCase().includes(snippetFilter.toLowerCase()) || s.title.toLowerCase().includes(snippetFilter.toLowerCase()))
              .map((s, i) => (
              <div
                key={s.key + ":" + i}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                  padding: "6px 10px",
                  background: "var(--c-bg-2)",
                  border: "1px solid var(--c-border-soft)",
                  borderRadius: 6,
                }}
              >
                <kbd style={{ minWidth: 50, textAlign: "center" }}>/{s.key}</kbd>
                <span style={{ flex: 1, fontSize: 12 }}>{s.title}</span>
                <button
                  className="btn secondary"
                  style={{ height: 24, padding: "0 10px" }}
                  onClick={async () => {
                    try {
                      await invoke("expand_snippet", { key: s.key });
                      showToast("ok", `/${s.key} развёрнут как тайл`);
                    } catch (e) {
                      showToast("err", `Expand failed: ${e}`);
                    }
                  }}
                  title={`Открыть тайл со снипетом /${s.key}`}
                >
                  Expand →
                </button>
              </div>
            ))}
          </div>
        )}
        <div style={{ fontSize: 11, color: "var(--c-text-dim)", marginTop: 8 }}>
          Редактирование снипетов через JSON: <code>%APPDATA%\overlay-mvp\config.json</code> → массив <code>snippets</code>.
          В будущей версии — palette через F4 и UI редактор прямо здесь.
        </div>
      </div>

      <div className="settings-section">
        <h3>🎙 STT (Groq Whisper)</h3>
        <div className="field">
          <label>Groq API key (gsk_…)</label>
          <input
            type="password"
            value={cfg.groq_api_key}
            onChange={(e) => update({ groq_api_key: e.target.value })}
          />
        </div>
        <div className="field">
          <label>Language (empty = auto-detect)</label>
          <input
            type="text"
            value={cfg.stt_language ?? ""}
            onChange={(e) => update({ stt_language: e.target.value || null })}
            placeholder="ru, en, …"
          />
        </div>
      </div>

      <div className="settings-section">
        <h3>⌨ Hotkeys</h3>
        <div className="field">
          <label>Ask AI</label>
          <input value={cfg.hotkey_ask} onChange={(e) => update({ hotkey_ask: e.target.value })} />
        </div>
        <div className="field">
          <label>Take screenshot</label>
          <input value={cfg.hotkey_screenshot} onChange={(e) => update({ hotkey_screenshot: e.target.value })} />
        </div>
        <div className="field">
          <label>Toggle visibility</label>
          <input value={cfg.hotkey_toggle_visibility} onChange={(e) => update({ hotkey_toggle_visibility: e.target.value })} />
        </div>
        <div className="field">
          <label>Pause audio</label>
          <input value={cfg.hotkey_pause_audio} onChange={(e) => update({ hotkey_pause_audio: e.target.value })} />
        </div>
      </div>

      <div className="btn-row">
        {savedFlash && <span style={{ color: "#4ade80", alignSelf: "center" }}>✓ Saved</span>}
        <button
          className="btn secondary"
          onClick={() => {
            window.location.search = "?replay=1";
          }}
          title="In-app просмотрщик session journals — timeline transcript/AI/detector/tiles"
        >
          📊 Replay
        </button>
        <button
          className="btn secondary"
          onClick={() => invoke("open_sessions_folder").catch((e) => console.warn("open_sessions:", e))}
          title="JSONL логи всех transcript/AI/detector событий по сессиям"
        >
          📁 Логи сессий
        </button>
        <button
          className="btn secondary"
          onClick={async () => {
            try {
              const path = await invoke<string>("export_config");
              showToast("ok", `Конфиг сохранён: ${path}`);
            } catch (e) {
              showToast("err", `Ошибка экспорта: ${e}`);
            }
          }}
          title="Сохранить config + профили + ключи на Desktop"
        >
          💾 Export
        </button>
        <button
          className="btn secondary"
          onClick={async () => {
            const path = await showPrompt(
              "Импорт конфига",
              "C:\\Users\\you\\Desktop\\overlay-config.json",
            );
            if (!path) return;
            try {
              await invoke("import_config", { path });
              const fresh = await invoke<Config>("get_config");
              setCfg(fresh);
              showToast("ok", "Конфиг загружен. Перезапустите session чтобы применить.");
            } catch (e) {
              showToast("err", `Ошибка импорта: ${e}`);
            }
          }}
          title="Загрузить config из json файла"
        >
          📥 Import
        </button>
        <button className="btn secondary" onClick={back}>← Back to overlay</button>
        <button className="btn" onClick={async () => { await save(); }}>Save</button>
      </div>

      {toast && (
        <div
          className={`settings-toast settings-toast-${toast.kind}`}
          role={toast.kind === "err" ? "alert" : "status"}
          aria-live={toast.kind === "err" ? "assertive" : "polite"}
          key={toast.ts}
        >
          <span style={{ flex: 1 }}>{toast.text}</span>
          <button
            className="settings-toast-close"
            onClick={() => setToast(null)}
            aria-label="Закрыть"
            title="Закрыть"
          >×</button>
        </div>
      )}

      {modal && (
        <div
          className="settings-modal-backdrop"
          onMouseDown={(e) => {
            // Only treat as backdrop-click when the press starts on the
            // backdrop itself — avoids race where a button click bubbles
            // and triggers an unintended cancel near the modal edge.
            if (e.target !== e.currentTarget) return;
            if (modal.kind === "prompt") modal.onCancel();
            else modal.onNo();
          }}
        >
          <div
            className="settings-modal"
            onMouseDown={(e) => e.stopPropagation()}
            role="dialog"
            aria-modal="true"
          >
            <div className="settings-modal-title">{modal.title}</div>
            {modal.kind === "prompt" ? (
              <>
                <input
                  className="settings-modal-input"
                  type="text"
                  autoFocus
                  value={promptValue}
                  placeholder={modal.placeholder}
                  onChange={(e) => setPromptValue(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      // Mirror the OK-button gate — empty input shouldn't
                      // submit a blank profile name / config path.
                      if (promptValue.trim()) modal.onSubmit(promptValue);
                    } else if (e.key === "Escape") {
                      modal.onCancel();
                    }
                  }}
                />
                <div className="settings-modal-actions">
                  <button className="btn secondary" onClick={modal.onCancel}>Отмена</button>
                  <button
                    className="btn"
                    onClick={() => modal.onSubmit(promptValue)}
                    disabled={!promptValue.trim()}
                  >OK</button>
                </div>
              </>
            ) : (
              <div className="settings-modal-actions">
                <button className="btn secondary" autoFocus onClick={modal.onNo}>Отмена</button>
                <button className="btn settings-modal-danger" onClick={modal.onYes}>Удалить</button>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
