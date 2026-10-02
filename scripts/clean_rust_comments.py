#!/usr/bin/env python3
"""Rust comment lexer and invariant-preserving sanitizer.
Preserves string literals, doc comments (///, //!) and safety invariants.
Strips dead commented code and narrative clutter.
"""
import re
import sys
from pathlib import Path

INVARIANT_KEYWORDS = [
    "jobobject", "wda", "stealth", "affinity", "single-flight", "deadlock", "invariant",
    "security", "mutex", "lock", "thread", "onnx", "isolate", "panic", "unwrap", "license",
    "atomic", "win32", "race", "crash", "dos", "leak", "secret", "token", "directml",
    "wasapi", "exclusive", "sandbox", "credential", "dpapi", "elevation"
]

def is_invariant_comment(comment_text: str) -> bool:
    lower = comment_text.lower()
    return any(k in lower for k in INVARIANT_KEYWORDS)

def sanitize_rust_source(source: str) -> tuple[str, dict]:
    stats = {
        "doc_comments_preserved": 0,
        "invariant_comments_preserved": 0,
        "comments_removed": 0,
        "lines_saved": 0
    }
    
    lines = source.splitlines(keepends=True)
    out_lines = []
    
    in_block_comment = False
    block_comment_buffer = []
    
    for line in lines:
        stripped = line.strip()
        
        # Preserve blank lines
        if not stripped:
            out_lines.append(line)
            continue
            
        # Doc comments are always preserved
        if stripped.startswith("///") or stripped.startswith("//!"):
            stats["doc_comments_preserved"] += 1
            out_lines.append(line)
            continue
            
        # Full-line // comment
        if stripped.startswith("//"):
            comment_content = stripped[2:].strip()
            if is_invariant_comment(comment_content):
                stats["invariant_comments_preserved"] += 1
                out_lines.append(line)
            else:
                stats["comments_removed"] += 1
                stats["lines_saved"] += 1
                # Drop narrative comment line
                continue
                
        # Trailing line comment on code line
        # Use regex to find trailing comment outside string literals
        # Simple scan:
        in_string = False
        raw_string = False
        char_lit = False
        escape = False
        cut_pos = None
        
        i = 0
        while i < len(line):
            ch = line[i]
            if escape:
                escape = False
                i += 1
                continue
            if ch == '\\' and (in_string or char_lit):
                escape = True
                i += 1
                continue
            if ch == '"' and not char_lit:
                in_string = not in_string
                i += 1
                continue
            if ch == "'" and not in_string:
                char_lit = not char_lit
                i += 1
                continue
            if not in_string and not char_lit and i + 1 < len(line) and line[i:i+2] == '//':
                # Found comment
                cut_pos = i
                break
            i += 1
            
        if cut_pos is not None:
            code_part = line[:cut_pos]
            comment_part = line[cut_pos+2:].strip()
            if is_invariant_comment(comment_part):
                stats["invariant_comments_preserved"] += 1
                out_lines.append(line)
            else:
                stats["comments_removed"] += 1
                # Keep code part with original trailing newline
                nl = "\r\n" if line.endswith("\r\n") else "\n"
                out_lines.append(code_part.rstrip() + nl)
        else:
            out_lines.append(line)
            
    return "".join(out_lines), stats


def dry_run_repo():
    root = Path(".")
    crates = ["overlay-backend", "slint-experiment", "suflyor-tts", "suflyor-teratts", "suflyor-wsola"]
    
    total_stats = {
        "doc_comments_preserved": 0,
        "invariant_comments_preserved": 0,
        "comments_removed": 0,
        "lines_saved": 0
    }
    
    for crate in crates:
        files = list((root / crate).rglob("*.rs"))
        crate_stats = {
            "doc_comments_preserved": 0,
            "invariant_comments_preserved": 0,
            "comments_removed": 0,
            "lines_saved": 0
        }
        for p in files:
            if "target" in p.parts:
                continue
            content = p.read_text(errors='ignore')
            _, s = sanitize_rust_source(content)
            for k in crate_stats:
                crate_stats[k] += s[k]
                total_stats[k] += s[k]
        print(f"{crate:18}: removed {crate_stats['comments_removed']:4} noise lines | kept {crate_stats['invariant_comments_preserved']:4} invariant lines | kept {crate_stats['doc_comments_preserved']:4} doc lines")
        
    print(f"\nTOTAL ACROSS 5 CRATES:")
    print(f"  Narrative comments to remove   : {total_stats['comments_removed']} lines")
    print(f"  Safety invariants preserved    : {total_stats['invariant_comments_preserved']} lines")
    print(f"  Doc comments (///, //!) kept   : {total_stats['doc_comments_preserved']} lines")


if __name__ == "__main__":
    dry_run_repo()
