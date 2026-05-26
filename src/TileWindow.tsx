import React, { useEffect, useLayoutEffect, useRef, useState } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
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
      // v0.0.29: max W/H come from URL params (mw/mh) — Rust computed
      // these per-monitor as percentage of screen with absolute floors.
      // Fall back to the v0.0.24 hardcoded values if missing (e.g. someone
      // hand-opens the tile route).
      const params = new URLSearchParams(window.location.search);
      const maxH = Math.max(parseInt(params.get("mh") || "0", 10) || 510, 280);
      const maxW = Math.max(parseInt(params.get("mw") || "0", 10) || 460, 320);
      const desiredH = Math.min(Math.max(el.scrollHeight + 16, 240), maxH);
      const desiredW = maxW;
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
        <button
          className="tile-close"
          onClick={close}
          title={t("tile.close.tip", lang)}
          aria-label={t("tile.close.aria", lang)}
        >
          ×
        </button>
      </div>
      <div className="tile-q" title={question}>{renderWithHighlights(question)}</div>
      <div className="tile-body markdown" role="region" aria-label="AI answer body">
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
