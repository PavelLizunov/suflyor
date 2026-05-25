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
    <div ref={rootRef} className={`tile-root tile-kind-${kind}`}>
      <div className="tile-bar" data-tauri-drag-region>
        <span className="tile-source" title={sourceLabel}>{sourceLabel}</span>
        <button
          className="tile-close"
          data-pinned={pinned ? "true" : undefined}
          onClick={togglePin}
          title={pinned ? "Pinned — no auto-close" : "Pin (cancel auto-close)"}
        >
          📌
        </button>
        <button
          className="tile-close"
          onClick={close}
          title="Close now"
        >
          ×
        </button>
      </div>
      <div className="tile-q" title={question}>{question}</div>
      <div className="tile-body markdown">
        <ReactMarkdown remarkPlugins={[remarkGfm]}>{answer}</ReactMarkdown>
      </div>
    </div>
  );
}
