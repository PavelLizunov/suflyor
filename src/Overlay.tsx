import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

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
  // v0.0.21: visible hotkey legend popover open/closed state.
  const [hotkeyHelpOpen, setHotkeyHelpOpen] = useState(false);
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
  const paletteOpenRef = useRef(paletteOpen);
  useEffect(() => { paletteOpenRef.current = paletteOpen; }, [paletteOpen]);
  useEffect(() => {
    if (!overlayRootRef.current) return;
    const ro = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      if (paletteOpenRef.current) return; // palette has its own size logic
      // contentRect doesn't include border. We add a small margin for
      // padding + safety. Clamp to sane bounds.
      const desiredW = Math.min(Math.max(Math.ceil(entry.contentRect.width) + 30, 520), 1200);
      const desiredH = Math.min(Math.max(Math.ceil(entry.contentRect.height) + 4, 96), 900);
      getCurrentWindow().outerSize().then((sz) => {
        getCurrentWindow().scaleFactor().then((scale) => {
          const currentW = Math.round(sz.width / scale);
          const currentH = Math.round(sz.height / scale);
          // Only resize on > 4px delta on either axis to dampen flicker
          // and to ignore sub-pixel measurement noise.
          if (Math.abs(currentW - desiredW) > 4 || Math.abs(currentH - desiredH) > 4) {
            getCurrentWindow().setSize(new LogicalSize(desiredW, desiredH))
              .catch((err) => console.warn("overlay autoresize:", err));
          }
        }).catch(() => {});
      }).catch(() => {});
    });
    ro.observe(overlayRootRef.current);
    return () => ro.disconnect();
  }, []);

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
  // enabled between sessions; without a visible reminder cost can creep
  // up unexpectedly.
  const [aggressive, setAggressive] = useState(false);

  // Load ask-mode + aggressive flag from config once on mount.
  useEffect(() => {
    invoke<{ manual_ask_mode?: string; auto_tile_every_line?: boolean }>("get_config")
      .then((c) => {
        if (!mountedRef.current) return;
        const mode = c.manual_ask_mode === "click" ? "click" : "hold";
        setAskMode(mode);
        setAggressive(Boolean(c.auto_tile_every_line));
      })
      .catch((e) => console.warn("get_config:", e));
  }, []);

  // Re-read aggressive flag when Settings window closes (user may have
  // toggled it). Use the same window-focus event we already listen to
  // for the Settings stale-state heal pattern.
  useEffect(() => {
    const onFocus = () => {
      if (!mountedRef.current) return;
      invoke<{ auto_tile_every_line?: boolean }>("get_config")
        .then((c) => mountedRef.current && setAggressive(Boolean(c.auto_tile_every_line)))
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
    <div className="overlay-root" ref={overlayRootRef}>
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
        title="Перетащи за пустую область бара, чтобы подвинуть overlay"
      >
        <div className={dotClass} aria-hidden="true" />
        <div
          className={status === "error" ? "status-text status-error" : "status-text"}
          role="status"
          aria-live="polite"
        >
          {status === "stopped" && "Stopped"}
          {status === "paused" && "⏸ Paused (F8 to resume)"}
          {status === "listening" && "Listening"}
          {status === "thinking" && "Asking AI…"}
          {status === "answering" && "Answering"}
          {status === "error" && `Error: ${errorText.slice(0, 60)}`}
        </div>
        {health && (
          <span className="health-hud" aria-label="Subsystem health">
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
        {hasScreenshot && <span className="hint" aria-label="Screenshot ready">📸 ready</span>}
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
            aria-label="Aggressive mode is enabled — tile spawns on every transcript line"
            title="🔥 AGGRESSIVE MODE ON — тайл на КАЖДУЮ строку транскрипта. AI cost растёт быстро (~$4-5/час непрерывной речи). Отключить: Settings → 🪟 Auto-tiles → снять галку «спавнить тайл на каждую строку»"
          >
            🔥 aggressive
          </span>
        )}
        {rateLimited && (
          <span className="hint" style={{ color: "#facc15" }} aria-label="Rate limited">
            ⏱ rate-limited
          </span>
        )}
        {overBudget && (
          <span
            className="hint"
            style={{ color: "#facc15" }}
            aria-label="Session cost over configured budget"
            title="Сессия превысила Soft budget warning (Settings → AI proxy). AI продолжает работать — это passive notice."
          >
            💰 over budget
          </span>
        )}
        {showCost && sessionCost > 0 && (
          <span
            className="hint"
            title="Accumulated session cost (Claude tokens) — toggle in Settings → UI"
            aria-label={`Session cost ${sessionCost.toFixed(3)} dollars`}
          >
            💰 ${sessionCost.toFixed(3)}
          </span>
        )}
        {hotkeyWarnings.length > 0 && (
          <span
            className="hint"
            title={`Hotkey issues:\n${hotkeyWarnings.join("\n")}`}
            style={{ color: "#facc15", cursor: "help" }}
            aria-label={`${hotkeyWarnings.length} hotkey warning(s)`}
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
                : `${icon} hold`
              : `${icon} ask`;
          const ariaLabel =
            askMode === "hold"
              ? `${src === "system" ? "System" : "Microphone"} push-to-talk${isRec ? " — recording" : ""}`
              : `Ask AI about recent ${src === "system" ? "system" : "microphone"} lines`;
          const title =
            askMode === "hold"
              ? `Зажми чтобы записать ${src === "system" ? "СОБЕСЕДНИКА" : "ПОЛЬЗОВАТЕЛЯ"}, отпусти чтобы спросить AI`
              : `Спросить AI про последние реплики ${src === "system" ? "СОБЕСЕДНИКА" : "ПОЛЬЗОВАТЕЛЯ"}`;
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
          aria-label="Hotkey legend — click to expand"
          title="Click для расшифровки всех hotkey'ев"
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
          title="Settings"
          aria-label="Open settings"
        >
          ⚙
        </button>
      </div>

      {transcriptTail && <div className="transcript-tail">{transcriptTail}</div>}
      {answer && <div className="answer-bubble">{answer}</div>}

      {hotkeyHelpOpen && (
        <div
          role="dialog"
          aria-label="Hotkey reference"
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
            Hotkeys (global) — click anywhere to close
          </div>
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <tbody>
              {[
                ["F3", "Reask — повторить последний вопрос со свежим контекстом"],
                ["F4", "KB palette — поиск в knowledge base (1643 entries)"],
                ["F6", "Manual tile — спавнить тайл из последней реплики"],
                ["F8", "Pause / Resume — пауза/возобновить сессию"],
                ["F9", "Ask AI — спросить AI сейчас (со screenshot если есть)"],
                ["F10", "Screenshot — захват для следующего F9"],
                ["F11", "PANIC HIDE — скрыть overlay + все тайлы"],
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
        </div>
      )}

      {paletteOpen && (
        <div className="kb-palette" role="dialog" aria-label="Knowledge base search">
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
            placeholder="KB search: kubernetes / dijkstra / iptables …   (Esc to close, Enter to expand)"
            spellCheck={false}
            autoComplete="off"
            autoCapitalize="none"
          />
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
