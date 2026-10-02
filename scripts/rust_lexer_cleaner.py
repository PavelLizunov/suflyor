#!/usr/bin/env python3
"""Robust character-level Rust lexer for stripping narrative comments while
strictly preserving doc comments (///, //!) and critical safety invariants.
Correctly handles multiline strings, raw strings (r#"..."#), and nested block comments.
"""
import sys
from pathlib import Path

INVARIANT_KEYWORDS = [
    "jobobject", "wda", "stealth", "affinity", "single-flight", "deadlock", "invariant",
    "security", "mutex", "lock", "thread", "onnx", "isolate", "panic", "unwrap", "license",
    "atomic", "win32", "race", "crash", "dos", "leak", "secret", "token", "directml",
    "wasapi", "exclusive", "sandbox", "credential", "dpapi", "elevation"
]

def is_invariant(text: str) -> bool:
    lower = text.lower()
    return any(k in lower for k in INVARIANT_KEYWORDS)

def strip_narrative_comments(src: str) -> tuple[str, dict]:
    stats = {
        "doc_comments": 0,
        "invariants_kept": 0,
        "narrative_stripped": 0,
    }

    n = len(src)
    i = 0
    out = []

    while i < n:
        # Check raw string r#"..."#
        if (src[i] == 'r' or src[i:i+2] == 'br' or src[i:i+2] == 'cr') and i + 1 < n:
            start_i = i
            if src[i:i+2] in ('br', 'cr'):
                i += 2
            else:
                i += 1
            hashes = 0
            while i < n and src[i] == '#':
                hashes += 1
                i += 1
            if i < n and src[i] == '"':
                i += 1
                delim = '"' + ('#' * hashes)
                end = src.find(delim, i)
                if end != -1:
                    i = end + len(delim)
                    out.append(src[start_i:i])
                    continue
                else:
                    out.append(src[start_i:])
                    break
            else:
                i = start_i

        # Normal string literal "..."
        if src[i] == '"' and (i == 0 or src[i-1] != '\\' or src[i-2:i] == '\\\\'):
            start_i = i
            i += 1
            while i < n:
                if src[i] == '\\':
                    i += 2
                elif src[i] == '"':
                    i += 1
                    break
                else:
                    i += 1
            out.append(src[start_i:i])
            continue

        # Character literal '...'
        if src[i] == "'" and i + 2 < n:
            # Avoid lifetime like 'a or 'static
            if src[i+1] == '\\' and i + 3 < n and src[i+3] == "'":
                out.append(src[i:i+4])
                i += 4
                continue
            elif src[i+2] == "'":
                out.append(src[i:i+3])
                i += 3
                continue

        # Doc comment line: /// or //!
        if src[i:i+3] in ("///", "//!"):
            stats["doc_comments"] += 1
            line_end = src.find("\n", i)
            if line_end == -1:
                out.append(src[i:])
                break
            else:
                out.append(src[i:line_end+1])
                i = line_end + 1
                continue

        # Block comment: /* ... */ (nested in Rust)
        if src[i:i+2] == "/*":
            start_i = i
            i += 2
            depth = 1
            while i < n and depth > 0:
                if src[i:i+2] == "/*":
                    depth += 1
                    i += 2
                elif src[i:i+2] == "*/":
                    depth -= 1
                    i += 2
                else:
                    i += 1
            comment_text = src[start_i:i]
            if is_invariant(comment_text):
                stats["invariants_kept"] += 1
                out.append(comment_text)
            else:
                stats["narrative_stripped"] += 1
                # Replace with single space if inside code
                out.append(" ")
            continue

        # Line comment: //
        if src[i:i+2] == "//":
            start_i = i
            line_end = src.find("\n", i)
            if line_end == -1:
                comment_text = src[start_i:]
                next_i = n
                has_nl = False
            else:
                comment_text = src[start_i:line_end]
                next_i = line_end + 1
                has_nl = True

            if is_invariant(comment_text):
                stats["invariants_kept"] += 1
                out.append(comment_text)
                if has_nl:
                    out.append("\n")
            else:
                stats["narrative_stripped"] += 1
                # Check if this comment was the entire line
                # Look back in out to see if only whitespace since last newline
                tail = "".join(out[-10:])
                last_nl = tail.rfind("\n")
                prefix = tail[last_nl+1:] if last_nl != -1 else tail
                if prefix.strip() == "":
                    # Full line comment: drop the whitespace prefix and the newline
                    while out and out[-1] != "\n":
                        if out[-1].strip() == "":
                            out.pop()
                        else:
                            break
                    # do not append newline
                else:
                    # Trailing comment on code line: keep newline
                    if has_nl:
                        out.append("\n")
            i = next_i
            continue

        # Normal character
        out.append(src[i])
        i += 1

    return "".join(out), stats


def process_all(dry_run=True):
    root = Path(".")
    crates = ["overlay-backend", "slint-experiment", "suflyor-tts", "suflyor-teratts", "suflyor-wsola"]

    total_stats = {"doc_comments": 0, "invariants_kept": 0, "narrative_stripped": 0}
    modified_files = 0

    for crate in crates:
        files = list((root / crate).rglob("*.rs"))
        for p in files:
            if "target" in p.parts:
                continue
            src = p.read_text(errors='ignore')
            cleaned, s = strip_narrative_comments(src)
            for k in total_stats:
                total_stats[k] += s[k]
            if cleaned != src:
                modified_files += 1
                if not dry_run:
                    p.write_text(cleaned)

    print(f"Mode: {'DRY-RUN' if dry_run else 'APPLIED'}")
    print(f"Files modified: {modified_files}")
    print(f"Doc comments preserved       : {total_stats['doc_comments']}")
    print(f"Safety invariants preserved  : {total_stats['invariants_kept']}")
    print(f"Narrative comments stripped  : {total_stats['narrative_stripped']}")


if __name__ == "__main__":
    dry_run = "--apply" not in sys.argv
    process_all(dry_run=dry_run)
