#!/usr/bin/env python3
"""
Fetch YouTube auto-CC transcripts for the corpus.

Three modes:

  # 1. Online via youtube-transcript-api (fails if YouTube blocks your IP)
  python fetch-transcripts.py <video_id_or_url> [<video_id_or_url> ...]

  # 2. Import a locally-downloaded subtitle file (.vtt, .srt, or plain .txt)
  python fetch-transcripts.py --from-file <path> --video-id <id>

  # 3. Glob-import all .vtt/.srt files in a directory (file stem = video ID)
  python fetch-transcripts.py --from-dir <dir>

Output: cases/raw/<video_id>/transcript.txt + transcript.json
"""

import argparse
import json
import re
import sys
from pathlib import Path


VIDEO_ID_RE = re.compile(r"(?:v=|youtu\.be/)([A-Za-z0-9_-]{11})")
# VTT/SRT timestamp matcher: HH:MM:SS.mmm or HH:MM:SS,mmm
VTT_TS_RE = re.compile(r"^(\d{2}):(\d{2}):(\d{2})[.,](\d{3})\s*-->\s*(\d{2}):(\d{2}):(\d{2})[.,](\d{3})")


def extract_id(s: str) -> str:
    """Accept either bare ID or full URL."""
    m = VIDEO_ID_RE.search(s)
    return m.group(1) if m else s


