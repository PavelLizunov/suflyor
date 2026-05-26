import React, { useEffect, useLayoutEffect, useRef, useState } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
// v0.0.53: highlight.js stylesheet. github-dark works well on our
// dark-themed tiles. Loaded as a CSS module so vite bundles it into
// the dist without us having to manually copy hljs CSS into styles.css.
import "highlight.js/styles/github-dark.css";
import { t, resolveLang, type Lang } from "./i18n";

export default function TileWindow() {
  // Params from URL ?tile=1&id=...&kind=...&q=...&a=...
  // SAFETY: URLSearchParams.get() already URL-decodes. We additionally
  // decodeURIComponent because the Rust backend uses a percent-encoder that
  // produces RFC 3986 strict encodings (Cyrillic etc.) — the two decoders
  // are idempotent on well-formed input. But on a malformed `%` sequence
  // (corrupted URL, future bug, attacker fuzz) decodeURIComponent throws
  // URIError and would blank the entire tile. safeDecode swallows it.
  const safeDecode = (s: string): string => {
    try { return decodeURIComponent(s); }
    catch { return s; }
  };
  const params = new URLSearchParams(window.location.search);
  const id = params.get("id") || "";
  const question = safeDecode(params.get("q") || "");
  const answerInitial = safeDecode(params.get("a") || "");
  // kind: 'auto' (detector) | 'system' (🔊) | 'mic' (🎤) | 'manual' (F6)
  // Defaults to 'auto' so old code paths still get a sensible class.
  const kindRaw = params.get("kind") || "auto";
  const kind = ["auto", "system", "mic", "manual"].includes(kindRaw) ? kindRaw : "auto";
  // v0.0.19: per-session sequence number — backend increments on each
  // spawn so the user can read tiles in chronological order even when
  // the grid is full and slots are being reused (esp. aggressive mode).
  // Old backend without seq param → undefined → don't render the badge.
  const seqRaw = params.get("seq");
  const seq = seqRaw && /^\d+$/.test(seqRaw) ? parseInt(seqRaw, 10) : null;
  // v0.0.20: comma-separated keywords to highlight (in question + answer).
  // Backend caps at 8 keywords / 150 chars; we defensively cap again
  // client-side at 12 in case URL was hand-crafted.
  const hlList: string[] = (() => {
    const raw = params.get("hl");
    if (!raw) return [];
    return safeDecode(raw)
      .split(",")
      .map((s) => s.trim())
      .filter((s) => s.length >= 2)
      .slice(0, 12);
  })();

  const [answer] = useState(answerInitial);
  const [pinned, setPinned] = useState(false);
  // v0.0.68: visual spinner state for the 🔄 reload button. Click sets it
  // true, backend closes this tile on success so unmount handles cleanup.
  // On AI error the backend keeps this tile alive but the spinner sticks
  // until the user clicks again or closes — acceptable since errors are
  // rare and the new tile that didn't spawn is obvious feedback.
  const [reloading, setReloading] = useState(false);
  // v0.0.71: collapsed state — true hides the answer body + question
  // text, leaving only the chrome (source label + pin/reload/close
  // buttons). Saves screen real estate when many tiles are open.
  // Stored per-tile in React state (not persisted); auto-resize effect
  // shrinks/restores window height accordingly.
  const [collapsed, setCollapsed] = useState(false);
  // v0.0.97: edit-question state. Click ✏️ → editingQuestion swaps
  // to a controlled string + the tile-q div renders as an input.
  // Enter triggers tile:reload-request with the edited text (reuses
  // the v0.0.68 bridge — same backend path as 🔄 reload). Esc cancels.
  const [editingQuestion, setEditingQuestion] = useState<string | null>(null);

  // v0.0.69: track tile age + reload generation. spawnedAt = mount time
  // (no backend plumbing needed; tile lifetime starts on this React
  // mount). ageStr re-computed every 5s by a setInterval — formatted
  // human-readable (12s, 1m, 3m, 1h+). gen = reload-counter passed via
  // URL param &gen=N (set by backend when tile_reload respawns); shown
  // as 🔄×N badge only if N≥1.
  const spawnedAtRef = useRef<number>(Date.now());
  const [ageStr, setAgeStr] = useState<string>("0s");
  const generationRaw = params.get("gen");
  const generation = generationRaw && /^\d+$/.test(generationRaw)
    ? Math.min(parseInt(generationRaw, 10), 99)
    : 0;
  const rootRef = useRef<HTMLDivElement>(null);
  // v0.0.48: language for tile chrome. tile-* windows have a narrow
  // capability set but get_config is allowed via assert_overlay's caller
  // check — wait, actually it isn't. Tile windows can't call get_config.
  // So we load from URL param ?lang= which the spawn_tile backend will
  // pass through starting v0.0.48. Falls back to "ru" if missing.
  const lang: Lang = resolveLang(params.get("lang"));
  // v0.0.55: tile body font size baked into URL via &fs=. Backend
  // already clamps to [11, 18] so we just parse + apply. Falls back to
  // 12 for older tiles or malformed values.
  const tileFs: number = (() => {
    const raw = params.get("fs");
    if (!raw) return 12;
    const n = parseInt(raw, 10);
    return Number.isFinite(n) && n >= 11 && n <= 18 ? n : 12;
  })();

  // Bar label by trigger source — shown uppercase via CSS text-transform.
  // Don't repeat 📌 here (the pin button is right next to it).
  // v0.0.48 i18n: the icon stays universal; only the word part gets t()'d.
  const sourceLabel =
    kind === "system" ? `🔊 ${lang === "en" ? "system" : "система"}` :
    kind === "mic"    ? `🎤 ${lang === "en" ? "mic" : "микр"}` :
    kind === "manual" ? (lang === "en" ? "manual · F6" : "вручную · F6") :
                        (lang === "en" ? "auto · detector" : "авто · детектор");

  useEffect(() => {
    document.body.classList.add("tile");
    return () => document.body.classList.remove("tile");
  }, []);

  // v0.0.82: listen for bulk collapse/expand events from the overlay
  // bar's 📦 chip. emit() broadcasts to all windows; each tile flips
  // its own collapsed state. Tiles that don't exist when the event
  // fires don't matter — new tiles spawn with collapsed=false (the
  // chip toggle re-emits when toggled again).
  useEffect(() => {
    const unlistens: Array<() => void> = [];
    listen<void>("tile:collapse-all", () => setCollapsed(true))
      .then((u) => unlistens.push(u))
      .catch(() => {});
    listen<void>("tile:expand-all", () => setCollapsed(false))
      .then((u) => unlistens.push(u))
      .catch(() => {});
    // v0.0.90: bulk pin / unpin via overlay 🔒 chip. Each tile flips
    // its pin state via the existing pin_tile Tauri command (which has
    // no assert_overlay since pin is tile-state). State follows; UI
    // updates from setPinned.
    const applyPin = async (next: boolean) => {
      try {
        await invoke("pin_tile", { label: `tile-${id}`, pinned: next });
        setPinned(next);
      } catch (err) {
        console.warn("bulk pin invoke:", err);
      }
    };
    listen<void>("tile:pin-all", () => { void applyPin(true); })
      .then((u) => unlistens.push(u))
      .catch(() => {});
    listen<void>("tile:unpin-all", () => { void applyPin(false); })
      .then((u) => unlistens.push(u))
      .catch(() => {});
    return () => { unlistens.forEach((u) => u()); };
  }, [id]);

  // v0.0.69: tick tile age every 5s. Formats: <60s as "Ns", <60m as
  // "Nm", ≥60m as "1h+". Cheap interval (one setInterval per tile
  // window, fires every 5s). React batches the setAgeStr so re-render
  // cost is negligible. Cleanup on unmount stops the timer.
  useEffect(() => {
    const format = (ms: number): string => {
      const sec = Math.max(0, Math.floor(ms / 1000));
      if (sec < 60) return `${sec}s`;
      const min = Math.floor(sec / 60);
      if (min < 60) return `${min}m`;
      return "1h+";
    };
    setAgeStr(format(Date.now() - spawnedAtRef.current));
    const handle = setInterval(() => {
      setAgeStr(format(Date.now() - spawnedAtRef.current));
    }, 5000);
    return () => clearInterval(handle);
  }, []);

  // v0.0.11: Esc closes the tile when it has focus. Useful when you've
  // mouse-overed onto a tile to read it — instead of finding the × you
  // can just press Esc. Listener at window level so Esc works regardless
  // of which inner element holds focus.
  useEffect(() => {
    const handler = async (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      try {
        const label = `tile-${id}`;
        await invoke("close_tile", { label });
      } catch {
        // Fallback: close directly. Won't run cleanup hooks but tile dies.
        try { await getCurrentWindow().close(); } catch {}
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [id]);

  // Auto-resize tile window to fit content height (within sane limits).
  // Runs after markdown renders so we measure the real DOM.
  // v0.0.71: also re-runs when `collapsed` toggles, so the window shrinks
  // to chrome-only height on collapse and expands back on uncollapse.
  useLayoutEffect(() => {
    const measure = async () => {
      const el = rootRef.current;
      if (!el) return;
      // Wait one frame so markdown has painted (or, on collapse, the body
      // has been hidden via CSS).
      await new Promise((r) => requestAnimationFrame(r));
      // v0.0.29: max W/H come from URL params (mw/mh) — Rust computed
      // these per-monitor as percentage of screen with absolute floors.
      // Fall back to the v0.0.24 hardcoded values if missing (e.g. someone
      // hand-opens the tile route).
      const params = new URLSearchParams(window.location.search);
      const maxH = Math.max(parseInt(params.get("mh") || "0", 10) || 510, 280);
      const maxW = Math.max(parseInt(params.get("mw") || "0", 10) || 460, 320);
      // v0.0.71: collapsed uses a fixed compact height that just fits the
      // chrome row (≈ 42 px). Otherwise measure scrollHeight as usual.
      const desiredH = collapsed
        ? 44
        : Math.min(Math.max(el.scrollHeight + 16, 240), maxH);
      const desiredW = maxW;
      try {
        const w = getCurrentWindow();
        await w.setSize(new LogicalSize(desiredW, desiredH));
      } catch (e) {
        console.warn("setSize:", e);
      }
    };
    measure();
  }, [answer, collapsed]);

  const close = async () => {
    try {
      const label = `tile-${id}`;
      await invoke("close_tile", { label });
    } catch {
      // Fallback: close current window directly.
      const w = getCurrentWindow();
      await w.close();
    }
  };

  const togglePin = async () => {
    const next = !pinned;
    try {
      await invoke("pin_tile", { label: `tile-${id}`, pinned: next });
      setPinned(next);
    } catch (e) {
      console.warn("pin_tile:", e);
    }
  };

  // v0.0.68: 🔄 reload — re-ask the same question, get fresh answer.
  // Tile windows can't call tile_reload directly (assert_overlay), so we
  // emit a Tauri event to ALL windows; Overlay listens + invokes the
  // backend command. On success the backend closes THIS tile (and spawns
  // a new one), so React unmount handles cleanup. On AI failure the tile
  // stays alive with the spinner — user can click × to close or try
  // again.
  const reload = async () => {
    if (reloading) return;
    setReloading(true);
    try {
      // v0.0.69: pass currentGeneration so backend can bump it for the
      // respawned tile (renders as 🔄×N+1 in chrome).
      await emit("tile:reload-request", {
        label: `tile-${id}`,
        question,
        currentGeneration: generation,
      });
    } catch (e) {
      console.warn("tile reload emit:", e);
      setReloading(false);
    }
  };

  // v0.0.89: 🌐 translate — re-ask in opposite language. Same bridge
  // pattern as reload (emit event → overlay invokes assert_overlay-
  // gated cmd). Reuses `reloading` spinner state since both actions
  // close + respawn this tile.
  const translate = async () => {
    if (reloading) return;
    setReloading(true);
    try {
      await emit("tile:translate-request", {
        label: `tile-${id}`,
        question,
      });
    } catch (e) {
      console.warn("tile translate emit:", e);
      setReloading(false);
    }
  };

  // v0.0.20: build a single regex out of the highlight list once per
  // render — used to split text nodes and wrap matches in <mark>.
  // Escape regex special chars per keyword. Case-insensitive. \b only
  // works on ASCII so for Cyrillic/mixed words we use lookaround on
  // \w (which JS treats as [A-Za-z0-9_] — still imperfect for Cyrillic
  // but better than substring-only matches). Plain string match anywhere
  // is fine for short keywords like "k8s".
  const hlRegex = (() => {
    if (hlList.length === 0) return null;
    const escaped = hlList.map((k) => k.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
    try {
      return new RegExp(`(${escaped.join("|")})`, "gi");
    } catch { return null; }
  })();

  const renderWithHighlights = (text: string): React.ReactNode => {
    if (!hlRegex) return text;
    const parts = text.split(hlRegex);
    if (parts.length === 1) return text;
    return parts.map((p, i) =>
      i % 2 === 1
        ? <mark key={i} className="tile-hl">{p}</mark>
        : <React.Fragment key={i}>{p}</React.Fragment>
    );
  };

  // v0.0.54: CodeBlock wrapper — adds a hover-visible "📋 Copy" button
  // overlay on each pre. Click extracts text content + writes to
  // navigator.clipboard. Brief "✓" feedback. The wrapper preserves
  // the existing pre+code structure so rehype-highlight's class
  // attribution still works.
  const CodeBlock = ({ children }: { children?: React.ReactNode }) => {
    const preRef = useRef<HTMLPreElement>(null);
    const [copied, setCopied] = useState(false);
    const onCopy = async () => {
      const text = preRef.current?.innerText ?? "";
      if (!text) return;
      try {
        await navigator.clipboard.writeText(text);
        setCopied(true);
        setTimeout(() => setCopied(false), 1200);
      } catch (err) {
        console.warn("clipboard write failed:", err);
      }
    };
    return (
      <div className="tile-code-wrap">
        <pre ref={preRef}>{children}</pre>
        <button
          type="button"
          className="tile-code-copy"
          onClick={onCopy}
          aria-label={lang === "en" ? "Copy code block" : "Скопировать код"}
          title={copied
            ? (lang === "en" ? "✓ Copied" : "✓ Скопировано")
            : (lang === "en" ? "Copy" : "Скопировать")}
        >
          {copied ? "✓" : "📋"}
        </button>
      </div>
    );
  };

  // Markdown component override: walk text nodes and highlight.
  // Only override what we need (p, li, h*, em, strong, code stay markdown).
  // v0.0.54: also override `pre` for the copy-button wrapper.
  const markdownComponents = {
    ...(hlRegex ? {
      text: ({ children }: { children?: React.ReactNode }) => {
        if (typeof children === "string") return <>{renderWithHighlights(children)}</>;
        return <>{children}</>;
      },
    } : {}),
    pre: CodeBlock,
  };

  return (
    <div
      ref={rootRef}
      className={`tile-root tile-kind-${kind}`}
      role="dialog"
      aria-label={`AI answer tile from ${sourceLabel}`}
      style={{ "--tile-font-size": `${tileFs}px` } as React.CSSProperties}
    >
      <div
        className="tile-bar"
        data-tauri-drag-region
        onDoubleClick={(e) => {
          // v0.0.25: SUPPRESS Tauri's default double-click → maximize
          // behaviour on drag regions. User reported: double-clicking a
          // tile «выделяет его, остальные блокируются». Root cause:
          // double-click maximizes the tile to full-screen, covering all
          // other tiles AND making them unreachable until you click off.
          // Stop the event before Tauri's drag-region handler sees it.
          e.preventDefault();
          e.stopPropagation();
        }}
      >
        {seq !== null && (
          <span
            className="tile-seq"
            title={lang === "en" ? `Tile #${seq} in this session` : `Тайл #${seq} в этой сессии`}
            aria-label={`Tile sequence number ${seq}`}
            style={{
              display: "inline-block",
              padding: "1px 6px",
              marginRight: 6,
              fontSize: 10,
              fontWeight: 700,
              fontFamily: "monospace",
              borderRadius: 8,
              background: "rgba(255,255,255,0.15)",
              color: "rgba(255,255,255,0.85)",
              userSelect: "none",
            }}
          >
            #{seq}
          </span>
        )}
        <span className="tile-source" title={sourceLabel}>{sourceLabel}</span>
        {/* v0.0.69: age + reload generation badges. Age updates every 5s
            via setInterval. Generation badge only renders when ≥1 (i.e.
            this tile is the result of a 🔄 reload). Both tiny + dimmed
            so they don't compete with the source label for visual
            attention. Title attrs give the full explanation on hover. */}
        <span
          className="tile-age"
          title={lang === "en" ? `Tile age (since spawn): ${ageStr}` : `Возраст тайла (с момента появления): ${ageStr}`}
          style={{
            marginLeft: 6,
            fontSize: 10,
            fontFamily: "monospace",
            opacity: 0.55,
            userSelect: "none",
            letterSpacing: "0.5px",
          }}
        >
          ⏱{ageStr}
        </span>
        {generation > 0 && (
          <span
            className="tile-gen"
            title={lang === "en"
              ? `This tile is reload #${generation} of the original question`
              : `Этот тайл — перезапрос #${generation} исходного вопроса`}
            style={{
              marginLeft: 4,
              fontSize: 10,
              fontFamily: "monospace",
              padding: "1px 5px",
              borderRadius: 6,
              background: "rgba(255, 180, 80, 0.18)",
              color: "rgba(255, 200, 120, 0.95)",
              userSelect: "none",
            }}
          >
            🔄×{generation}
          </span>
        )}
        {/* v0.0.94: answer word count. Helps you tell at a glance
            whether AI gave a one-liner or a deep dive without expanding
            the tile body. Strips markdown formatting first so the
            count reflects actual prose.
            v0.0.96 P2 fix: preserve hyphenated words ("self-employed"
            stays 1 word). Strip leading ordered-list markers ("1.")
            per line so they don't count. */}
        {(() => {
          const stripped = answer
            .replace(/```[\s\S]*?```/g, "")     // drop code blocks
            .replace(/`[^`]*`/g, "")            // drop inline code
            .replace(/^\s*\d+\.\s+/gm, " ")     // drop ordered-list markers
            .replace(/[*_#>]/g, " ");           // drop md punct (preserve -)
          const words = stripped.split(/\s+/).filter(Boolean).length;
          if (words === 0) return null;
          return (
            <span
              className="tile-words"
              title={lang === "en"
                ? `Answer length: ${words} words (excluding code)`
                : `Длина ответа: ${words} слов (без кода)`}
              style={{
                marginLeft: 4,
                fontSize: 10,
                fontFamily: "monospace",
                opacity: 0.5,
                userSelect: "none",
              }}
            >
              {words}w
            </span>
          );
        })()}
        <button
          className="tile-pin"
          data-pinned={pinned ? "true" : undefined}
          onClick={togglePin}
          title={pinned ? t("tile.unpin.tip", lang) : t("tile.pin.tip", lang)}
          aria-label={pinned ? t("tile.unpin.aria", lang) : t("tile.pin.aria", lang)}
          aria-pressed={pinned}
        >
          📌
        </button>
        {/* v0.0.71: ▾/▴ collapse toggle. Collapsed = body+question hidden,
            tile height shrunk to chrome only (≈44 px). Useful when you
            want to keep a tile visible as a reference but reclaim screen
            real estate for the meeting itself. Pin status preserved
            across collapse so reaper still respects it. */}
        <button
          className="tile-collapse"
          onClick={() => setCollapsed((v) => !v)}
          title={collapsed
            ? (lang === "en" ? "Expand tile" : "Развернуть тайл")
            : (lang === "en" ? "Collapse tile (body hides, only chrome stays)" : "Свернуть тайл (тело скрыто, остаётся только заголовок)")}
          aria-label={collapsed
            ? (lang === "en" ? "Expand tile" : "Развернуть тайл")
            : (lang === "en" ? "Collapse tile" : "Свернуть тайл")}
          aria-pressed={collapsed}
          style={{
            background: "transparent",
            border: "none",
            cursor: "pointer",
            opacity: 0.85,
            padding: "0 6px",
            fontSize: 13,
            color: "rgba(255,255,255,0.85)",
          }}
        >
          {collapsed ? "▴" : "▾"}
        </button>
        {/* v0.0.68: 🔄 reload — re-ask same question. Backend closes
            this tile + spawns new one with fresh answer. Spinner during
            in-flight; click guarded by `reloading` state to prevent
            double-spawn. */}
        <button
          className="tile-reload"
          onClick={reload}
          disabled={reloading}
          title={reloading
            ? (lang === "en" ? "Reloading…" : "Перезапрос…")
            : (lang === "en" ? "Re-ask this question (fresh AI answer)" : "Переспросить (новый ответ AI)")}
          aria-label={lang === "en" ? "Reload tile" : "Перезапросить тайл"}
          style={{
            background: "transparent",
            border: "none",
            cursor: reloading ? "wait" : "pointer",
            opacity: reloading ? 0.5 : 0.85,
            padding: "0 6px",
            fontSize: 13,
          }}
        >
          {reloading ? "⏳" : "🔄"}
        </button>
        {/* v0.0.97: ✏️ edit question — opens inline input for the
            tile's question, Enter re-asks with the edited text via
            the v0.0.68 reload bridge (passes the edited question
            payload instead of the original). Esc cancels. */}
        <button
          className="tile-edit-q"
          onClick={() => {
            if (reloading) return;
            setEditingQuestion(question);
          }}
          disabled={reloading}
          title={lang === "en"
            ? "Edit question and re-ask (Enter to submit, Esc to cancel)"
            : "Изменить вопрос и переспросить (Enter — отправить, Esc — отменить)"}
          aria-label={lang === "en" ? "Edit question" : "Изменить вопрос"}
          style={{
            background: "transparent",
            border: "none",
            cursor: reloading ? "wait" : "pointer",
            opacity: reloading ? 0.5 : 0.85,
            padding: "0 6px",
            fontSize: 13,
          }}
        >
          ✏️
        </button>
        {/* v0.0.93: 📋 copy question to clipboard. Useful for pasting
            into Slack/email/notes. Confirmation: button briefly shows
            "✓" for 1.2s. Pure frontend, no backend. */}
        <CopyQuestionButton question={question} lang={lang} />
        {/* v0.0.89: 🌐 translate — re-ask in opposite language. Same
            bridge pattern as 🔄 reload (event → overlay → backend cmd).
            Disabled together with reload via shared `reloading` state. */}
        <button
          className="tile-translate"
          onClick={translate}
          disabled={reloading}
          title={lang === "en"
            ? "Translate: re-ask in the opposite language"
            : "Перевести: переспросить на противоположном языке"}
          aria-label={lang === "en" ? "Translate tile" : "Перевести тайл"}
          style={{
            background: "transparent",
            border: "none",
            cursor: reloading ? "wait" : "pointer",
            opacity: reloading ? 0.5 : 0.85,
            padding: "0 6px",
            fontSize: 13,
          }}
        >
          🌐
        </button>
        <button
          className="tile-close"
          onClick={close}
          title={t("tile.close.tip", lang)}
          aria-label={t("tile.close.aria", lang)}
        >
          ×
        </button>
      </div>
      {/* v0.0.71: hide question + body when collapsed. CSS display:none
          via inline style — simpler than a conditional render and keeps
          the React tree (and hljs class attributions) stable so toggling
          back is instant with no reflow flash.
          v0.0.97: swap to input when editingQuestion !== null. Enter
          re-asks with the edited text via tile:reload-request. Esc
          cancels and restores question display. */}
      {editingQuestion !== null ? (
        <input
          autoFocus
          type="text"
          value={editingQuestion}
          onChange={(e) => setEditingQuestion(e.target.value)}
          onKeyDown={async (e) => {
            if (e.key === "Escape") {
              setEditingQuestion(null);
              return;
            }
            if (e.key === "Enter") {
              const edited = editingQuestion.trim();
              if (!edited) return;
              setEditingQuestion(null);
              if (reloading) return;
              setReloading(true);
              try {
                await emit("tile:reload-request", {
                  label: `tile-${id}`,
                  question: edited,
                  currentGeneration: generation,
                });
              } catch (err) {
                console.warn("tile edit reload emit:", err);
                setReloading(false);
              }
            }
          }}
          onBlur={() => setEditingQuestion(null)}
          style={collapsed ? { display: "none" } : {
            width: "100%",
            boxSizing: "border-box",
            padding: "6px 8px",
            fontSize: 13,
            fontWeight: 500,
            background: "rgba(255,255,255,0.06)",
            border: "1px solid rgba(255,255,255,0.2)",
            borderRadius: 4,
            color: "inherit",
            outline: "none",
          }}
        />
      ) : (
        <div
          className="tile-q"
          title={question}
          style={collapsed ? { display: "none" } : undefined}
        >
          {renderWithHighlights(question)}
        </div>
      )}
      <div
        className="tile-body markdown"
        role="region"
        aria-label="AI answer body"
        style={collapsed ? { display: "none" } : undefined}
      >
        <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          rehypePlugins={[[rehypeHighlight, { detect: true, ignoreMissing: true }]]}
          components={markdownComponents}
        >
          {answer}
        </ReactMarkdown>
      </div>
    </div>
  );
}

// v0.0.93: small button — copies tile question text to clipboard via
// navigator.clipboard.writeText. Brief ✓ feedback for 1.2s. Defined
// outside the main TileWindow to keep its render loop lean (it owns
// its own copied state). Compact styling matches reload/translate
// buttons.
function CopyQuestionButton({ question, lang }: { question: string; lang: Lang }) {
  // v0.0.96 P2 fix: track failure state so button shows ✗ on rejected
  // clipboard write (denied permission / secure-context issue) instead
  // of looking dead. ok / fail labels distinct in tooltip.
  const [status, setStatus] = useState<"idle" | "ok" | "fail">("idle");
  const onClick = async () => {
    try {
      await navigator.clipboard.writeText(question);
      setStatus("ok");
      setTimeout(() => setStatus("idle"), 1200);
    } catch (e) {
      console.warn("clipboard write:", e);
      setStatus("fail");
      setTimeout(() => setStatus("idle"), 2000);
    }
  };
  const icon = status === "ok" ? "✓" : status === "fail" ? "✗" : "📋";
  const title = status === "ok"
    ? (lang === "en" ? "✓ Copied" : "✓ Скопировано")
    : status === "fail"
      ? (lang === "en" ? "✗ Clipboard write failed (permission?)" : "✗ Не удалось записать в буфер")
      : (lang === "en" ? "Copy question to clipboard" : "Скопировать вопрос в буфер");
  return (
    <button
      className="tile-copy-q"
      onClick={onClick}
      title={title}
      aria-label={lang === "en" ? "Copy question" : "Скопировать вопрос"}
      style={{
        background: "transparent",
        border: "none",
        cursor: "pointer",
        opacity: 0.85,
        padding: "0 6px",
        fontSize: 13,
        color: status === "fail" ? "#d05050" : undefined,
      }}
    >
      {icon}
    </button>
  );
}
