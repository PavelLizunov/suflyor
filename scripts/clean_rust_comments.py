#!/usr/bin/env python3
"""Block-aware, invariant-preserving Rust comment cleaner.
Groups contiguous full-line comments into semantic blocks.
Preserves the whole block if any line contains a safety invariant.
Strips dead commented code and narrative clutter without breaking multi-line sentences.
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

def clean_rust_file(content: str) -> tuple[str, dict]:
    stats = {
        "doc_comments": 0,
        "invariant_blocks_kept": 0,
        "narrative_blocks_dropped": 0,
        "trailing_comments_dropped": 0,
        "lines_saved": 0,
    }

    lines = content.splitlines(keepends=True)
    out_lines = []

    i = 0
    n = len(lines)

    in_block_comment = False

    while i < n:
        line = lines[i]
        stripped = line.strip()

        # 1. Blank lines
        if not stripped:
            out_lines.append(line)
            i += 1
            continue

        # 2. Block comment continuation
        if in_block_comment:
            out_lines.append(line)
            if "*/" in line:
                in_block_comment = False
            i += 1
            continue

        # 3. Block comment start
        if stripped.startswith("/*"):
            out_lines.append(line)
            if "*/" not in line:
                in_block_comment = True
            i += 1
            continue

        # 4. Doc comments (///, //!) -> always keep intact
        if stripped.startswith("///") or stripped.startswith("//!"):
            stats["doc_comments"] += 1
            out_lines.append(line)
            i += 1
            continue

        # 5. Full-line comment (//) -> collect contiguous comment block
        if stripped.startswith("//"):
            block_lines = [line]
            j = i + 1
            while j < n:
                next_stripped = lines[j].strip()
                if next_stripped.startswith("//") and not (next_stripped.startswith("///") or next_stripped.startswith("//!")):
                    block_lines.append(lines[j])
                    j += 1
                else:
                    break

            # Check if any line in block is an invariant
            block_text = "".join(block_lines)
            if is_invariant(block_text):
                stats["invariant_blocks_kept"] += 1
                out_lines.extend(block_lines)
            else:
                stats["narrative_blocks_dropped"] += 1
                stats["lines_saved"] += len(block_lines)

            i = j
            continue

        # 6. Code line with potential trailing comment
        # Find // outside string literals and raw strings
        cut_pos = None
        in_str = False
        in_char = False
        escape = False
        idx = 0
        while idx < len(line):
            ch = line[idx]
            if escape:
                escape = False
                idx += 1
                continue
            if ch == '\\' and (in_str or in_char):
                escape = True
                idx += 1
                continue
            if ch == '"' and not in_char:
                in_str = not in_str
                idx += 1
                continue
            if ch == "'" and not in_str:
                in_char = not in_char
                idx += 1
                continue
            if not in_str and not in_char and idx + 1 < len(line) and line[idx:idx+2] == '//':
                cut_pos = idx
                break
            idx += 1

        if cut_pos is not None:
            code_part = line[:cut_pos]
            comment_part = line[cut_pos+2:].strip()
            if is_invariant(comment_part):
                out_lines.append(line)
            else:
                stats["trailing_comments_dropped"] += 1
                nl = "\r\n" if line.endswith("\r\n") else "\n"
                out_lines.append(code_part.rstrip() + nl)
        else:
            out_lines.append(line)

        i += 1

    return "".join(out_lines), stats


def process_all(dry_run=True):
    root = Path(".")
    crates = ["overlay-backend", "slint-experiment", "suflyor-tts", "suflyor-teratts", "suflyor-wsola"]

    total_stats = {
        "doc_comments": 0,
        "invariant_blocks_kept": 0,
        "narrative_blocks_dropped": 0,
        "trailing_comments_dropped": 0,
        "lines_saved": 0,
    }
    modified_files = 0

    for crate in crates:
        files = list((root / crate).rglob("*.rs"))
        for p in files:
            if "target" in p.parts:
                continue
            src = p.read_text(errors='ignore')
            cleaned, s = clean_rust_file(src)
            for k in total_stats:
                total_stats[k] += s[k]
            if cleaned != src:
                modified_files += 1
                if not dry_run:
                    p.write_text(cleaned)

    print(f"Mode: {'DRY-RUN' if dry_run else 'APPLIED'}")
    print(f"Files modified: {modified_files}")
    print(f"Doc comments preserved          : {total_stats['doc_comments']}")
    print(f"Invariant blocks kept           : {total_stats['invariant_blocks_kept']}")
    print(f"Narrative comment blocks dropped: {total_stats['narrative_blocks_dropped']}")
    print(f"Trailing comments dropped       : {total_stats['trailing_comments_dropped']}")
    print(f"Total lines saved               : {total_stats['lines_saved']}")


if __name__ == "__main__":
    dry_run = "--apply" not in sys.argv
    process_all(dry_run=dry_run)