def fmt_timestamp(t: float) -> str:
    h = int(t // 3600)
    m = int((t % 3600) // 60)
    s = int(t % 60)
    return f"{h:02d}:{m:02d}:{s:02d}"


def parse_ts(h, m, s, ms) -> float:
    return int(h) * 3600 + int(m) * 60 + int(s) + int(ms) / 1000.0


def parse_vtt_or_srt(text: str) -> list[dict]:
    """Returns list of {start, duration, text}. Works for both VTT and SRT."""
    snippets = []
    lines = text.split("\n")
    i = 0
    while i < len(lines):
        line = lines[i].strip()
        m = VTT_TS_RE.match(line)
        if not m:
            i += 1
            continue
        start = parse_ts(m.group(1), m.group(2), m.group(3), m.group(4))
        end = parse_ts(m.group(5), m.group(6), m.group(7), m.group(8))
        # Collect cue text until blank line or next timestamp
        text_lines = []
        i += 1
        while i < len(lines) and lines[i].strip() and not VTT_TS_RE.match(lines[i].strip()):
            # Skip cue-position metadata and SRT cue numbers
            t = lines[i].strip()
            if not (t.isdigit() or t.startswith("NOTE") or t.startswith("STYLE")):
                # Strip <c> tags and similar VTT markup
                t = re.sub(r"<[^>]+>", "", t)
                text_lines.append(t)
            i += 1
        if text_lines:
            snippets.append({
                "start": start,
                "duration": end - start,
                "text": " ".join(text_lines),
            })
        i += 1
    return snippets


def parse_plain_youtube_transcript(text: str) -> list[dict]:
    """YouTube's UI transcript copy gives lines like:

      0:00
      Hello and welcome to today's interview

      0:05
      We'll be covering Kubernetes basics

    Some variants have timestamp inline ("[0:00] Hello..."). Be tolerant.
    """
    snippets = []
    lines = [l.rstrip() for l in text.split("\n")]
    ts_re = re.compile(r"^\[?(\d+):(\d{2})(?::(\d{2}))?\]?$")
    inline_re = re.compile(r"^\[?(\d+):(\d{2})(?::(\d{2}))?\]?\s+(.+)$")
    i = 0
    while i < len(lines):
        line = lines[i].strip()
        if not line:
            i += 1
            continue
        # Inline timestamp + text on same line
        m = inline_re.match(line)
        if m:
            h, mn, s = m.group(1), m.group(2), m.group(3)
            if s is None:
                # m:ss → 0:mm:ss
                start = int(h) * 60 + int(mn)
            else:
                start = int(h) * 3600 + int(mn) * 60 + int(s)
            snippets.append({"start": float(start), "duration": 0.0, "text": m.group(4).strip()})
            i += 1
            continue
        # Standalone timestamp; next non-empty line is text
        m = ts_re.match(line)
        if m:
            h, mn, s = m.group(1), m.group(2), m.group(3)
            if s is None:
                start = int(h) * 60 + int(mn)
            else:
                start = int(h) * 3600 + int(mn) * 60 + int(s)
            i += 1
            text_lines = []
            while i < len(lines) and lines[i].strip() and not ts_re.match(lines[i].strip()) and not inline_re.match(lines[i].strip()):
                text_lines.append(lines[i].strip())
                i += 1
            if text_lines:
                snippets.append({
                    "start": float(start),
                    "duration": 0.0,
                    "text": " ".join(text_lines),
                })
            continue
        i += 1
    # Backfill durations as gap to next snippet
    for j in range(len(snippets) - 1):
        snippets[j]["duration"] = max(0.5, snippets[j + 1]["start"] - snippets[j]["start"])
    if snippets:
        snippets[-1]["duration"] = 3.0  # rough guess for last
    return snippets


def parse_file(path: Path) -> list[dict]:
    """Auto-detect format by extension + content sniff."""
    text = path.read_text(encoding="utf-8")
    suffix = path.suffix.lower()
    if suffix in (".vtt", ".srt"):
        return parse_vtt_or_srt(text)
    if suffix == ".json":
        # Maybe already in our format
        try:
            data = json.loads(text)
            if isinstance(data, list) and data and "start" in data[0]:
                return data
        except json.JSONDecodeError:
            pass
    # Fallback: plain text from YouTube UI
    if VTT_TS_RE.search(text):
        return parse_vtt_or_srt(text)
    return parse_plain_youtube_transcript(text)


def fetch_online(video_id: str) -> list[dict]:
    """Use youtube-transcript-api if installed + IP not blocked."""
    try:
        from youtube_transcript_api import YouTubeTranscriptApi  # type: ignore
    except ImportError:
        raise RuntimeError(
            "pip install --user youtube-transcript-api  (or use --from-file)"
        )
    api = YouTubeTranscriptApi()
    transcript_list = api.list(video_id)
    # Prefer Russian (manual > auto), then English, then anything.
    candidates = []
    for t in transcript_list:
        rank = (
            0 if t.language_code == "ru" and not t.is_generated
            else 1 if t.language_code == "ru"
            else 2 if t.language_code.startswith("en") and not t.is_generated
            else 3 if t.language_code.startswith("en")
            else 4
        )
        candidates.append((rank, t))
    candidates.sort(key=lambda x: x[0])
    if not candidates:
        raise RuntimeError("no transcripts available")
    fetched = candidates[0][1].fetch()
    return [{"start": s.start, "duration": s.duration, "text": s.text} for s in fetched]


def save(video_id: str, snippets: list[dict], out_root: Path) -> dict:
    case_dir = out_root / video_id
    case_dir.mkdir(parents=True, exist_ok=True)

    # Human-readable timestamped text
    lines = [f"[{fmt_timestamp(s['start'])}] {s['text'].replace(chr(10), ' ').strip()}" for s in snippets]
    (case_dir / "transcript.txt").write_text("\n".join(lines), encoding="utf-8")
    # Raw JSON for downstream
    (case_dir / "transcript.json").write_text(
        json.dumps(snippets, ensure_ascii=False, indent=2), encoding="utf-8"
    )

    duration = int(snippets[-1]["start"] + snippets[-1]["duration"]) if snippets else 0
    return {"video_id": video_id, "snippets": len(snippets), "duration_sec": duration}


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("urls", nargs="*", help="YouTube URLs or 11-char IDs (online mode)")
    parser.add_argument("--from-file", help="Local .vtt/.srt/.txt to parse instead of fetching")
    parser.add_argument("--video-id", help="Required with --from-file: explicit video ID for output dir")
    parser.add_argument("--from-dir", help="Glob .vtt/.srt/.txt in this directory (stem = ID)")
    args = parser.parse_args()

    script_dir = Path(__file__).parent.resolve()
    out_root = script_dir.parent / "cases" / "raw"
    out_root.mkdir(parents=True, exist_ok=True)

    results = []

    if args.from_file:
        if not args.video_id:
            parser.error("--from-file requires --video-id")
        p = Path(args.from_file)
        print(f"== {args.video_id} (file: {p.name}) ==", flush=True)
        try:
            snippets = parse_file(p)
            info = save(args.video_id, snippets, out_root)
            print(f"  saved {info['snippets']} snippets ({info['duration_sec']}s)", flush=True)
            results.append(info)
        except Exception as e:
            print(f"  FAILED: {e}", file=sys.stderr)
            results.append({"video_id": args.video_id, "error": str(e)})

    elif args.from_dir:
        d = Path(args.from_dir)
        for p in sorted(d.iterdir()):
            if p.suffix.lower() not in (".vtt", ".srt", ".txt", ".json"):
                continue
            vid = p.stem
            print(f"== {vid} (file: {p.name}) ==", flush=True)
            try:
                snippets = parse_file(p)
                info = save(vid, snippets, out_root)
                print(f"  saved {info['snippets']} snippets ({info['duration_sec']}s)", flush=True)
                results.append(info)
            except Exception as e:
                print(f"  FAILED: {e}", file=sys.stderr)
                results.append({"video_id": vid, "error": str(e)})

    else:
        if not args.urls:
            parser.print_help()
            sys.exit(2)
        for url in args.urls:
            vid = extract_id(url)
            print(f"== {vid} (online) ==", flush=True)
            try:
                snippets = fetch_online(vid)
                info = save(vid, snippets, out_root)
                print(f"  saved {info['snippets']} snippets ({info['duration_sec']}s)", flush=True)
                results.append(info)
            except Exception as e:
                print(f"  FAILED: {type(e).__name__}: {str(e)[:160]}", file=sys.stderr)
                results.append({"video_id": vid, "error": f"{type(e).__name__}: {e}"})

    (out_root / "manifest.json").write_text(
        json.dumps(results, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    ok = sum(1 for r in results if "error" not in r)
    total = len(results)
    print(f"\nDone: {ok}/{total} transcripts saved to {out_root}", flush=True)
    sys.exit(0 if ok == total else 1)


if __name__ == "__main__":
    main()
