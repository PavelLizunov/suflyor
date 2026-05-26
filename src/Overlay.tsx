import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow, currentMonitor, LogicalSize } from "@tauri-apps/api/window";
import { t, resolveLang, type Lang } from "./i18n";

type AudioSource = "system" | "mic";

type TranscriptLine = {
  source: AudioSource;
  text: string;
  timestamp_ms: number;
};

type AiEvent =
  | { type: "start"; id: string }
  | { type: "delta"; text: string }
  | { type: "done"; reason: string }
  | { type: "error"; message: string };

type Status = "stopped" | "listening" | "thinking" | "answering" | "error" | "paused";

type HealthState = "ok" | "degraded" | "down" | "idle";
type HealthPayload = {
  audio: HealthState;
  stt: HealthState;
  ai: HealthState;
  audio_age_ms: number | null;
  stt_age_ms: number | null;
  ai_age_ms: number | null;
};

type SpeechPace = "low" | "ok" | "fast" | "idle";
type SpeechCoach = {
  words_60s: number;
  fillers_60s: number;
  filler_per_100: number | null;
  wpm: number | null;
  pace: SpeechPace;
};

type TimerHandle = ReturnType<typeof setTimeout>;

export default function Overlay() {
  const [status, setStatus] = useState<Status>("stopped");
  const [errorText, setErrorText] = useState<string>("");
  const [lastLines, setLastLines] = useState<TranscriptLine[]>([]);
  const [answer, setAnswer] = useState<string>("");
  const [hasScreenshot, setHasScreenshot] = useState(false);
  const [hotkeyWarnings, setHotkeyWarnings] = useState<string[]>([]);
  const [rateLimited, setRateLimited] = useState(false);
  // v0.0.12: separate "over budget" chip (was conflated with rate-limit).
  // Soft warning per v0.0.5 cost cap pivot — AI keeps working, user just
  // sees they've crossed their configured budget.
  const [overBudget, setOverBudget] = useState(false);
  const [sessionCost, setSessionCost] = useState(0);
  // Push-to-talk mode + live recording indicator state.
  const [askMode, setAskMode] = useState<"click" | "hold">("hold");
  const [recordingSource, setRecordingSource] = useState<"mic" | "system" | null>(null);
  const [recordingStartMs, setRecordingStartMs] = useState<number>(0);
  // Elapsed seconds while recording — updated by an interval so the counter
  // visibly ticks. Previously we forced a re-render via a `void recTick;`
  // hack; this is cleaner and avoids the wall-clock-based race on stop.
  const [elapsedSec, setElapsedSec] = useState(0);
  // UI preference: show running session cost chip in the overlay bar.
  // Stored client-side in localStorage so Settings can flip it without a
  // backend round-trip. Default true preserves prior behaviour.
  const [showCost, setShowCost] = useState<boolean>(() => {
    try { return localStorage.getItem("overlay.showCost") !== "false"; }
    catch { return true; }
  });
  // v0.0.55: compact mode flag. When true, the bar hides the optional
  // chips (cost, wpm, screenshot, aggressive, rate-limited, over-budget,
  // hotkey-warnings) — leaves only status text + dot + HUD dots + PTT
  // buttons + gear. Same localStorage pattern as showCost; flipped from
  // Settings → 🎨 Interface → Compact mode.
  const [compact, setCompact] = useState<boolean>(() => {
    try { return localStorage.getItem("overlay.compact") === "true"; }
    catch { return false; }
  });
  // Failure HUD — 3 dots (audio/stt/ai). null = no signal received yet.
  const [health, setHealth] = useState<HealthPayload | null>(null);
  // Voice coach — live mic WPM / filler density. null = backend hasn't
  // emitted yet (pre-session or just after start).
  const [coach, setCoach] = useState<SpeechCoach | null>(null);
  // KB palette state — opened by F4 global hotkey.
  type KBHit = { key: string; heading: string; body: string; source: string };
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [paletteQuery, setPaletteQuery] = useState("");
  const [paletteResults, setPaletteResults] = useState<KBHit[]>([]);
  const [paletteIdx, setPaletteIdx] = useState(0);
  const paletteInputRef = useRef<HTMLInputElement | null>(null);
  // v0.0.21: visible hotkey legend popover open/closed state.
  // v0.0.33 doubled popover height by adding the indicator legend.
  // v0.0.36 (agent P0): popover paints inside `.overlay-root` which
  // has `overflow: hidden`, AND it's `position: absolute` so it
  // doesn't contribute to contentRect → ResizeObserver doesn't grow
  // the OS window → bottom half of the popover is invisible. Mirror
  // the palette resize pattern: stash original size, grow on open,
  // restore on close. Declared up here (was at L220) so the resize
  // useEffect below can reference it without a forward-reference TS
  // error.
  const [hotkeyHelpOpen, setHotkeyHelpOpen] = useState(false);
  const originalHelpSizeRef = useRef<{ w: number; h: number } | null>(null);
  // v0.0.48: UI language for overlay chrome. Loaded from config on mount
  // (same useEffect that reads manual_ask_mode + auto_tile_every_line).
  // Defaults to "ru" until config arrives — same pattern as Settings.
  const [lang, setLang] = useState<Lang>("ru");
  // v0.0.56: recent KB queries history. Seeded from localStorage on
  // mount; shown as quick-pick chips when palette is open + input is
  // empty. Click a chip → re-runs that query. Cap at 10 entries.
  const [kbHistory, setKbHistory] = useState<string[]>(() => {
    try {
      const raw = localStorage.getItem("kb.history");
      if (!raw) return [];
      const arr = JSON.parse(raw);
      return Array.isArray(arr) ? arr.filter((s) => typeof s === "string").slice(0, 10) : [];
    } catch { return []; }
  });
  const pushKbHistory = (q: string) => {
    const trimmed = q.trim();
    if (trimmed.length < 2) return;
    setKbHistory((prev) => {
      const next = [trimmed, ...prev.filter((x) => x.toLowerCase() !== trimmed.toLowerCase())].slice(0, 10);
      try { localStorage.setItem("kb.history", JSON.stringify(next)); }
      catch (err) { console.warn("kb.history write failed:", err); }
      return next;
    });
  };
  // Search debounced when palette open + query non-empty.
  useEffect(() => {
    if (!paletteOpen) return;
    const q = paletteQuery.trim();
    if (!q) { setPaletteResults([]); setPaletteIdx(0); return; }
    let cancelled = false;
    const t = setTimeout(() => {
      invoke<KBHit[]>("kb_search", { query: q, limit: 8 })
        .then((res) => {
          if (cancelled || !mountedRef.current) return;
          setPaletteResults(res);
          setPaletteIdx(0);
        })
        .catch((e) => {
          if (cancelled || !mountedRef.current) return;
          console.warn("kb_search:", e);
          setPaletteResults([]);
        });
    }, 80);
    return () => { cancelled = true; clearTimeout(t); };
  }, [paletteQuery, paletteOpen]);
  // Focus input + grow overlay window so palette results don't clip into
  // overflow:hidden (S1 from 2nd-pass: palette `top:40px` + max-height 280
  // overflows the 96px overlay-bar window). On close, shrink back.
  // Stash original size in ref so we restore exactly.
  const originalSizeRef = useRef<{ w: number; h: number } | null>(null);
  useEffect(() => {
    const w = getCurrentWindow();
    if (paletteOpen) {
      requestAnimationFrame(() => paletteInputRef.current?.focus());
      // Save current size + grow to fit max palette (input ~36 + 8 results
      // × ~28 + padding ≈ 280 + bar ≈ 36 = ~320, +slack).
      (async () => {
        try {
          const size = await w.outerSize();
          const scale = await w.scaleFactor();
          originalSizeRef.current = {
            w: Math.round(size.width / scale),
            h: Math.round(size.height / scale),
          };
          await w.setSize(new LogicalSize(Math.max(originalSizeRef.current.w, 540), 380));
        } catch (e) { console.warn("palette resize:", e); }
      })();
    } else if (originalSizeRef.current) {
      const { w: w0, h: h0 } = originalSizeRef.current;
      originalSizeRef.current = null;
      w.setSize(new LogicalSize(w0, h0)).catch((e) => console.warn("palette restore:", e));
    }
  }, [paletteOpen]);
  // v0.0.36 (agent P0): hotkey-help popover is `position: absolute`
  // inside `.overlay-root { overflow: hidden }` → does NOT contribute
  // to contentRect → ResizeObserver doesn't grow the OS window → the
  // bottom of the popover is invisibly clipped. Mirror palette: stash
  // current size, grow to fit (~500 px tall covers the hotkey table +
  // indicator legend with slack), restore on close.
  useEffect(() => {
    const w = getCurrentWindow();
    if (hotkeyHelpOpen) {
      (async () => {
        try {
          const size = await w.outerSize();
          const scale = await w.scaleFactor();
          originalHelpSizeRef.current = {
            w: Math.round(size.width / scale),
            h: Math.round(size.height / scale),
          };
          // Keep current width; grow height to fit the popover.
          // 8 hotkey rows + 9 indicator rows + 2 headers + padding.
          // v0.0.36 smoke test: 500 cut off the bottom 2 indicator
          // rows (💰 over budget, 💰 $). Bumped to 600 with slack.
          await w.setSize(new LogicalSize(originalHelpSizeRef.current.w, 600));
        } catch (e) { console.warn("help-popover resize:", e); }
      })();
    } else if (originalHelpSizeRef.current) {
      const { w: w0, h: h0 } = originalHelpSizeRef.current;
      originalHelpSizeRef.current = null;
      w.setSize(new LogicalSize(w0, h0)).catch((e) => console.warn("help-popover restore:", e));
    }
  }, [hotkeyHelpOpen]);
  // Esc-anywhere closes the palette — the input's own onKeyDown only fires
  // while the input has focus, but in practice focus can land on result
  // items or get lost to driver clicks. A window-level keydown effect is
  // the same pattern I used for Settings confirm-modals. Capture phase so
  // the press doesn't reach DevTools etc.
  useEffect(() => {
    if (!paletteOpen) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        closePalette();
      }
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [paletteOpen]);
  const closePalette = () => {
    setPaletteOpen(false);
    setPaletteQuery("");
    setPaletteResults([]);
    setPaletteIdx(0);
  };
  const expandSelected = async () => {
    const hit = paletteResults[paletteIdx];
    if (!hit) return;
    // v0.0.56: record successful expansion in KB history.
    pushKbHistory(paletteQuery);
    try {
      await invoke("kb_spawn", { key: hit.key });
      closePalette();
    } catch (e) {
      console.warn("kb_spawn:", e);
    }
  };

  // --- Stable refs to dodge stale-closure traps in event listeners ----
  // The hotkey:pause_audio listener registered in the [] effect previously
  // captured `status` at mount-time and got stuck "stopped". This ref is
  // updated by an effect below and read inside the listener.
  const statusRef = useRef<Status>(status);
  useEffect(() => { statusRef.current = status; }, [status]);

  // Refs for the various setTimeout one-shots so they can be cancelled on
  // unmount (and replaced cleanly when re-fired before they elapsed).
  const screenshotTimerRef = useRef<TimerHandle | null>(null);
  const rateTimerRef = useRef<TimerHandle | null>(null);
  const errorTimerRef = useRef<TimerHandle | null>(null);
  const startSessionTimerRef = useRef<TimerHandle | null>(null);
  const overBudgetTimerRef = useRef<TimerHandle | null>(null);
  // v0.0.21: serialises pause/resume (F8) so rapid double-press doesn't
  // start a new session while the previous stop is still tearing down
  // WASAPI. Audio device race was crashing the app mid-call.
  const pauseInFlightRef = useRef(false);
  // hotkeyHelpOpen + originalHelpSizeRef moved to top with paletteOpen
  // (forward-reference TS error from the resize useEffect above).
  // Mounted-flag for promise resolutions that may land after unmount
  // (StrictMode double-mount, settings round-trip).
  const mountedRef = useRef(true);

  // Hold latest answer for streaming concatenation without React batching loss.
  const answerRef = useRef("");

  // v0.0.26: auto-resize overlay window based on actual content of
  // overlay-root (which includes the bar + transcript-tail + answer-bubble
  // siblings). v0.0.25 observed only the bar and hard-coded height=96 in
  // setSize — that CLIPPED transcript-tail / answer-bubble (they grow
  // downward below the bar) AND undid the user's manual vertical resize.
  // Agent-review finding 2026-05-26.
  //
  // Now: ResizeObserver on overlay-root → measure BOTH width and height
  // of the actual visible flex column. setSize preserves whatever vertical
  // growth the children produced.
  //
  // Skipped when palette is open — the palette resize logic (see useEffect
  // at line 110) sets its own size; observing here would race.
  const overlayBarRef = useRef<HTMLDivElement | null>(null);
  const overlayRootRef = useRef<HTMLDivElement | null>(null);
  // v0.0.33: removed `paletteOpenRef` — the ref-update useEffect ran
  // AFTER React commit, leaving a race window where ResizeObserver
  // could fire DURING the palette open/close transition and see palette
  // content with the guard still stale. That triggered a setSize from
  // RO while the palette's own setSize was in-flight → competing setSize
  // calls → potential hang on rapid F4-keystroke-Esc cycles. The fix
  // moves the guard into the `useEffect` deps array — RO is now literally
  // not attached while palette is open. Zero race possible.
  // v0.0.35: track whether we've done the initial size-fit. v0.0.34's
  // infinite-grow bug left some users with a persisted 1000+ px window
  // state — on next launch that gets restored, and a pure grow-only
  // policy would never shrink it back. We do ONE shrink-allowed fit on
  // the first RO fire of each session; after that, grow-only.
  const initialFitDoneRef = useRef(false);
  // v0.0.36 (agent P2): cap by CURRENT monitor's width, not
  // window.screen.availWidth (primary monitor only). User who drags
  // the overlay to a wider secondary monitor was previously stuck
  // with the primary monitor's cap. Tauri's currentMonitor() returns
  // the screen the window is on. Refreshed on `move` events.
  const currentMonitorWRef = useRef<number>(1920);
  useEffect(() => {
    const w = getCurrentWindow();
    const refresh = async () => {
      try {
        const m = await currentMonitor();
        if (m) {
          // m.size is in PHYSICAL pixels; divide by scaleFactor for logical.
          const scale = m.scaleFactor;
          currentMonitorWRef.current = Math.round(m.size.width / scale);
        }
      } catch { /* monitor info unavailable — keep default */ }
    };
    refresh();
    let unlistenFn: (() => void) | null = null;
    w.onMoved(() => { refresh(); }).then((u) => { unlistenFn = u; }).catch(() => {});
    return () => { if (unlistenFn) unlistenFn(); };
  }, []);
  useEffect(() => {
    if (!overlayRootRef.current) return;
    if (paletteOpen) return; // RO disabled while palette is open
    if (hotkeyHelpOpen) return; // v0.0.36: same gate for help popover
    const ro = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      // v0.0.35: FIX infinite-grow bug in v0.0.34.
      // v0.0.34 used `bar.scrollWidth` to estimate intrinsic width — but
      // when children fit, scrollWidth == offsetWidth == current window
      // width. So scrollWidth+50 > currentW+4 was always true → setSize
      // grew the window → next RO fire scrollWidth == new larger width →
      // grew again → infinite loop. User reported overlay «уехало в
      // бесконечность» on startup.
      //
      // Real intrinsic measurement: SUM children offsetWidths + gaps +
      // bar's horizontal padding. With `.overlay-bar > * { flex-shrink:
      // 0 }`, each child's offsetWidth IS its natural width regardless
      // of bar/window size. Sum = true intrinsic content extent, stable
      // across window resizes.
      //
      // Policy unchanged from v0.0.34:
      //   - GROW only when intrinsic + 50 > current width
      //   - Never shrink — user can drag wider freely
      //   - Height auto-grows for transcript-tail / answer-bubble
      //   - Hard safety: never exceed screen.availWidth - 20
      const bar = overlayBarRef.current;
      let intrinsicBarW = 0;
      if (bar) {
        const children = Array.from(bar.children) as HTMLElement[];
        const cs = window.getComputedStyle(bar);
        const gap = parseFloat(cs.gap) || 0;
        const padL = parseFloat(cs.paddingLeft) || 0;
        const padR = parseFloat(cs.paddingRight) || 0;
        intrinsicBarW = children.reduce((sum, c) => sum + c.offsetWidth, 0)
          + gap * Math.max(0, children.length - 1)
          + padL + padR;
      }
      const measuredH = Math.ceil(entry.contentRect.height);
      const desiredH = Math.min(Math.max(measuredH + 4, 96), 900);
      getCurrentWindow().outerSize().then((sz) => {
        getCurrentWindow().scaleFactor().then((scale) => {
          const currentW = Math.round(sz.width / scale);
          const currentH = Math.round(sz.height / scale);
          // Hard ceiling: CURRENT monitor width - 20 px so even with a
          // bug the window can never escape the visible monitor. Floor
          // 520 keeps a usable minimum on first paint.
          // v0.0.36: switched from `window.screen.availWidth` (primary
          // monitor only) to `currentMonitorWRef` (Tauri's currentMonitor,
          // updated on window move). User who drags overlay to a wider
          // secondary monitor now correctly gets the wider cap.
          const monitorW = currentMonitorWRef.current;
          const maxAllowedW = Math.max(520, monitorW - 20);
          const neededW = Math.min(Math.max(intrinsicBarW + 50, 520), maxAllowedW);
          // GROW-ONLY after the initial fit. On the FIRST RO fire of
          // this session we allow SHRINK too — this corrects a persisted
          // oversized window state (e.g. user upgraded from v0.0.34
          // which had an infinite-grow bug). Subsequent fires are
          // grow-only so user manual-drag stays respected.
          const isFirstFit = !initialFitDoneRef.current;
          const targetW = isFirstFit
            ? neededW
            : (neededW > currentW + 4 ? neededW : currentW);
          if (isFirstFit) initialFitDoneRef.current = true;
          if (Math.abs(currentW - targetW) > 4 || Math.abs(currentH - desiredH) > 4) {
            getCurrentWindow().setSize(new LogicalSize(targetW, desiredH))
              .catch((err) => console.warn("overlay autoresize:", err));
          }
        }).catch(() => {});
      }).catch(() => {});
    });
    ro.observe(overlayRootRef.current);
    return () => ro.disconnect();
  }, [paletteOpen, hotkeyHelpOpen]);

  // v0.0.25: re-assert always-on-top every 3s. User complaint: overlay
  // bar sometimes goes BEHIND other always-on-top windows (Zoom call
  // window, screen-share toolbars). Tauri's always_on_top=true is set
  // at creation but Windows reorders TOPMOST windows on focus changes;
  // SetWindowPos(HWND_TOPMOST) each tick keeps us on top of the
  // always-on-top stack.
  useEffect(() => {
    const tick = async () => {
      if (!mountedRef.current) return;
      try {
        await getCurrentWindow().setAlwaysOnTop(true);
      } catch { /* ok — sometimes fails transiently during window state changes */ }
    };
    tick();
    const id = window.setInterval(tick, 3000);
    return () => window.clearInterval(id);
  }, []);

  // Cleanup all timers + flag on unmount, in ONE place.
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      for (const r of [screenshotTimerRef, rateTimerRef, errorTimerRef, startSessionTimerRef, overBudgetTimerRef]) {
        if (r.current) {
          clearTimeout(r.current);
          r.current = null;
        }
      }
    };
  }, []);

  // Recording elapsed-seconds ticker — re-armed whenever a PTT capture starts.
  useEffect(() => {
    if (!recordingSource) {
      setElapsedSec(0);
      return;
    }
    setElapsedSec(0);
    const id = setInterval(() => {
      setElapsedSec((Date.now() - recordingStartMs) / 1000);
    }, 100);
    return () => clearInterval(id);
  }, [recordingSource, recordingStartMs]);

  useEffect(() => {
    // Settings lives in a separate Tauri window — its localStorage writes
    // dispatch a 'storage' event here on the overlay window. We react to
    // that so flipping the toggle is instant, no overlay reload needed.
    const onStorage = (e: StorageEvent) => {
      if (e.key === "overlay.showCost") {
        setShowCost(e.newValue !== "false");
      } else if (e.key === "overlay.compact") {
        // v0.0.55: same pattern as showCost — Settings writes flips
        // compact mode without reload.
        setCompact(e.newValue === "true");
      }
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  useEffect(() => {
    document.body.classList.add("overlay");
    return () => document.body.classList.remove("overlay");
  }, []);

  // v0.0.26: also pull auto_tile_every_line so we can show a 🔥 chip
  // in the bar when aggressive mode is on. User easily forgets this is
  // enabled between sessions — chip is a status indicator (not a cost
  // warning per v0.0.28; user said unlimited budget).
  const [aggressive, setAggressive] = useState(false);

  // Load ask-mode + aggressive flag + ui_language from config once on mount.
  useEffect(() => {
    invoke<{ manual_ask_mode?: string; auto_tile_every_line?: boolean; ui_language?: string }>("get_config")
      .then((c) => {
        if (!mountedRef.current) return;
        const mode = c.manual_ask_mode === "click" ? "click" : "hold";
        setAskMode(mode);
        setAggressive(Boolean(c.auto_tile_every_line));
        setLang(resolveLang(c.ui_language));
      })
      .catch((e) => console.warn("get_config:", e));
  }, []);

  // Re-read aggressive flag + ui_language on window-focus as a safety net.
  // NOTE: in the common Settings → overlay path the overlay actually
  // unmounts (Settings lives in the same window under `?settings=1`), so
  // the mount-time useEffect above is what restores the chip then. This
  // focus listener covers the secondary case where the user alt-tabs away
  // from the entire suflyor window and back after editing config via some
  // other route (e.g. hand-editing %APPDATA%\overlay-mvp\config.json in
  // Notepad). v0.0.48 also re-reads ui_language so switching language in
  // Settings + Save reflects on the overlay without an app restart.
  useEffect(() => {
    const onFocus = () => {
      if (!mountedRef.current) return;
      invoke<{ auto_tile_every_line?: boolean; ui_language?: string }>("get_config")
        .then((c) => {
          if (!mountedRef.current) return;
          setAggressive(Boolean(c.auto_tile_every_line));
          setLang(resolveLang(c.ui_language));
        })
        .catch(() => {});
    };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, []);

  // Push-to-talk handlers — start on mousedown, stop on mouseup/leave.
  const holdStart = async (source: "mic" | "system") => {
    if (askMode !== "hold" || recordingSource) return;
    setRecordingSource(source);
    setRecordingStartMs(Date.now());
    try {
      await invoke("manual_ask_hold_start", { source });
    } catch (e) {
      console.warn("manual_ask_hold_start:", e);
      if (mountedRef.current) setRecordingSource(null);
    }
  };
  const holdEnd = async (source: "mic" | "system") => {
    if (askMode !== "hold" || recordingSource !== source) return;
    setRecordingSource(null);
    try {
      await invoke("manual_ask_hold_end", { source });
    } catch (e) {
      console.warn("manual_ask_hold_end:", e);
    }
  };

  // Click-mode handler — original behaviour (last 5 lines).
  const clickAsk = (source: "mic" | "system") => {
    const cmd = source === "system" ? "ask_from_system" : "ask_from_mic";
    invoke(cmd).catch((e) => console.warn(`${cmd}:`, e));
  };

  // Helper: set an auto-clearing flag using a tracked timer ref.
  const flashFlag = (
    ref: React.MutableRefObject<TimerHandle | null>,
    setter: (v: boolean) => void,
    on: boolean,
    autoOffMs: number
  ) => {
    setter(on);
    if (ref.current) {
      clearTimeout(ref.current);
      ref.current = null;
    }
    if (on && autoOffMs > 0) {
      ref.current = setTimeout(() => {
        if (mountedRef.current) setter(false);
        ref.current = null;
      }, autoOffMs);
    }
  };

  // Auto-start session: defer to next tick so WebView2 finishes initialisation
  // before we hit Tauri commands (avoids race in WebResourceRequested handler).
  // Also restore transcript tail from backend (survives Settings round-trip).
  useEffect(() => {
    startSessionTimerRef.current = setTimeout(() => {
      try {
        invoke<TranscriptLine[]>("get_transcript")
          .then((lines) => {
            if (!mountedRef.current) return;
            if (lines.length > 0) {
              setLastLines(lines.slice(-5));
            }
          })
          .catch((e) => {
            if (!mountedRef.current) return;
            console.warn("get_transcript on mount:", e);
            setErrorText("get_transcript: " + String(e));
          });

        invoke("start_session")
          .then(() => {
            if (mountedRef.current) setStatus("listening");
          })
          .catch((e) => {
            if (!mountedRef.current) return;
            setStatus("error");
            setErrorText("start_session: " + String(e));
          });
      } catch (outer) {
        if (mountedRef.current) {
          setStatus("error");
          setErrorText("outer: " + String(outer));
        }
      }
      startSessionTimerRef.current = null;
    }, 800);
    // CRITICAL: cancel the deferred start on unmount. In React StrictMode
    // (dev), the component mounts→unmounts→re-mounts rapidly. Without this
    // cleanup, BOTH mount instances queue a start_session call 800ms later;
    // the second one races against the first's still-initialising audio
    // device → fails → status="error" + sticky chip. Cancel cleanly.
    return () => {
      if (startSessionTimerRef.current) {
        clearTimeout(startSessionTimerRef.current);
        startSessionTimerRef.current = null;
      }
    };
  }, []);

  // Wire up all event listeners. Single [] effect — listeners use refs to
  // read latest state. Cleanup awaits all listen() promises before invoking
  // their unlisten fns, so an early unmount doesn't leak listeners.
  useEffect(() => {
    const unlistens: Promise<UnlistenFn>[] = [];

    unlistens.push(
      listen<TranscriptLine>("transcript:line", (e) => {
        setLastLines((prev) => [...prev.slice(-4), e.payload]);
        // Clear sticky Error chip on real backend activity. If a transcript
        // arrives, the session IS running regardless of what an earlier
        // failed start_session attempt set the status to.
        if (statusRef.current === "error") {
          setStatus("listening");
          setErrorText("");
        }
      })
    );

    unlistens.push(
      listen<AiEvent>("ai:event", (e) => {
        const ev = e.payload;
        if (ev.type === "start") {
          answerRef.current = "";
          setAnswer("");
          setStatus("answering");
        } else if (ev.type === "delta") {
          answerRef.current += ev.text;
          setAnswer(answerRef.current);
        } else if (ev.type === "done") {
          setStatus("listening");
        } else if (ev.type === "error") {
          setStatus("error");
          setErrorText(ev.message);
        }
      })
    );

    unlistens.push(
      listen<void>("hotkey:ask", async () => {
        setStatus("thinking");
        try {
          await invoke("ask_ai");
        } catch (err) {
          setStatus("error");
          setErrorText(String(err));
        }
      })
    );

    unlistens.push(
      listen<void>("hotkey:screenshot", async () => {
        try {
          await invoke<string>("take_screenshot");
          // Clear flag after 5 s (screenshot is consumed on next ask).
          flashFlag(screenshotTimerRef, setHasScreenshot, true, 5000);
        } catch (err) {
          console.error("screenshot:", err);
        }
      })
    );

    unlistens.push(
      listen<string[]>("hotkeys:warnings", (e) => {
        setHotkeyWarnings(e.payload || []);
      })
    );

    // Single cost:update handler — keeps the running session_usd display
    // up to date AND resets the soft-budget chip when a new session starts
    // (cost goes back to 0). Previously two separate listeners did this;
    // collapsed in v0.0.13 to reduce event-bus chatter and avoid the future
    // "which listener wins on cleanup" trap.
    unlistens.push(
      listen<{ session_usd: number }>("cost:update", (e) => {
        setSessionCost(e.payload.session_usd);
        // start_session emits {session_usd: 0} so a stale chip from the prior
        // session clears immediately. Cancel any pending 60s auto-clear too.
        if (e.payload.session_usd === 0 && mountedRef.current) {
          flashFlag(overBudgetTimerRef, setOverBudget, false, 0);
        }
      })
    );

    // Cost budget warning (v0.0.5 soft warn semantics — AI continues, this
    // is a passive notice). Persists for 60s so the user notices it but
    // doesn't stay forever. Distinct chip from rate-limited — they mean
    // different things (rate-limit = AI WAS dropped this call; over budget
    // = AI succeeded but you crossed your configured budget).
    unlistens.push(
      listen<{ reason: string; source: string; blocking?: boolean }>("cost:cap-hit", (e) => {
        if (!mountedRef.current) return;
        // Use flashFlag so a fresh cap-hit re-extends the 60s window (instead
        // of leaving the original timer running to clear-early) AND so the
        // timer is tracked in overBudgetTimerRef for unmount cleanup.
        flashFlag(overBudgetTimerRef, setOverBudget, true, 60_000);
        console.warn(`over budget (source=${e.payload.source}): ${e.payload.reason}`);
      })
    );

    unlistens.push(
      listen<HealthPayload>("health:update", (e) => {
        if (!mountedRef.current) return;
        // Narrow unknown backend states to "idle" so a future enum value
        // doesn't render as `.hud-undefined` (S1 from 2nd-pass).
        const allowed: HealthState[] = ["ok", "degraded", "down", "idle"];
        const sanitize = (s: HealthState): HealthState =>
          allowed.includes(s) ? s : "idle";
        setHealth({
          ...e.payload,
          audio: sanitize(e.payload.audio),
          stt: sanitize(e.payload.stt),
          ai: sanitize(e.payload.ai),
        });
      })
    );

    unlistens.push(
      listen<SpeechCoach>("speech:coach", (e) => {
        if (!mountedRef.current) return;
        const allowedPace: SpeechPace[] = ["low", "ok", "fast", "idle"];
        const pace: SpeechPace = allowedPace.includes(e.payload.pace)
          ? e.payload.pace
          : "idle";
        setCoach({ ...e.payload, pace });
      })
    );

    unlistens.push(
      listen<void>("hotkey:kb-palette", () => {
        // F4-while-open: re-focus the input (S2 from 2nd-pass — was a noop).
        if (paletteOpen) {
          requestAnimationFrame(() => paletteInputRef.current?.focus());
        } else {
          setPaletteOpen(true);
        }
      })
    );

    unlistens.push(
      listen<{ text: string }>("tile:rate-limited", () => {
        flashFlag(rateTimerRef, setRateLimited, true, 3000);
      })
    );

    // Backend emits this when 🔊/🎤 manual-ask is pressed with empty
    // transcript (or any other tile-side failure). Without a listener the
    // user would press the button and see nothing happen.
    unlistens.push(
      listen<{ message: string }>("tile:error", (e) => {
        const msg = e.payload.message || "tile error";
        setErrorText(msg);
        // Auto-clear after a few seconds so it doesn't linger.
        if (errorTimerRef.current) clearTimeout(errorTimerRef.current);
        errorTimerRef.current = setTimeout(() => {
          if (mountedRef.current) setErrorText("");
          errorTimerRef.current = null;
        }, 5000);
      })
    );

    unlistens.push(
      listen<void>("hotkey:pause_audio", async () => {
        // v0.0.21: re-entry guard. User reported F8-during-call crashes
        // periodically — root cause was rapid double-press firing a second
        // start_session while the first stop_session was still awaiting
        // its WASAPI shutdown. Audio device race → panic in worker thread.
        // Now a ref flag drops the second press silently. flashFlag-style
        // pattern but for a simple in-flight bool, not a timer.
        if (pauseInFlightRef.current) {
          console.warn("F8 ignored — previous pause/resume still in flight");
          return;
        }
        pauseInFlightRef.current = true;
        try {
          // Read CURRENT status via ref to avoid the stale-closure trap that
          // used to plague this listener (registered once with [] deps).
          if (statusRef.current === "listening") {
            // Pause via F8 → status "paused" (distinct from "stopped" which
            // means tray-Quit or initial state). Lets UI show "⏸ Paused
            // (F8 to resume)" instead of generic "Stopped" — clearer user
            // mental model that the session is just suspended, not over.
            try {
              await invoke("stop_session");
              if (mountedRef.current) setStatus("paused");
            } catch (err) {
              if (!mountedRef.current) return;
              setStatus("error");
              setErrorText(String(err));
            }
          } else {
            try {
              await invoke("start_session");
              if (mountedRef.current) setStatus("listening");
            } catch (err) {
              if (!mountedRef.current) return;
              setStatus("error");
              setErrorText(String(err));
            }
          }
        } finally {
          pauseInFlightRef.current = false;
        }
      })
    );

    return () => {
      // Await each listener registration before calling its unlisten fn —
      // otherwise an early unmount can leak listeners that registered after
      // cleanup ran (the promise still resolves and the fn is never invoked).
      Promise.all(unlistens).then((fs) => fs.forEach((f) => f()));
    };
  }, []);

  const openSettings = () => invoke("open_settings");

  // Explicit branches for every Status variant — exhaustive switch makes
  // future status additions an obvious compile-target (TS will narrow `_`)
  // rather than a silent miss like "paused → gray dot" was almost.
  const dotClass = (() => {
    switch (status) {
      case "listening":
        return "dot active";
      case "thinking":
      case "answering":
        return "dot thinking";
      case "paused":
      case "stopped":
      case "error":
        return "dot"; // explicit gray — visible "session not running"
    }
  })();

  const transcriptTail = lastLines
    .slice(-2)
    .map((l) => `${l.source === "system" ? "🗣" : "🎤"} ${l.text}`)
    .join("  ");

  return (
    <div className={"overlay-root" + (compact ? " compact" : "")} ref={overlayRootRef}>
      <div
        ref={overlayBarRef}
        className="overlay-bar"
        data-tauri-drag-region
        onDoubleClick={(e) => {
          // v0.0.25: same fix as TileWindow — suppress default
          // double-click → maximize. Overlay must never go fullscreen.
          e.preventDefault();
          e.stopPropagation();
        }}
        onMouseDown={(e) => {
          // v0.0.10: explicit drag fix — same pattern as Settings header
          // (v0.0.1). CSS -webkit-app-region: drag doesn't work on Windows
          // WebView2; the attribute alone is unreliable. Explicit
          // startDragging() works because capability has allow-start-dragging.
          // Skip interactive children (buttons, inputs) so clicks aren't
          // hijacked into drags.
          if (e.button !== 0) return;
          const target = e.target as HTMLElement;
          if (target.closest("button, input, select, .no-drag, kbd")) return;
          getCurrentWindow().startDragging().catch((err) => {
            console.warn("overlay startDragging failed:", err);
          });
        }}
        title={t("overlay.drag.tip", lang)}
      >
        <div className={dotClass} aria-hidden="true" />
        <div
          className={status === "error" ? "status-text status-error" : "status-text"}
          role="status"
          aria-live="polite"
        >
          {status === "stopped" && t("overlay.status.stopped", lang)}
          {status === "paused" && t("overlay.status.paused", lang)}
          {status === "listening" && t("overlay.status.listening", lang)}
          {status === "thinking" && t("overlay.status.thinking", lang)}
          {status === "answering" && t("overlay.status.answering", lang)}
          {status === "error" && t("overlay.status.error", lang).replace("{msg}", errorText.slice(0, 60))}
        </div>
        {health && (
          <span className="health-hud" aria-label={t("overlay.health.aria", lang)}>
            {(["audio", "stt", "ai"] as const).map((k) => {
              const state = health[k];
              const ageMs = health[`${k}_age_ms` as const];
              const ageText = ageMs == null ? "—" : ageMs < 1000 ? "<1s" : `${(ageMs / 1000).toFixed(0)}s`;
              const tip = `${k.toUpperCase()}: ${state} (last ok ${ageText} ago)`;
              return (
                <span
                  key={k}
                  className={`hud-dot hud-${state}`}
                  title={tip}
                  aria-label={tip}
                />
              );
            })}
          </span>
        )}
        {coach && coach.pace !== "idle" && (
          <span
            className={`coach-pill coach-${coach.pace}`}
            title={
              `Voice coach (you, last 60s):\n` +
              `  pace: ${coach.wpm ?? "—"} wpm (${coach.pace})\n` +
              `  fillers: ${coach.fillers_60s} / ${coach.words_60s} words` +
              (coach.filler_per_100 != null ? ` (${coach.filler_per_100}/100)` : "")
            }
            aria-label={
              `Voice coach: ${coach.wpm ?? "?"} wpm, ${coach.fillers_60s} fillers in ${coach.words_60s} words`
            }
          >
            🎙 {coach.wpm ?? "?"}wpm
            {coach.filler_per_100 != null && coach.filler_per_100 > 0 && (
              <span className="coach-fillers"> · {coach.fillers_60s}ⓕ</span>
            )}
          </span>
        )}
        {hasScreenshot && <span className="hint" aria-label={t("overlay.screenshot.aria", lang)}>{t("overlay.screenshot.text", lang)}</span>}
        {aggressive && (
          <span
            className="hint"
            style={{
              color: "#fb923c",
              fontWeight: 600,
              padding: "0 4px",
              borderRadius: 4,
              background: "rgba(251, 146, 60, 0.15)",
              border: "1px solid rgba(251, 146, 60, 0.4)",
            }}
            aria-label={t("overlay.aggressive.aria", lang)}
            title={t("overlay.aggressive.tip", lang)}
          >
            🔥 aggressive
          </span>
        )}
        {rateLimited && (
          <span className="hint" style={{ color: "#facc15" }} aria-label={t("overlay.ratelimit.aria", lang)}>
            ⏱ rate-limited
          </span>
        )}
        {overBudget && (
          <span
            className="hint"
            style={{ color: "#facc15" }}
            aria-label={t("overlay.overbudget.aria", lang)}
            title={t("overlay.overbudget.tip", lang)}
          >
            💰 over budget
          </span>
        )}
        {showCost && sessionCost > 0 && (
          <span
            className="hint"
            title={t("overlay.cost.tip", lang)}
            aria-label={t("overlay.cost.aria", lang).replace("{usd}", sessionCost.toFixed(3))}
          >
            💰 ${sessionCost.toFixed(3)}
          </span>
        )}
        {hotkeyWarnings.length > 0 && (
          <span
            className="hint"
            title={t("overlay.hotkey.warn.tip", lang).replace("{warnings}", hotkeyWarnings.join("\n"))}
            style={{ color: "#facc15", cursor: "help" }}
            aria-label={t("overlay.hotkey.warn.aria", lang).replace("{n}", String(hotkeyWarnings.length))}
          >
            ⚠ {hotkeyWarnings.length}
          </span>
        )}
        {(["system", "mic"] as const).map((src) => {
          const icon = src === "system" ? "🔊" : "🎤";
          const isRec = recordingSource === src;
          const otherRec = recordingSource && recordingSource !== src;
          const label =
            askMode === "hold"
              ? isRec
                ? `${icon} ⏺ ${elapsedSec.toFixed(1)}s`
                : `${icon} ${t("overlay.ptt.hold", lang)}`
              : `${icon} ${t("overlay.ptt.ask", lang)}`;
          const ariaLabel =
            askMode === "hold"
              ? src === "system"
                ? t(isRec ? "overlay.ptt.system.aria.hold.rec" : "overlay.ptt.system.aria.hold", lang)
                : t(isRec ? "overlay.ptt.mic.aria.hold.rec" : "overlay.ptt.mic.aria.hold", lang)
              : src === "system"
                ? t("overlay.ptt.system.aria.click", lang)
                : t("overlay.ptt.mic.aria.click", lang);
          const title =
            askMode === "hold"
              ? t(src === "system" ? "overlay.ptt.system.hold" : "overlay.ptt.mic.hold", lang)
              : t(src === "system" ? "overlay.ptt.system.click" : "overlay.ptt.mic.click", lang);
          return (
            <button
              key={src}
              className={`icon-btn${isRec ? " recording" : ""}`}
              onClick={askMode === "click" ? () => clickAsk(src) : undefined}
              onMouseDown={askMode === "hold" ? () => holdStart(src) : undefined}
              onMouseUp={askMode === "hold" ? () => holdEnd(src) : undefined}
              onMouseLeave={askMode === "hold" && isRec ? () => holdEnd(src) : undefined}
              title={title}
              aria-label={ariaLabel}
              disabled={Boolean(otherRec)}
            >
              {label}
            </button>
          );
        })}
        <button
          className="hint"
          type="button"
          onClick={() => setHotkeyHelpOpen((v) => !v)}
          aria-expanded={hotkeyHelpOpen}
          aria-label={t("overlay.help.aria", lang)}
          title={t("overlay.help.tip", lang)}
          style={{
            border: "none",
            background: "transparent",
            font: "inherit",
            color: "inherit",
            cursor: "pointer",
            padding: 0,
          }}
        >F3·F4·F6·F8·F9·F10·F11&nbsp;ℹ</button>
        <button
          className="icon-btn icon-only"
          onClick={openSettings}
          title={t("overlay.gear.tip", lang)}
          aria-label={t("overlay.gear.aria", lang)}
        >
          ⚙
        </button>
      </div>

      {transcriptTail && <div className="transcript-tail">{transcriptTail}</div>}
      {answer && <div className="answer-bubble">{answer}</div>}

      {hotkeyHelpOpen && (
        <div
          role="dialog"
          aria-label={t("overlay.help.dialog.aria", lang)}
          onClick={() => setHotkeyHelpOpen(false)}
          style={{
            position: "absolute",
            top: 38,
            right: 8,
            minWidth: 320,
            padding: "10px 14px",
            background: "rgba(20, 22, 30, 0.95)",
            backdropFilter: "blur(10px)",
            border: "1px solid var(--c-border-soft)",
            borderRadius: 8,
            boxShadow: "0 8px 32px -8px rgba(0,0,0,0.6)",
            color: "var(--c-text)",
            fontSize: 12,
            lineHeight: 1.6,
            zIndex: 100,
            cursor: "default",
          }}
        >
          <div style={{ fontWeight: 600, marginBottom: 6, fontSize: 11, textTransform: "uppercase", letterSpacing: "0.05em", color: "var(--c-text-mute)" }}>
            {t("overlay.help.hk.title", lang)}
          </div>
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <tbody>
              {[
                ["F3", t("overlay.help.hk.f3", lang)],
                ["F4", t("overlay.help.hk.f4", lang)],
                ["F6", t("overlay.help.hk.f6", lang)],
                ["F8", t("overlay.help.hk.f8", lang)],
                ["F9", t("overlay.help.hk.f9", lang)],
                ["F10", t("overlay.help.hk.f10", lang)],
                ["F11", t("overlay.help.hk.f11", lang)],
                ["Ctrl+Alt+W", t("overlay.help.hk.ctrl_w", lang)],
              ].map(([key, desc]) => (
                <tr key={key}>
                  <td style={{ padding: "2px 8px 2px 0", verticalAlign: "top", width: 40 }}>
                    <kbd style={{
                      display: "inline-block",
                      padding: "1px 6px",
                      fontFamily: "monospace",
                      fontSize: 11,
                      fontWeight: 600,
                      background: "rgba(255,255,255,0.1)",
                      border: "1px solid rgba(255,255,255,0.2)",
                      borderRadius: 4,
                    }}>{key}</kbd>
                  </td>
                  <td style={{ padding: "2px 0", color: "var(--c-text)" }}>{desc}</td>
                </tr>
              ))}
            </tbody>
          </table>

          {/* v0.0.33: indicator legend per user request («нужна расшифровка
             индикаторов»). Explains the 3 HUD dots + transient chips that
             appear in the overlay bar. Hover-tooltips still exist; this
             gives the visible reference. */}
          <div style={{
            fontWeight: 600,
            margin: "10px 0 6px",
            fontSize: 11,
            textTransform: "uppercase",
            letterSpacing: "0.05em",
            color: "var(--c-text-mute)",
          }}>
            {t("overlay.help.ind.title", lang)}
          </div>
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <tbody>
              {[
                ["audio", t("overlay.help.ind.audio", lang)],
                ["stt", t("overlay.help.ind.stt", lang)],
                ["ai", t("overlay.help.ind.ai", lang)],
                ["🎙", t("overlay.help.ind.mic", lang)],
                ["📸", t("overlay.help.ind.screenshot", lang)],
                ["🔥", t("overlay.help.ind.aggressive", lang)],
                ["⏱", t("overlay.help.ind.ratelimit", lang)],
                ["💰", t("overlay.help.ind.overbudget", lang)],
                ["💰 $", t("overlay.help.ind.cost", lang)],
              ].map(([label, desc]) => (
                <tr key={label}>
                  <td style={{ padding: "2px 8px 2px 0", verticalAlign: "top", width: 40 }}>
                    <span style={{
                      display: "inline-block",
                      padding: "1px 6px",
                      fontFamily: "monospace",
                      fontSize: 11,
                      background: "rgba(255,255,255,0.08)",
                      borderRadius: 4,
                    }}>{label}</span>
                  </td>
                  <td style={{ padding: "2px 0", color: "var(--c-text)" }}>{desc}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {paletteOpen && (
        <div className="kb-palette" role="dialog" aria-label={t("overlay.palette.aria", lang)}>
          <input
            ref={paletteInputRef}
            type="text"
            className="kb-palette-input"
            value={paletteQuery}
            onChange={(e) => setPaletteQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") { e.preventDefault(); closePalette(); }
              else if (e.key === "Enter") { e.preventDefault(); void expandSelected(); }
              else if (e.key === "ArrowDown") { e.preventDefault(); setPaletteIdx((i) => Math.min(i + 1, Math.max(0, paletteResults.length - 1))); }
              else if (e.key === "ArrowUp") { e.preventDefault(); setPaletteIdx((i) => Math.max(0, i - 1)); }
            }}
            placeholder={t("overlay.palette.placeholder", lang)}
            spellCheck={false}
            autoComplete="off"
            autoCapitalize="none"
          />
          {/* v0.0.56: recent searches shown when input empty. */}
          {!paletteQuery && kbHistory.length > 0 && (
            <div className="kb-palette-history" role="region" aria-label={lang === "en" ? "Recent searches" : "Недавние поиски"}>
              <div className="kb-palette-history-label">
                {lang === "en" ? "Recent:" : "Недавнее:"}
              </div>
              <div className="kb-palette-history-chips">
                {kbHistory.map((q) => (
                  <button
                    key={q}
                    type="button"
                    className="kb-palette-history-chip"
                    onClick={() => setPaletteQuery(q)}
                    title={lang === "en" ? `Search ${q}` : `Поиск ${q}`}
                  >
                    {q}
                  </button>
                ))}
              </div>
            </div>
          )}
          {paletteResults.length > 0 && (
            <ul
              className="kb-palette-list"
              role="listbox"
              aria-label="Knowledge base search results"
            >
              {paletteResults.map((h, i) => (
                <li
                  key={h.source + ":" + h.key + ":" + i}
                  className={"kb-palette-item" + (i === paletteIdx ? " active" : "")}
                  role="option"
                  aria-selected={i === paletteIdx}
                  onMouseEnter={() => setPaletteIdx(i)}
                  onClick={() => void expandSelected()}
                >
                  <span className="kb-palette-source">{h.source}</span>
                  <kbd>{h.key}</kbd>
                  <span className="kb-palette-heading">{h.heading}</span>
                </li>
              ))}
            </ul>
          )}
          {paletteQuery && paletteResults.length === 0 && (
            <div className="kb-palette-empty" role="status" aria-live="polite">no matches for «{paletteQuery}»</div>
          )}
        </div>
      )}
    </div>
  );
}
