import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { t, resolveLang, type Lang } from "./i18n";

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
      // v0.0.31: contextual confirm button label + danger styling. User
      // reported the quit-app confirm said «Удалить» (hardcoded for the
      // delete-snippet flow) — confusing because the action was «Выйти».
      // Default label: «Подтвердить». Default danger: false (neutral primary).
      confirmLabel?: string;
      danger?: boolean;
      onYes: () => void;
      onNo: () => void;
    }
  | {
      // v0.0.10: 3-field snippet add/edit modal. `isNew=true` allows key
      // entry; for edit, key is locked (snippets keyed by it). `existingKeys`
      // is used for uniqueness validation on add.
      kind: "snippet";
      title: string;
      initial: { key: string; title: string; body: string };
      isNew: boolean;
      existingKeys: string[];
      onSubmit: (s: { key: string; title: string; body: string }) => void;
      onCancel: () => void;
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
  stt_model?: string;
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
  max_session_cost_usd?: number;
  detector_skip_mic?: boolean;
  auto_tile_every_line?: boolean;
  // v0.0.42: UI language for Settings/overlay chrome. Defaults to "ru" on
  // backend. Optional in TS because old configs from <v0.0.42 lack it.
  ui_language?: string;
  // v0.0.55: tile body font size in px (range 11-18, default 12).
  // Optional for backward compat with <v0.0.55 configs.
  tile_font_size?: number;
};

type BridgeStatus = {
  reachable: boolean;
  status: number;
  latency_ms: number;
  hint: string;
};

