import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

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

  const [answer] = useState(answerInitial);
  const [pinned, setPinned] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  // Bar label by trigger source — shown uppercase via CSS text-transform.
  // Don't repeat 📌 here (the pin button is right next to it).
  const sourceLabel =
    kind === "system" ? "🔊 system" :
    kind === "mic"    ? "🎤 mic" :
    kind === "manual" ? "manual · F6" :
                        "auto · detector";

  useEffect(() => {
    document.body.classList.add("tile");
    return () => document.body.classList.remove("tile");
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
  useLayoutEffect(() => {
    const measure = async () => {
      const el = rootRef.current;
      if (!el) return;
      // Wait one frame so markdown has painted.
      await new Promise((r) => requestAnimationFrame(r));
      // Cap at 400 — MUST match tile.rs TILE_H_MAX. Anything taller
      // makes the next-row tile overlap on a 1080p monitor.
      const desiredH = Math.min(Math.max(el.scrollHeight + 16, 220), 400);
      const desiredW = 380;
      try {
        const w = getCurrentWindow();
        await w.setSize(new LogicalSize(desiredW, desiredH));
      } catch (e) {
        console.warn("setSize:", e);
      }
    };
    measure();
  }, [answer]);

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

  return (
    <div
      ref={rootRef}
      className={`tile-root tile-kind-${kind}`}
      role="dialog"
      aria-label={`AI answer tile from ${sourceLabel}`}
    >
      <div className="tile-bar" data-tauri-drag-region>
        {seq !== null && (
          <span
            className="tile-seq"
            title={`Тайл #${seq} в этой сессии`}
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
        <button
          className="tile-close"
          data-pinned={pinned ? "true" : undefined}
          onClick={togglePin}
          title={pinned ? "Pinned — no auto-close" : "Pin (cancel auto-close)"}
          aria-label={pinned ? "Unpin tile (re-enable auto-close)" : "Pin tile (disable auto-close)"}
          aria-pressed={pinned}
        >
          📌
        </button>
        <button
          className="tile-close"
          onClick={close}
          title="Close now"
          aria-label="Close tile"
        >
          ×
        </button>
      </div>
      <div className="tile-q" title={question}>{question}</div>
      <div className="tile-body markdown" role="region" aria-label="AI answer body">
        <ReactMarkdown remarkPlugins={[remarkGfm]}>{answer}</ReactMarkdown>
      </div>
    </div>
  );
}