type UpdateInfo = {
  current: string;
  latest: string | null;
  update_available: boolean;
  download_url: string;
  notes: string;
  error: string;
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
  // v0.0.55: overlay compact mode — hides cost/wpm/screenshot chips in
  // overlay bar, leaving only status + dot + HUD + gear. Stored in
  // localStorage like showCost so it can be flipped without backend
  // round-trip. Same storage-event pattern; Overlay.tsx listens.
  const [overlayCompact, setOverlayCompact] = useState<boolean>(() => {
    try { return localStorage.getItem("overlay.compact") === "true"; }
    catch { return false; }
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
  // v0.0.10 snippet edit modal — 3 separate fields, reset when opened.
  const [snipKey, setSnipKey] = useState("");
  const [snipTitle, setSnipTitle] = useState("");
  const [snipBody, setSnipBody] = useState("");
  const [snipError, setSnipError] = useState("");
  // Snippets section state: collapsed by default + filter, otherwise 57
  // entries make the Settings page scroll forever (live regression
  // 2026-05-25: "snippet бесконечно длинный список").
  const [snippetsExpanded, setSnippetsExpanded] = useState(false);
  const [snippetFilter, setSnippetFilter] = useState("");
  // Bridge probe — tests ai_base_url + ai_bearer with a cheap 1-token POST.
  const [bridgeStatus, setBridgeStatus] = useState<BridgeStatus | null>(null);
  const [bridgeBusy, setBridgeBusy] = useState(false);
  // Update check — GH releases API for "is there a new version".
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateBusy, setUpdateBusy] = useState(false);
  // v0.0.23: one-click update download + install (the second button).
  // Separate flag so the user can still hit "Проверить" while a
  // download is in flight (though that's unusual).
  const [oneClickBusy, setOneClickBusy] = useState(false);
  // Crash report — if %APPDATA%\overlay-mvp\crash-report.txt exists, show
  // a button to open it. Probed on mount + on window focus.
  const [crashReport, setCrashReport] = useState<string | null>(null);
  // v0.0.30: sidebar redesign — active panel + search query for filter.
  // Implementation per Claude Design handoff (rust-overlay/project/Settings.html).
  // Wraps the existing 13 settings-section blocks in conditional render
  // instead of moving them — preserves all save/load field bindings.
  const [activeSection, setActiveSection] = useState<string>("profile");
  const [navFilter, setNavFilter] = useState("");
  // v0.0.42: i18n. Resolve the UI language from cfg on every render.
  // Defaults to "ru" when cfg is null (initial paint before load_config
  // completes) and for any value other than the explicit "en". This is
  // intentionally a derived value not its own state — the source of truth
  // is the persisted config, and we want a single re-render when it loads.
  const lang: Lang = resolveLang(cfg?.ui_language);
  // v0.0.36 (agent P1): track the 2-sec setTimeout that fires quit_app
  // after a successful download_and_install_update spawn. Without this
  // ref, if Settings unmounts (e.g. user clicks Back to overlay), the
  // timer still fires and kills the app while the user is back on the
  // bar. Now we clear it in the unmount cleanup below.
  const quitAfterDownloadTimerRef = useRef<number | null>(null);
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
      // v0.0.36 (agent P1): cancel the 2-sec quit-after-download timer
      // if Settings unmounts before it fires (e.g. user clicked Back to
      // overlay during the 2-sec UAC wait). Otherwise the app would
      // quit while the user is back on the bar.
      if (quitAfterDownloadTimerRef.current) {
        window.clearTimeout(quitAfterDownloadTimerRef.current);
        quitAfterDownloadTimerRef.current = null;
      }
      // Resolve any pending modal so awaiting callers don't hang.
      if (pendingModalRejectRef.current) {
        pendingModalRejectRef.current();
        pendingModalRejectRef.current = null;
      }
    };
  }, []);

  // v0.0.17: drag-and-drop a .json file onto Settings to import it.
  // Tauri's onDragDropEvent fires at the window level with the file paths.
  // Same backend path as the explicit import button — invoke("import_config").
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    (async () => {
      try {
        const u = await getCurrentWindow().onDragDropEvent(({ payload }) => {
          if (!mountedRef.current) return;
          if (payload.type !== "drop") return;
          const paths = (payload as { type: "drop"; paths: string[] }).paths || [];
          const json = paths.find((p) => p.toLowerCase().endsWith(".json"));
          if (!json) {
            if (paths.length > 0) {
              showToast("err", t("settings.dnd.import.bad", lang).replace("{ext}", paths[0].split(/[\\/]/).pop() ?? ""));
            }
            return;
          }
          (async () => {
            try {
              await invoke("import_config", { path: json });
              const fresh = await invoke<Config>("get_config");
              if (mountedRef.current) {
                setCfg(fresh);
                showToast("ok", t("settings.dnd.import.ok", lang));
              }
            } catch (e) {
              if (mountedRef.current) showToast("err", t("settings.import.error", lang).replace("{err}", String(e)));
            }
          })();
        });
        if (cancelled) u();
        else unlisten = u;
      } catch (e) {
        console.warn("onDragDropEvent register failed:", e);
      }
    })();
    return () => {
      cancelled = true;
      if (unlisten) unlisten();
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
  const showConfirm = useCallback((
    title: string,
    opts?: { confirmLabel?: string; danger?: boolean },
  ) =>
    new Promise<boolean>((resolve) => {
      if (!mountedRef.current) { resolve(false); return; }
      pendingModalRejectRef.current = () => resolve(false);
      setModal({
        kind: "confirm",
        title,
        confirmLabel: opts?.confirmLabel,
        danger: opts?.danger,
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
  // v0.0.10: open the 3-field snippet add/edit modal. For "+ New" pass
  // empty initial + isNew=true; for "✎ Edit" pass existing snippet + false.
  const showSnippetEdit = useCallback((
    title: string,
    initial: { key: string; title: string; body: string },
    isNew: boolean,
    existingKeys: string[],
  ) =>
    new Promise<{ key: string; title: string; body: string } | null>((resolve) => {
      if (!mountedRef.current) { resolve(null); return; }
      pendingModalRejectRef.current = () => resolve(null);
      setSnipKey(initial.key);
      setSnipTitle(initial.title);
      setSnipBody(initial.body);
      setSnipError("");
      setModal({
        kind: "snippet",
        title,
        initial,
        isNew,
        existingKeys,
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

  // Esc-anywhere-to-cancel for confirm + snippet modals (prompt input already handles it).
  useEffect(() => {
    if (!modal || modal.kind === "prompt") return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (modal.kind === "confirm") modal.onNo();
        else if (modal.kind === "snippet") modal.onCancel();
      }
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
      // Crash report probe: backend returns path if file exists, else null.
      invoke<string | null>("crash_report_path")
        .then((p) => { if (mountedRef.current) setCrashReport(p); })
        .catch((e) => console.warn("crash_report_path:", e));
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
      setRecError(t("meeting.error.empty", lang));
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
        title={t("settings.drag.tip", lang)}
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
          ⋮⋮  {t("settings.title", lang)}
        </h2>
        <button
          className="btn settings-modal-danger"
          style={{ height: 28, padding: "0 12px", fontSize: 12 }}
          onClick={async () => {
            const ok = await showConfirm(
              t("settings.quit.confirm", lang),
              { confirmLabel: t("settings.quit.confirm.label", lang), danger: true },
            );
            if (ok) {
              try { await invoke("quit_app"); }
              catch (e) { showToast("err", `${t("settings.quit.failed", lang)}: ${e}`); }
            }
          }}
          title={t("settings.quit.tip", lang)}
        >
          {t("settings.quit", lang)}
        </button>
      </div>

      {/* v0.0.30 sidebar layout per Claude Design handoff.
       * The 13 existing settings-section blocks are wrapped in conditionals
       * driven by `activeSection`. Sidebar nav lives on the left. The old
       * btn-row footer (Save/Back/Replay/etc) stays below the shell. */}
      <div className="settings-shell">
        <nav className="settings-nav" aria-label={t("nav.aria.settings", lang)}>
          <div className="nav-search">
            <input
              type="search"
              placeholder={t("nav.filter.placeholder", lang)}
              value={navFilter}
              onChange={(e) => setNavFilter(e.target.value)}
              aria-label={t("nav.filter.aria", lang)}
            />
          </div>
          {(() => {
            const items: Array<
              | { group: string }
              | { id: string; icon: string; label: string; badge?: string; warn?: boolean }
            > = [
              { group: t("nav.group.session", lang) },
              { id: "profile", icon: "👤", label: t("nav.profile", lang) },
              { id: "audio", icon: "🎚", label: t("nav.audio", lang) },
              { group: t("nav.group.ai", lang) },
              {
                id: "ai",
                icon: "🛰",
                label: t("nav.ai", lang),
                ...(cfg && cfg.ai_base_url && cfg.ai_base_url.startsWith("http://") &&
                  !cfg.ai_base_url.includes("localhost") &&
                  !cfg.ai_base_url.includes("127.0.0.1") &&
                  !cfg.ai_base_url.includes("[::1]")
                  ? { warn: true, badge: "HTTP" }
                  : {}),
              },
              { group: t("nav.group.logic", lang) },
              { id: "tiles", icon: "🪟", label: t("nav.tiles", lang),
                ...(cfg?.snippets?.length ? { badge: String(cfg.snippets.length) } : {}) },
              { id: "knowledge", icon: "📚", label: t("nav.knowledge", lang),
                ...(kbStats?.total ? { badge: kbStats.total >= 1000
                  ? `${(kbStats.total / 1000).toFixed(1)}k`
                  : String(kbStats.total) } : {}) },
              { id: "coaching", icon: "🎓", label: t("nav.coaching", lang) },
              { group: t("nav.group.app", lang) },
              { id: "interface", icon: "🎨", label: t("nav.interface", lang) },
              { id: "stealth", icon: "🫥", label: t("nav.stealth", lang) },
              { id: "hotkeys", icon: "⌨", label: t("nav.hotkeys", lang) },
              { id: "advanced", icon: "🔧", label: t("nav.advanced", lang) },
            ];
            const q = navFilter.trim().toLowerCase();
            const filtered = q
              ? items.filter((it) =>
                  "group" in it ? false : it.label.toLowerCase().includes(q) || it.id.includes(q),
                )
              : items;
            // v0.0.36 (agent P1): identify the LAST group in the filtered
            // list so we can give it an explicit `.nav-group-pinned`
            // class. Was using CSS `:nth-last-of-type(1)` which is
            // brittle — any future div added after the group breaks
            // the bottom-pinned layout. Now: explicit class.
            const lastGroupIdx = (() => {
              for (let i = filtered.length - 1; i >= 0; i--) {
                if ("group" in filtered[i]) return i;
              }
              return -1;
            })();
            return filtered.map((it, i) =>
              "group" in it ? (
                <div
                  key={`g${i}`}
                  className={"nav-group" + (i === lastGroupIdx ? " nav-group-pinned" : "")}
                >
                  {it.group}
                </div>
              ) : (
                <button
                  key={it.id}
                  type="button"
                  className={
                    "nav-item" +
                    (activeSection === it.id ? " active" : "") +
                    (it.warn ? " has-warn" : "")
                  }
                  onClick={() => setActiveSection(it.id)}
                  aria-current={activeSection === it.id ? "page" : undefined}
                >
                  <span className="nav-icon">{it.icon}</span>
                  <span>{it.label}</span>
                  {it.badge && <span className="nav-badge">{it.badge}</span>}
                </button>
              ),
            );
          })()}
        </nav>

        <section className="settings-pane" aria-label={t("nav.aria.pane", lang)}>

      {activeSection === "profile" && (<div className="settings-section">
        <h3>{t("profile.profiles.title", lang)}</h3>
        <div className="field">
          <label>{t("profile.active.label", lang)}</label>
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
            <option value="">{t("profile.none", lang)}</option>
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
                t("profile.prompt.name", lang),
                t("profile.prompt.placeholder", lang),
              );
              const name = raw?.trim();
              if (!name) return;
              const profiles = [
                ...cfg.context_profiles.filter((p) => p.name !== name),
                { name, context: cfg.meeting_context },
              ];
              update({ context_profiles: profiles, active_profile: name });
              showToast("ok", t("profile.saved.toast", lang).replace("{name}", name));
            }}
          >{t("profile.save.button", lang)}</button>
          {cfg.active_profile && (
            <button
              className="btn secondary"
              onClick={async () => {
                const ok = await showConfirm(
                  t("profile.delete.confirm", lang).replace("{name}", cfg.active_profile ?? ""),
                  { confirmLabel: t("common.delete", lang), danger: true },
                );
                if (!ok) return;
                const removed = cfg.active_profile;
                const profiles = cfg.context_profiles.filter((p) => p.name !== cfg.active_profile);
                update({ context_profiles: profiles, active_profile: null });
                showToast("ok", t("profile.deleted.toast", lang).replace("{name}", removed ?? ""));
              }}
            >{t("profile.delete.button", lang)}</button>
          )}
        </div>
      </div>)}

      {activeSection === "profile" && (<div className="settings-section">
        <h3>{t("meeting.title", lang)}</h3>
        <div className="field">
          <label>{t("meeting.label", lang)}</label>
          <textarea
            value={cfg.meeting_context}
            onChange={(e) => update({ meeting_context: e.target.value })}
            placeholder={t("meeting.placeholder", lang)}
          />
        </div>
        <div className="btn-row" style={{ justifyContent: "flex-start", gap: 8 }}>
          <button
            className="btn secondary"
            onClick={recordPrep}
            disabled={recState !== "idle"}
            title={t("meeting.record.tip", lang)}
          >
            {recState === "recording"
              ? t("meeting.record.busy", lang).replace("{sec}", String(recCountdown))
              : t("meeting.record.button", lang).replace("{sec}", String(RECORD_SECONDS))}
          </button>
          <button
            className="btn secondary"
            onClick={structurePrep}
            disabled={recState !== "idle"}
            title={t("meeting.structure.tip", lang).replace("{model}", cfg.prep_model)}
          >
            {recState === "structuring"
              ? t("meeting.structure.busy", lang)
              : t("meeting.structure.button", lang).replace("{model}", cfg.prep_model)}
          </button>
        </div>
        {recError && (
          <div style={{ color: "#ef4444", fontSize: 11, marginTop: 6 }}>
            {recError}
          </div>
        )}
      </div>)}

      {activeSection === "audio" && (<div className="settings-section">
        <h3>{t("audio.devices.title", lang)}</h3>
        <div className="field">
          <label>{t("audio.mic.label", lang)}</label>
          <select
            value={cfg.mic_device ?? ""}
            onChange={(e) => update({ mic_device: e.target.value || null })}
          >
            <option value="">{t("audio.mic.default", lang)}</option>
            {devices.inputs.map((d) => (
              <option key={d} value={d}>{d}</option>
            ))}
          </select>
        </div>
        <div className="field">
          <label>{t("audio.sys.label", lang)}</label>
          <select
            value={cfg.system_audio_device ?? ""}
            onChange={(e) => update({ system_audio_device: e.target.value || null })}
          >
            <option value="">{t("audio.sys.default", lang)}</option>
            {devices.inputs.map((d) => (
              <option key={"in:" + d} value={d}>{d}</option>
            ))}
            {devices.outputs.map((d) => (
              <option key={"out:" + d} value={d}>{d} {t("audio.loopback.suffix", lang)}</option>
            ))}
          </select>
        </div>
      </div>)}

      {activeSection === "ai" && (<div className="settings-section">
        {/* v0.0.40 polish — AI panel split into 4 logical .card sub-
            sections (was a wall of 9 .field blocks). Same hooks, same
            backend, same state. Pure structural conversion. */}

        {/* ─ 🛰 Bridge endpoint ─────────────────────────────────── */}
        <div className="card">
          <div className="card-title">{t("ai.bridge.title", lang)}</div>
          <div className="card-row">
            <div className="row-label">
              {t("ai.bridge.url.label", lang)}
              <span className="row-hint">{t("ai.bridge.url.hint", lang)}</span>
            </div>
            <div className="row-control">
              <input
                type="text"
                value={cfg.ai_base_url}
                onChange={(e) => update({ ai_base_url: e.target.value })}
                placeholder="http://192.168.0.142:18902/v1"
              />
              {(() => {
                const url = cfg.ai_base_url.trim().toLowerCase();
                if (!url.startsWith("http://")) return null;
                const host = url.slice("http://".length).split("/")[0].split(":")[0];
                const isLoopback = host === "127.0.0.1" || host === "localhost" || host === "[::1]" || host === "::1";
                if (isLoopback) return null;
                return (
                  <div className="banner warn">
                    {t("ai.bridge.warn.http", lang)} ({host})
                  </div>
                );
              })()}
            </div>
          </div>
          <div className="card-row">
            <div className="row-label">
              {t("ai.bridge.bearer.label", lang)}
              <span className="row-hint">{t("ai.bridge.bearer.hint", lang)}</span>
            </div>
            <div className="row-control">
              <input
                type="password"
                value={cfg.ai_bearer}
                onChange={(e) => update({ ai_bearer: e.target.value })}
              />
            </div>
          </div>
          <div className="card-row">
            <div className="row-label">
              {t("ai.bridge.health.label", lang)}
              <span className="row-hint">{t("ai.bridge.health.hint", lang)}</span>
            </div>
            <div className="row-control">
              <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
                <button
                  className="btn secondary"
                  disabled={bridgeBusy || !cfg.ai_base_url.trim() || !cfg.ai_bearer.trim()}
                  onClick={async () => {
                    setBridgeBusy(true);
                    setBridgeStatus(null);
                    try {
                      const s = await invoke<BridgeStatus>("check_bridge", {
                        baseUrl: cfg.ai_base_url,
                        bearer: cfg.ai_bearer,
                        model: cfg.ai_model || null,
                      });
                      setBridgeStatus(s);
                    } catch (e) {
                      setBridgeStatus({ reachable: false, status: 0, latency_ms: 0, hint: `${e}` });
                    } finally {
                      setBridgeBusy(false);
                    }
                  }}
                  title={t("ai.bridge.check.tip", lang)}
                >
                  {bridgeBusy ? t("ai.bridge.check.busy", lang) : t("ai.bridge.check.button", lang)}
                </button>
                {bridgeStatus && (
                  <span
                    style={{
                      fontSize: 12,
                      color: bridgeStatus.reachable ? "var(--c-mic, #4ade80)" : "var(--c-error, #f87171)",
                    }}
                  >
                    {bridgeStatus.reachable ? "🟢" : "🔴"}{" "}
                    {bridgeStatus.reachable
                      ? `OK (HTTP ${bridgeStatus.status}, ${bridgeStatus.latency_ms}ms)`
                      : bridgeStatus.hint}
                  </span>
                )}
              </div>
              {bridgeStatus && !bridgeStatus.reachable && bridgeStatus.hint && (
                <div className="hint">{t("ai.bridge.fail.tip", lang)}</div>
              )}
            </div>
          </div>
        </div>

        {/* ─ 🧠 Models ─────────────────────────────────────────── */}
        <div className="card">
          <div className="card-title">{t("ai.models.title", lang)}</div>
          <div className="card-row">
            <div className="row-label">
              {t("ai.models.live.label", lang)}
              <span className="row-hint">{t("ai.models.live.hint", lang)}</span>
            </div>
            <div className="row-control">
              <select
                value={cfg.ai_model}
                onChange={(e) => update({ ai_model: e.target.value })}
              >
                <option value="claude-haiku-4-5">claude-haiku-4-5 {lang === "en" ? "(fast, default)" : "(быстро, default)"}</option>
                <option value="claude-sonnet-4-6">claude-sonnet-4-6 {lang === "en" ? "(smarter, slower)" : "(умнее, медленнее)"}</option>
                <option value="claude-opus-4-7">claude-opus-4-7 {lang === "en" ? "(smartest, slow)" : "(самый умный, медленный)"}</option>
              </select>
            </div>
          </div>
          <div className="card-row">
            <div className="row-label">
              {t("ai.models.prep.label", lang)}
              <span className="row-hint">{t("ai.models.prep.hint", lang)}</span>
            </div>
            <div className="row-control">
              <select
                value={cfg.prep_model}
                onChange={(e) => update({ prep_model: e.target.value })}
              >
                <option value="claude-sonnet-4-6">claude-sonnet-4-6 {lang === "en" ? "(default, 30-50% faster than 4-5)" : "(default, 30-50% быстрее 4-5)"}</option>
                <option value="claude-sonnet-4-5">claude-sonnet-4-5 {lang === "en" ? "(older, still works)" : "(старая, ещё работает)"}</option>
                <option value="claude-haiku-4-5">claude-haiku-4-5 {lang === "en" ? "(fast)" : "(быстро)"}</option>
                <option value="claude-opus-4-7">claude-opus-4-7 {lang === "en" ? "(max quality)" : "(максимум качества)"}</option>
              </select>
            </div>
          </div>
          <div className="card-row">
            <div className="row-label">
              {t("ai.models.lang.label", lang)}
              <span className="row-hint">{t("ai.models.lang.hint", lang)}</span>
            </div>
            <div className="row-control">
              <select
                value={cfg.response_language}
                onChange={(e) => update({ response_language: e.target.value })}
              >
                <option value="ru">{lang === "en" ? "Russian (ru)" : "Русский (ru)"}</option>
                <option value="en">English (en)</option>
              </select>
            </div>
          </div>
        </div>

        {/* ─ 💰 Budget ─────────────────────────────────────────── */}
        <div className="card">
          <div className="card-title">{t("ai.budget.title", lang)}</div>
          <div className="card-row">
            <div className="row-label">
              {t("ai.budget.cap.label", lang)}
              <span className="row-hint">{t("ai.budget.cap.hint", lang)}</span>
            </div>
            <div className="row-control">
              <input
                type="number"
                min={0}
                step={0.10}
                value={cfg.max_session_cost_usd ?? 0.0}
                onChange={(e) => {
                  const v = parseFloat(e.target.value);
                  if (Number.isFinite(v) && v >= 0) {
                    update({ max_session_cost_usd: v });
                  }
                }}
                style={{ width: 120 }}
              />
              <div className="hint">{t("ai.budget.note", lang)}</div>
            </div>
          </div>
        </div>

        {/* ─ 🎯 Detector ───────────────────────────────────────── */}
        <div className="card">
          <div className="card-title">{t("ai.det.title", lang)}</div>
          <div className="switch-row">
            <div className="switch-meta">
              <div className="switch-title">{t("ai.det.skip.title", lang)}</div>
              <div className="switch-desc">{t("ai.det.skip.desc", lang)}</div>
            </div>
            <button
              type="button"
              className="switch"
              role="switch"
              aria-checked={cfg.detector_skip_mic ?? true}
              aria-label={t("ai.det.skip.aria", lang)}
              onClick={() => update({ detector_skip_mic: !(cfg.detector_skip_mic ?? true) })}
            />
          </div>
          <div className="switch-row">
            <div className="switch-meta">
              <div className="switch-title">{t("ai.det.agg.title", lang)}</div>
              <div className="switch-desc">{t("ai.det.agg.desc", lang)}</div>
            </div>
            <button
              type="button"
              className="switch"
              role="switch"
              aria-checked={cfg.auto_tile_every_line ?? false}
              aria-label={t("ai.det.agg.aria", lang)}
              onClick={() => update({ auto_tile_every_line: !(cfg.auto_tile_every_line ?? false) })}
            />
          </div>
        </div>
      </div>)}

      {activeSection === "interface" && (<div className="settings-section">
        {/* v0.0.42 i18n: language picker card. Two-pill toggle (RU/EN).
            Persists via standard save() flow — value is on cfg.ui_language,
            picked up by next mount (and the current mount via the `lang`
            derivation above). */}
        <div className="card">
          <div className="card-title">{t("interface.language.title", lang)}</div>
          <div className="card-row" style={{ flexDirection: "column", alignItems: "stretch", gap: 8 }}>
            <div className="row-hint">{t("interface.language.desc", lang)}</div>
            <div style={{ display: "flex", gap: 8 }}>
              <button
                type="button"
                className={"btn" + (lang === "ru" ? "" : " secondary")}
                style={{ flex: 1 }}
                onClick={() => update({ ui_language: "ru" })}
                aria-pressed={lang === "ru"}
              >
                🇷🇺 {t("interface.language.ru", lang)}
              </button>
              <button
                type="button"
                className={"btn" + (lang === "en" ? "" : " secondary")}
                style={{ flex: 1 }}
                onClick={() => update({ ui_language: "en" })}
                aria-pressed={lang === "en"}
              >
                🇬🇧 {t("interface.language.en", lang)}
              </button>
            </div>
          </div>
        </div>

        {/* v0.0.38 polish — same template as Stealth/Coaching panels.
            v0.0.43 i18n: all strings via t(). */}
        <div className="card">
          <div className="card-title">{t("interface.cost.title", lang)}</div>
          <div className="switch-row">
            <div className="switch-meta">
              <div className="switch-title">{t("interface.cost.switch.title", lang)}</div>
              <div className="switch-desc">{t("interface.cost.switch.desc", lang)}</div>
            </div>
            <button
              type="button"
              className="switch"
              role="switch"
              aria-checked={showCost}
              aria-label={t("interface.cost.switch.aria", lang)}
              onClick={() => {
                const v = !showCost;
                setShowCost(v);
                try { localStorage.setItem("overlay.showCost", String(v)); }
                catch (err) { console.warn("localStorage write failed:", err); }
              }}
            />
          </div>
        </div>

        {/* v0.0.55: compact overlay mode + tile font size. */}
        <div className="card">
          <div className="card-title">{t("interface.compact.title", lang)}</div>
          <div className="switch-row">
            <div className="switch-meta">
              <div className="switch-title">{t("interface.compact.switch.title", lang)}</div>
              <div className="switch-desc">{t("interface.compact.switch.desc", lang)}</div>
            </div>
            <button
              type="button"
              className="switch"
              role="switch"
              aria-checked={overlayCompact}
              aria-label={t("interface.compact.switch.aria", lang)}
              onClick={() => {
                const v = !overlayCompact;
                setOverlayCompact(v);
                try { localStorage.setItem("overlay.compact", String(v)); }
                catch (err) { console.warn("localStorage write failed:", err); }
              }}
            />
          </div>
        </div>

        <div className="card">
          <div className="card-title">{t("interface.tilefs.title", lang)}</div>
          <div className="card-row">
            <div className="row-label">
              {t("interface.tilefs.label", lang)}
              <span className="row-hint">{t("interface.tilefs.hint", lang)}</span>
            </div>
            <div className="row-control" style={{ display: "flex", alignItems: "center", gap: 10 }}>
              <input
                type="range"
                min={11}
                max={18}
                step={1}
                value={cfg.tile_font_size ?? 12}
                onChange={(e) => update({ tile_font_size: parseInt(e.target.value, 10) || 12 })}
                style={{ flex: 1 }}
              />
              <span style={{ minWidth: 36, fontFamily: "monospace", fontSize: 12 }}>
                {cfg.tile_font_size ?? 12} px
              </span>
            </div>
          </div>
        </div>
      </div>)}

      {activeSection === "stealth" && (<div className="settings-section">
        {/* v0.0.37 polish: converted from legacy .field+checkbox to the
            design's .card + .switch-row + .switch + .banner.info. Template
            for the per-panel rollout — see docs/SETTINGS_POLISH_PLAN.md.
            Behavior unchanged: same `cfg.stealth_enabled` + `set_stealth`
            backend invoke; just visual conversion. */}
        <div className="card">
          <div className="card-title">{t("stealth.card.title", lang)}</div>
          <div className="switch-row">
            <div className="switch-meta">
              <div className="switch-title">{t("stealth.switch.title", lang)}</div>
              <div className="switch-desc">{t("stealth.switch.desc", lang)}</div>
            </div>
            <button
              type="button"
              className="switch"
              role="switch"
              aria-checked={cfg.stealth_enabled}
              aria-label={t("stealth.switch.aria", lang)}
              onClick={async () => {
                const v = !cfg.stealth_enabled;
                update({ stealth_enabled: v });
                try {
                  await invoke("set_stealth", { enabled: v });
                } catch (err) {
                  console.warn("set_stealth:", err);
                }
              }}
            />
          </div>
          <div className="banner info">{t("stealth.banner", lang)}</div>
        </div>
      </div>)}

      {activeSection === "coaching" && (<div className="settings-section">
        {/* v0.0.38 polish — same template as Stealth panel (v0.0.37):
            .card + .card-title + .switch-row. Behavior unchanged. */}
        <div className="card">
          <div className="card-title">{t("coaching.card.title", lang)}</div>
          <div className="switch-row">
            <div className="switch-meta">
              <div className="switch-title">{t("coaching.switch.title", lang)}</div>
              <div className="switch-desc">{t("coaching.switch.desc", lang)}</div>
            </div>
            <button
              type="button"
              className="switch"
              role="switch"
              aria-checked={cfg.post_meeting_debrief_enabled ?? false}
              aria-label={t("coaching.switch.aria", lang)}
              onClick={() => update({ post_meeting_debrief_enabled: !(cfg.post_meeting_debrief_enabled ?? false) })}
            />
          </div>
        </div>
      </div>)}

      {activeSection === "tiles" && (<div className="settings-section">
        {/* v0.0.39 polish: Auto-tiles section converted to .card +
            .switch-row (for the boolean) + .card-row (for monitor select
            and trigger-keywords textarea). Snippets section below stays
            unchanged for now — too much state, deferred. */}
        <div className="card">
          <div className="card-title">{t("tiles.auto.title", lang)}</div>
          <div className="switch-row">
            <div className="switch-meta">
              <div className="switch-title">{t("tiles.auto.switch.title", lang)}</div>
              <div className="switch-desc">{t("tiles.auto.switch.desc", lang)}</div>
            </div>
            <button
              type="button"
              className="switch"
              role="switch"
              aria-checked={cfg.auto_tiles_enabled}
              aria-label={t("tiles.auto.switch.aria", lang)}
              onClick={() => update({ auto_tiles_enabled: !cfg.auto_tiles_enabled })}
            />
          </div>
          <div className="card-row">
            <div className="row-label">
              {t("tiles.monitor.label", lang)}
              <span className="row-hint">{t("tiles.monitor.hint", lang)}</span>
            </div>
            <div className="row-control">
              <select
                value={cfg.tile_monitor_name ?? ""}
                onChange={(e) => update({ tile_monitor_name: e.target.value || null })}
              >
                <option value="">{t("tiles.monitor.auto", lang)}</option>
                {monitors.map((m) => (
                  <option key={m} value={m}>{m}</option>
                ))}
              </select>
            </div>
          </div>
          <div className="card-row">
            <div className="row-label">
              {t("tiles.keywords.label", lang)}
              <span className="row-hint">{t("tiles.keywords.hint", lang)}</span>
            </div>
            <div className="row-control">
              <textarea
                style={{ minHeight: 60 }}
                value={cfg.trigger_keywords}
                onChange={(e) => update({ trigger_keywords: e.target.value })}
                placeholder="kubernetes etcd istio terraform prometheus"
              />
            </div>
          </div>
        </div>
      </div>)}

      {activeSection === "knowledge" && (<div className="settings-section">
        <h3>
          {t("kb.title", lang)}
          {kbStats && (
            <span style={{ marginLeft: 8, fontSize: 11, color: "var(--c-text-dim)", textTransform: "none", letterSpacing: 0 }}>
              · {t("kb.stats", lang)
                  .replace("{total}", String(kbStats.total))
                  .replace("{glossary}", String(kbStats.glossary))
                  .replace("{commands}", String(kbStats.commands))
                  .replace("{patterns}", String(kbStats.patterns))}
            </span>
          )}
        </h3>
        <div className="field">
          <label>{t("kb.search.label", lang)}</label>
          <input
            type="text"
            value={kbQuery}
            onChange={(e) => setKbQuery(e.target.value)}
            placeholder={t("kb.search.placeholder", lang)}
            spellCheck={false}
            autoComplete="off"
          />
        </div>
        {kbBusy && (
          <div style={{ fontSize: 11, color: "var(--c-text-dim)" }}>{t("kb.searching", lang)}</div>
        )}
        {!kbBusy && kbQuery && kbResults.length === 0 && (
          <div style={{ fontSize: 11, color: "var(--c-text-dim)" }}>
            {t("kb.no.match", lang).replace("{q}", kbQuery)}
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
                  title={t("kb.source.aria", lang).replace("{s}", e.source)}
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
                      showToast("ok", t("kb.opened.toast", lang).replace("{h}", e.heading));
                    } catch (err) {
                      showToast("err", `${t("kb.spawn.fail.toast", lang)}: ${err}`);
                    }
                  }}
                  title={t("kb.open.tip", lang).replace("{h}", e.heading)}
                >
                  {t("kb.open.button", lang)}
                </button>
              </div>
            ))}
          </div>
        )}
        <div style={{ fontSize: 11, color: "var(--c-text-dim)", marginTop: 6 }}>
          {t("kb.note", lang)}
        </div>
      </div>)}

      {activeSection === "tiles" && (<div className="settings-section">
        <h3>
          {t("snippets.title", lang).replace("{n}", String((cfg.snippets || []).length))}{" "}
          <button
            className="btn secondary"
            style={{ height: 22, padding: "0 8px", fontSize: 11, marginLeft: 8 }}
            onClick={() => setSnippetsExpanded(v => !v)}
            title={snippetsExpanded ? t("snippets.collapse.tip", lang) : t("snippets.expand.tip", lang)}
          >{snippetsExpanded ? t("snippets.collapse.button", lang) : t("snippets.expand.button", lang)}</button>
          <button
            className="btn"
            style={{ height: 22, padding: "0 10px", fontSize: 11, marginLeft: 8 }}
            onClick={async () => {
              const newSnip = await showSnippetEdit(
                t("snippets.new.title", lang),
                { key: "", title: "", body: "" },
                true,
                (cfg.snippets || []).map(s => s.key),
              );
              if (!newSnip) return;
              const next = { ...cfg, snippets: [...(cfg.snippets || []), newSnip] };
              setCfg(next);
              try {
                await invoke("save_config", { newCfg: next });
                showToast("ok", t("snippets.create.toast.ok", lang).replace("{key}", newSnip.key).replace("{n}", String(next.snippets.length)));
                if (!snippetsExpanded) setSnippetsExpanded(true);
              } catch (e) {
                showToast("err", t("snippets.create.toast.fail", lang).replace("{err}", String(e)));
                setCfg(cfg);
              }
            }}
            title={t("snippets.new.tip", lang)}
          >{t("snippets.new.button", lang)}</button>
        </h3>
        {snippetsExpanded && (
          <div className="field">
            <label>{t("snippets.desc", lang)}</label>
            <input
              type="text"
              value={snippetFilter}
              onChange={(e) => setSnippetFilter(e.target.value)}
              placeholder={t("snippets.filter.placeholder", lang).replace("{n}", String((cfg.snippets || []).length))}
              style={{ marginTop: 4 }}
            />
          </div>
        )}
        {!snippetsExpanded ? (
          <div style={{ fontSize: 11, color: "var(--c-text-dim)" }}>
            {t("snippets.collapsed.hint", lang)}
          </div>
        ) : (cfg.snippets || []).length === 0 ? (
          <div style={{ fontSize: 12, color: "var(--c-text-mute)" }}>
            {lang === "en" ? "No snippets. Config will reset to defaults on next restart." : "Нет снипетов. Конфиг сбросит дефолтные при следующем перезапуске."}
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 6, maxHeight: 320, overflowY: "auto" }}>
            {cfg.snippets
              .filter(s => {
                if (!snippetFilter.trim()) return true;
                const f = snippetFilter.toLowerCase();
                return (
                  s.key.toLowerCase().includes(f) ||
                  s.title.toLowerCase().includes(f) ||
                  s.body.toLowerCase().includes(f) // v0.0.7: search body too
                );
              })
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
                      showToast("ok", t("snip.expand.toast.ok", lang).replace("{key}", s.key));
                    } catch (e) {
                      showToast("err", t("snip.expand.toast.fail", lang).replace("{err}", String(e)));
                    }
                  }}
                  title={t("snip.expand.tip", lang).replace("{key}", s.key)}
                >
                  {t("snip.expand.button", lang)}
                </button>
                <button
                  className="btn secondary"
                  style={{ height: 24, padding: "0 8px", fontSize: 12 }}
                  onClick={async () => {
                    const edited = await showSnippetEdit(
                      t("snip.edit.modal.title", lang).replace("{key}", s.key),
                      { key: s.key, title: s.title, body: s.body },
                      false,
                      [],
                    );
                    if (!edited) return;
                    // Key is locked when editing, so just replace by key.
                    const next = {
                      ...cfg,
                      snippets: cfg.snippets.map(x =>
                        x.key === s.key ? edited : x
                      ),
                    };
                    setCfg(next);
                    try {
                      await invoke("save_config", { newCfg: next });
                      showToast("ok", t("snip.edit.toast.ok", lang).replace("{key}", s.key));
                    } catch (e) {
                      showToast("err", t("snip.edit.toast.fail", lang).replace("{err}", String(e)));
                      setCfg(cfg);
                    }
                  }}
                  title={t("snip.edit.button.tip", lang).replace("{key}", s.key)}
                >
                  ✎
                </button>
                <button
                  className="btn secondary"
                  style={{
                    height: 24,
                    padding: "0 8px",
                    color: "var(--c-err, #f87171)",
                    fontSize: 12,
                  }}
                  onClick={async () => {
                    // v0.0.9: delete-by-confirm. Edit/New require a 3-field
                    // modal — deferred to v0.1.0. For now users can still
                    // create new snippets via config.json directly.
                    const ok = await showConfirm(
                      t("snip.delete.confirm", lang).replace("{key}", s.key).replace("{title}", s.title),
                      { confirmLabel: t("common.delete", lang), danger: true },
                    );
                    if (!ok) return;
                    const next = { ...cfg, snippets: cfg.snippets.filter(x => x.key !== s.key) };
                    setCfg(next);
                    try {
                      await invoke("save_config", { newCfg: next });
                      showToast("ok", t("snip.delete.toast.ok", lang).replace("{key}", s.key).replace("{n}", String(next.snippets.length)));
                    } catch (e) {
                      showToast("err", t("snip.delete.toast.fail", lang).replace("{err}", String(e)));
                      // Roll back the optimistic UI update.
                      setCfg(cfg);
                    }
                  }}
                  title={t("snip.delete.button.tip", lang).replace("{key}", s.key)}
                >
                  🗑
                </button>
              </div>
            ))}
          </div>
        )}
        <div style={{ fontSize: 11, color: "var(--c-text-dim)", marginTop: 8 }}>
          {t("snippets.json.hint.before", lang)}<code>%APPDATA%\overlay-mvp\config.json</code>{t("snippets.json.hint.middle", lang)}<code>snippets</code>{t("snippets.json.hint.after", lang)}
        </div>
      </div>)}

      {activeSection === "audio" && (<div className="settings-section">
        <h3>{t("audio.stt.title", lang)}</h3>
        <div className="field">
          <label>{t("audio.stt.key.label", lang)}</label>
          <input
            type="password"
            value={cfg.groq_api_key}
            onChange={(e) => update({ groq_api_key: e.target.value })}
          />
        </div>
        <div className="field">
          <label>{t("audio.stt.lang.label", lang)}</label>
          <input
            type="text"
            value={cfg.stt_language ?? ""}
            onChange={(e) => update({ stt_language: e.target.value || null })}
            placeholder={t("audio.stt.lang.placeholder", lang)}
          />
        </div>
        <div className="field">
          <label>{t("audio.stt.model.label", lang)}</label>
          <select
            value={cfg.stt_model ?? "whisper-large-v3"}
            onChange={(e) => update({ stt_model: e.target.value })}
          >
            <option value="whisper-large-v3">{t("audio.stt.model.large", lang)}</option>
            <option value="whisper-large-v3-turbo">{t("audio.stt.model.turbo", lang)}</option>
          </select>
          <div style={{ fontSize: 11, color: "var(--c-text-dim)", marginTop: 4 }}>
            {t("audio.stt.note", lang)}
          </div>
        </div>
      </div>)}

      {activeSection === "hotkeys" && (<div className="settings-section">
        {/* v0.0.39 polish: converted from .field + text-input rows to the
            design's .card with text-input pairs. The full .hotkey-row /
            kbd pattern (used in the ℹ-popover) requires a re-binding flow
            for click-to-assign, deferred to a future release. For now:
            same text-input UX, just inside the new card frame.
            Behavior unchanged. */}
        <div className="card">
          <div className="card-title">{t("hotkeys.card.title", lang)}</div>
          <div className="hint" style={{ fontSize: 11, color: "var(--c-text-dim)" }}>
            {t("hotkeys.hint", lang)}
          </div>
          <div className="card-row">
            <div className="row-label">
              {t("hotkeys.ask.label", lang)}
              <span className="row-hint">{t("hotkeys.ask.hint", lang)}</span>
            </div>
            <div className="row-control">
              <input value={cfg.hotkey_ask} onChange={(e) => update({ hotkey_ask: e.target.value })} />
            </div>
          </div>
          <div className="card-row">
            <div className="row-label">
              {t("hotkeys.screenshot.label", lang)}
              <span className="row-hint">{t("hotkeys.screenshot.hint", lang)}</span>
            </div>
            <div className="row-control">
              <input value={cfg.hotkey_screenshot} onChange={(e) => update({ hotkey_screenshot: e.target.value })} />
            </div>
          </div>
          <div className="card-row">
            <div className="row-label">
              {t("hotkeys.toggle.label", lang)}
              <span className="row-hint">{t("hotkeys.toggle.hint", lang)}</span>
            </div>
            <div className="row-control">
              <input value={cfg.hotkey_toggle_visibility} onChange={(e) => update({ hotkey_toggle_visibility: e.target.value })} />
            </div>
          </div>
          <div className="card-row">
            <div className="row-label">
              {t("hotkeys.pause.label", lang)}
              <span className="row-hint">{t("hotkeys.pause.hint", lang)}</span>
            </div>
            <div className="row-control">
              <input value={cfg.hotkey_pause_audio} onChange={(e) => update({ hotkey_pause_audio: e.target.value })} />
            </div>
          </div>
        </div>
      </div>)}

      {activeSection === "advanced" && (<div className="settings-section">
        <h3>{t("adv.updates.title", lang)}</h3>
        <div className="field">
          <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
            <button
              className="btn secondary"
              disabled={updateBusy}
              onClick={async () => {
                setUpdateBusy(true);
                try {
                  const info = await invoke<UpdateInfo>("check_update");
                  setUpdateInfo(info);
                  if (info.error) {
                    showToast("err", t("adv.check.toast.err", lang).replace("{err}", info.error));
                  } else if (info.update_available) {
                    showToast("ok",
                      t("adv.check.toast.new", lang)
                        .replace("{latest}", info.latest ?? "?")
                        .replace("{current}", info.current));
                  } else {
                    showToast("ok", t("adv.check.toast.same", lang).replace("{current}", info.current));
                  }
                } catch (e) {
                  showToast("err", t("adv.check.toast.fail", lang).replace("{err}", String(e)));
                } finally {
                  setUpdateBusy(false);
                }
              }}
              title={t("adv.check.tip", lang)}
            >
              {updateBusy ? t("adv.check.busy", lang) : t("adv.check.button", lang)}
            </button>
            {updateInfo && !updateInfo.error && (
              <span style={{ fontSize: 12, color: "var(--c-text-dim)" }}>
                {t("adv.current.label", lang).replace("{v}", updateInfo.current)}
                {updateInfo.latest && updateInfo.latest !== updateInfo.current
                  ? t("adv.latest.suffix", lang).replace("{v}", updateInfo.latest)
                  : ""}
              </span>
            )}
          </div>
          {updateInfo && updateInfo.update_available && (
            <div
              style={{
                marginTop: 8,
                padding: 10,
                background: "color-mix(in srgb, var(--c-accent, #6366f1) 12%, transparent)",
                border: "1px solid color-mix(in srgb, var(--c-accent, #6366f1) 35%, transparent)",
                borderLeft: "3px solid var(--c-accent, #6366f1)",
                borderRadius: "var(--r-1)",
              }}
            >
              <div style={{ marginBottom: 6, fontWeight: 600 }}>
                {t("adv.available.title", lang).replace("{latest}", updateInfo.latest ?? "?")}
              </div>
              {updateInfo.notes && (
                <details style={{ marginBottom: 8 }}>
                  <summary style={{ cursor: "pointer", fontSize: 12 }}>{t("adv.available.notes", lang)}</summary>
                  <pre style={{
                    fontSize: 11,
                    whiteSpace: "pre-wrap",
                    maxHeight: 220,
                    overflowY: "auto",
                    padding: 8,
                    background: "var(--c-bg-2, rgba(0,0,0,0.2))",
                    borderRadius: 4,
                    marginTop: 4,
                  }}>{updateInfo.notes}</pre>
                </details>
              )}
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                <button
                  className="btn"
                  onClick={async () => {
                    // v0.0.23: one-click download + install. Backend
                    // fetches NSIS setup.exe, spawns it; we then quit so
                    // the installer can replace overlay-mvp.exe cleanly.
                    if (oneClickBusy) return;
                    setOneClickBusy(true);
                    try {
                      showToast("ok", t("adv.download.toast.start", lang));
                      const path = await invoke<string>("download_and_install_update");
                      showToast("ok", t("adv.download.toast.ok", lang).replace("{file}", path.split(/[\\/]/).pop() ?? ""));
                      // Give the OS a moment to bring up the UAC prompt
                      // before we kill ourselves; otherwise the user can
                      // miss the prompt and think nothing happened.
                      //
                      // v0.0.36 (agent P1): timer ID stored in ref so
                      // unmount cleanup can clear it. Previously, if user
                      // clicked Back to overlay during the 2-sec window,
                      // Settings unmounted but quit_app still fired,
                      // killing the app while user was back on the bar.
                      quitAfterDownloadTimerRef.current = window.setTimeout(() => {
                        quitAfterDownloadTimerRef.current = null;
                        if (!mountedRef.current) return;
                        invoke("quit_app").catch(() => {
                          // Fallback if quit_app refuses: hard exit by
                          // closing the overlay window which is the only
                          // main window — Tauri will tear the app down.
                          getCurrentWindow().close().catch(() => {
                            // v0.0.26: if BOTH quit_app and window.close
                            // fail (extremely rare — would mean Tauri is
                            // totally broken), unstick the button so user
                            // isn't trapped at "⏳ Скачиваю…" forever and
                            // can at least retry / report it.
                            // v0.0.28: also tell the backend to release
                            // its UPDATE_IN_FLIGHT lock — otherwise a
                            // retry click would get rejected with
                            // «Update already in progress» until the
                            // user manually restarts the app.
                            if (mountedRef.current) {
                              setOneClickBusy(false);
                              invoke("clear_update_in_flight").catch(() => {
                                // best-effort — if even this fails the
                                // user really has no choice but to kill
                                // the process from Task Manager.
                              });
                              showToast("err", t("adv.download.toast.stuck", lang));
                            }
                          });
                        });
                      }, 2000);
                    } catch (e) {
                      setOneClickBusy(false);
                      showToast("err", t("adv.download.toast.fail", lang).replace("{err}", String(e)));
                    }
                  }}
                  disabled={oneClickBusy}
                  title={t("adv.download.tip", lang)}
                >
                  {oneClickBusy ? t("adv.download.busy", lang) : t("adv.download.button", lang)}
                </button>
                <button
                  className="btn secondary"
                  onClick={async () => {
                    try {
                      const { openUrl } = await import("@tauri-apps/plugin-opener");
                      await openUrl(updateInfo.download_url);
                    } catch (e) {
                      showToast("err", t("adv.browser.toast.fail", lang).replace("{err}", String(e)));
                    }
                  }}
                  title={t("adv.browser.tip", lang)}
                >
                  {t("adv.browser.button", lang)}
                </button>
              </div>
              <div style={{ fontSize: 11, color: "var(--c-text-dim)", marginTop: 6 }}>
                {t("adv.smartscreen.note", lang)}
              </div>
            </div>
          )}
          {updateInfo && !updateInfo.update_available && !updateInfo.error && (
            <div style={{ fontSize: 11, color: "var(--c-text-dim)", marginTop: 6 }}>
              {t("adv.available.upToDate", lang).replace("{current}", updateInfo.current)}
            </div>
          )}
          <div style={{ fontSize: 11, color: "var(--c-text-dim)", marginTop: 8 }}>
            {t("adv.update.note", lang)}
          </div>
          {crashReport && (
            <div
              style={{
                marginTop: 12,
                padding: 8,
                background: "color-mix(in srgb, var(--c-err, #f87171) 12%, transparent)",
                border: "1px solid color-mix(in srgb, var(--c-err, #f87171) 35%, transparent)",
                borderLeft: "3px solid var(--c-err, #f87171)",
                borderRadius: "var(--r-1)",
              }}
            >
              <div style={{ fontWeight: 600, marginBottom: 4 }}>
                {t("adv.crash.title", lang)}
              </div>
              <div style={{ fontSize: 11, color: "var(--c-text-dim)", marginBottom: 6 }}>
                {t("adv.crash.desc", lang).split("{path}").map((part, i, arr) =>
                  i === arr.length - 1
                    ? <span key={i}>{part}</span>
                    : <span key={i}>{part}<code>{crashReport}</code></span>
                )}
              </div>
              <button
                className="btn secondary"
                onClick={async () => {
                  try {
                    const { openPath } = await import("@tauri-apps/plugin-opener");
                    await openPath(crashReport);
                  } catch (e) {
                    showToast("err", t("adv.crash.toast.fail", lang).replace("{err}", String(e)));
                  }
                }}
                title={t("adv.crash.tip", lang)}
              >
                {t("adv.crash.button", lang)}
              </button>
            </div>
          )}
          <div style={{ marginTop: 12 }}>
            <button
              className="btn secondary"
              onClick={async () => {
                try {
                  const path = await invoke<string>("dump_diagnostics");
                  showToast("ok", t("adv.dump.toast.ok", lang).replace("{path}", path));
                } catch (e) {
                  showToast("err", t("adv.dump.toast.fail", lang).replace("{err}", String(e)));
                }
              }}
              title={t("adv.dump.tip", lang)}
            >
              {t("adv.dump.button", lang)}
            </button>
            <div style={{ fontSize: 11, color: "var(--c-text-dim)", marginTop: 4 }}>
              {t("adv.dump.note", lang)}
            </div>
          </div>
        </div>

        {/* v0.0.32: moved Replay/Logs/Export×2/Import here from the footer.
           The footer was getting 7 buttons wide which wrapped Save to its
           own line. These are «диагностика / сессии» conceptually so they
           belong in the Advanced panel with the update + dump buttons. */}
        <div className="field">
          <label>{t("adv.sessions.label", lang)}</label>
          <div className="btn-row" style={{ justifyContent: "flex-start", gap: 8, flexWrap: "wrap" }}>
            <button
              className="btn secondary"
              onClick={() => {
                window.location.search = "?replay=1";
              }}
              title={t("adv.replay.tip", lang)}
            >
              {t("adv.replay.button", lang)}
            </button>
            <button
              className="btn secondary"
              onClick={() => invoke("open_sessions_folder").catch((e) => console.warn("open_sessions:", e))}
              title={t("adv.logs.tip", lang)}
            >
              {t("adv.logs.button", lang)}
            </button>
            <button
              className="btn secondary"
              onClick={async () => {
                try {
                  const path = await invoke<string>("export_config");
                  showToast("ok", t("adv.export.full.toast.ok", lang).replace("{path}", path));
                } catch (e) {
                  showToast("err", t("adv.export.fail", lang).replace("{err}", String(e)));
                }
              }}
              title={t("adv.export.full.tip", lang)}
            >
              {t("adv.export.full.button", lang)}
            </button>
            <button
              className="btn secondary"
              onClick={async () => {
                try {
                  const path = await invoke<string>("export_config_safe");
                  showToast("ok", t("adv.export.share.toast.ok", lang).replace("{path}", path));
                } catch (e) {
                  showToast("err", t("adv.export.fail", lang).replace("{err}", String(e)));
                }
              }}
              title={t("adv.export.share.tip", lang)}
            >
              {t("adv.export.share.button", lang)}
            </button>
            <button
              className="btn secondary"
              onClick={async () => {
                try {
                  const { open } = await import("@tauri-apps/plugin-dialog");
                  const path = await open({
                    multiple: false,
                    directory: false,
                    title: t("adv.import.dialog.title", lang),
                    filters: [
                      { name: t("adv.import.filter.json", lang), extensions: ["json"] },
                      { name: t("adv.import.filter.all", lang), extensions: ["*"] },
                    ],
                  });
                  if (!path) return;
                  const picked = typeof path === "string" ? path : path[0];
                  await invoke("import_config", { path: picked });
                  const fresh = await invoke<Config>("get_config");
                  setCfg(fresh);
                  showToast("ok", t("adv.import.toast.ok", lang));
                } catch (e) {
                  showToast("err", t("adv.import.toast.fail", lang).replace("{err}", String(e)));
                }
              }}
              title={t("adv.import.tip", lang)}
            >
              {t("adv.import.button", lang)}
            </button>
          </div>
          <div style={{ fontSize: 11, color: "var(--c-text-dim)", marginTop: 4 }}>
            {t("adv.export.note", lang)}
          </div>
        </div>
      </div>)}

        </section>
      </div>
      {/* end v0.0.30 settings-shell (sidebar + pane) */}

      {/* v0.0.32: footer minimal — Back + Save. Other 5 buttons moved to
         Advanced panel. v0.0.34: added `.settings-footer` class for the
         visual pin treatment (border-top + bg-2) so it reads as fixed
         instead of floating. */}
      <div className="btn-row settings-footer">
        {savedFlash && <span style={{ color: "#4ade80", alignSelf: "center" }}>{t("settings.saved", lang)}</span>}
        <button className="btn secondary" onClick={back}>{t("settings.back", lang)}</button>
        <button className="btn" onClick={async () => { await save(); }}>{t("settings.save", lang)}</button>
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
            aria-label={t("toast.close", lang)}
            title={t("toast.close", lang)}
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
            else if (modal.kind === "confirm") modal.onNo();
            else if (modal.kind === "snippet") modal.onCancel();
          }}
        >
          <div
            className="settings-modal"
            onMouseDown={(e) => e.stopPropagation()}
            role="dialog"
            aria-modal="true"
          >
            <div className="settings-modal-title">{modal.title}</div>
            {modal.kind === "prompt" && (
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
                  <button className="btn secondary" onClick={modal.onCancel}>{t("common.cancel", lang)}</button>
                  <button
                    className="btn"
                    onClick={() => modal.onSubmit(promptValue)}
                    disabled={!promptValue.trim()}
                  >OK</button>
                </div>
              </>
            )}
            {modal.kind === "confirm" && (
              <div className="settings-modal-actions">
                <button className="btn secondary" autoFocus onClick={modal.onNo}>{t("common.cancel", lang)}</button>
                {/* v0.0.31: use the caller-supplied label + danger flag.
                   Default «Подтвердить» (neutral). Danger callers (delete
                   profile / snippet) pass `danger:true` for red styling. */}
                <button
                  className={modal.danger ? "btn settings-modal-danger" : "btn"}
                  onClick={modal.onYes}
                >
                  {modal.confirmLabel ?? t("modal.confirm.default", lang)}
                </button>
              </div>
            )}
            {modal.kind === "snippet" && (
              <>
                <div className="field">
                  <label>{t("snip.modal.key.label", lang).replace("{key}", snipKey || "key")}</label>
                  <input
                    type="text"
                    autoFocus={modal.isNew}
                    value={snipKey}
                    disabled={!modal.isNew}
                    onChange={(e) => {
                      setSnipKey(e.target.value.trim().toLowerCase());
                      setSnipError("");
                    }}
                    placeholder={t("snip.modal.key.placeholder", lang)}
                  />
                  {!modal.isNew && (
                    <div style={{ fontSize: 11, color: "var(--c-text-dim)", marginTop: 4 }}>
                      {t("snip.modal.key.locked.hint", lang)}
                    </div>
                  )}
                </div>
                <div className="field">
                  <label>{t("snip.modal.title.label", lang)}</label>
                  <input
                    type="text"
                    autoFocus={!modal.isNew}
                    value={snipTitle}
                    onChange={(e) => { setSnipTitle(e.target.value); setSnipError(""); }}
                    placeholder={t("snip.modal.title.placeholder", lang)}
                  />
                </div>
                <div className="field">
                  <label>{t("snip.modal.body.label", lang)}</label>
                  <textarea
                    rows={8}
                    style={{ width: "100%", fontFamily: "var(--font-mono, monospace)", fontSize: 12 }}
                    value={snipBody}
                    onChange={(e) => { setSnipBody(e.target.value); setSnipError(""); }}
                    placeholder={t("snip.modal.body.placeholder", lang)}
                  />
                </div>
                {snipError && (
                  <div style={{
                    fontSize: 12,
                    color: "var(--c-err, #f87171)",
                    padding: "6px 8px",
                    background: "color-mix(in srgb, var(--c-err, #f87171) 10%, transparent)",
                    borderLeft: "3px solid var(--c-err, #f87171)",
                    borderRadius: 4,
                    marginBottom: 8,
                  }}>{snipError}</div>
                )}
                <div className="settings-modal-actions">
                  <button className="btn secondary" onClick={modal.onCancel}>{t("common.cancel", lang)}</button>
                  <button
                    className="btn"
                    onClick={() => {
                      // NOTE: rename `t` → `title` to avoid shadowing the
                      // imported t() translation function (agent v0.0.51
                      // review caught this footgun).
                      const k = snipKey.trim().toLowerCase();
                      const title = snipTitle.trim();
                      const b = snipBody.trim();
                      if (!k) { setSnipError(t("snip.error.key.required", lang)); return; }
                      // No /i flag — toLowerCase() above already canonicalised.
                      if (!/^[a-z0-9][a-z0-9-_]*$/.test(k)) {
                        setSnipError(t("snip.error.key.format", lang));
                        return;
                      }
                      if (!title) { setSnipError(t("snip.error.title.required", lang)); return; }
                      if (!b) { setSnipError(t("snip.error.body.required", lang)); return; }
                      if (modal.isNew && modal.existingKeys.includes(k)) {
                        setSnipError(t("snip.error.key.dup", lang).replace("{key}", k));
                        return;
                      }
                      modal.onSubmit({ key: k, title, body: b });
                    }}
                  >{t("common.save", lang)}</button>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
