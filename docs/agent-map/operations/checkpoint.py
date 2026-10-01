#!/usr/bin/env python3
"""Read-only source verification and transactional reconciliation checkpoints.

This utility never dispatches agents, restarts DSH, or modifies application source.
"""
import argparse
import hashlib
import json
import sqlite3
from pathlib import Path

STATUSES = {"confirmed", "hypothesis", "rejected", "duplicate", "obsolete", "unresolved"}
LANES = {
    "data": ("wave1_",),
    "audio": ("wave2_",),
    "ui": ("wave3_",),
    "security-build": ("wave4_",),
}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def inspect(root, map_root):
    snapshot = load(map_root / "reconciliation/snapshot.json")
    issues = []
    for entry in snapshot["source_files"] + snapshot["grok_reports"]:
        path = root / entry["path"]
        if not path.is_file():
            issues.append({"kind": "missing_input", "path": entry["path"]})
        elif digest(path) != entry["sha256"]:
            issues.append({"kind": "input_drift", "path": entry["path"]})
    register = load(map_root / "reconciliation/candidates.json")
    ids = [row["id"] for row in register]
    if len(ids) != len(set(ids)):
        issues.append({"kind": "duplicate_register_ids"})
    if len(ids) != snapshot["candidate_count"]:
        issues.append({"kind": "register_count_mismatch"})
    artifacts = []
    observed = set()
    by_id = {row["id"]: row for row in register}
    for lane, prefixes in LANES.items():
        path = map_root / "reconciliation" / (lane + ".json")
        expected = {row["id"] for row in register if any(Path(row["report"]).name.startswith(p) for p in prefixes)}
        if not path.is_file():
            artifacts.append({"lane": lane, "state": "pending", "expected": len(expected)})
            continue
        try:
            report = load(path)
            rows = report["candidates"]
            actual = [row["id"] for row in rows]
            errors = []
            if report.get("baseline") != snapshot["source_commit"]:
                errors.append("baseline_mismatch")
            if report.get("lane") != lane:
                errors.append("lane_mismatch")
            if set(actual) != expected or len(actual) != len(set(actual)):
                errors.append("candidate_ids_mismatch")
            for row in rows:
                if row.get("status") not in STATUSES:
                    errors.append("invalid_status:" + row["id"])
                if not row.get("verification") or not row.get("remaining_check"):
                    errors.append("missing_verification_boundary:" + row["id"])
                refs = row.get("source_references", [])
                if row.get("status") in {"confirmed", "rejected", "obsolete"} and not refs:
                    errors.append("missing_source_evidence:" + row["id"])
                for ref in refs:
                    rel = Path(ref["path"])
                    if rel.is_absolute() or ".." in rel.parts:
                        errors.append("unsafe_reference:" + row["id"])
                        continue
                    src = root / rel
                    if not src.is_file():
                        errors.append("missing_reference:" + row["id"])
                        continue
                    size = len(src.read_text(encoding="utf-8", errors="replace").splitlines())
                    start, end = ref["start_line"], ref["end_line"]
                    if not isinstance(start, int) or not isinstance(end, int) or not 1 <= start <= end <= size:
                        errors.append("out_of_range_reference:" + row["id"])
            semantic_warnings = []
            for row in rows:
                original = by_id.get(row["id"], {})
                if original.get("original_claim_redacted") is not None and row.get("original_claim_redacted") != original["original_claim_redacted"]:
                    semantic_warnings.append("original_claim_mismatch:" + row["id"])
                if original.get("title") is not None and row.get("title") != original["title"]:
                    semantic_warnings.append("original_title_mismatch:" + row["id"])
            refused = report.get("transport_status", "").startswith("final_result_null") or bool(semantic_warnings)
            state = "invalid" if errors else ("unaccepted_proposal" if refused else "structurally_valid")
            artifacts.append({"lane": lane, "state": state, "sha256": digest(path), "candidates": len(rows), "errors": errors, "semantic_warnings": semantic_warnings})
            observed.update(actual)
            issues.extend({"kind": "report_validation", "lane": lane, "detail": e} for e in errors)
        except (ValueError, KeyError, TypeError) as exc:
            artifacts.append({"lane": lane, "state": "invalid", "error": str(exc)})
            issues.append({"kind": "report_parse", "lane": lane})
    return {"source_commit": snapshot["source_commit"], "candidate_count": len(register), "reported_candidates": len(observed), "artifacts": artifacts, "issues": issues, "semantic_acceptance": "not_established_by_this_utility"}


def checkpoint(db_path, report):
    db_path.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(db_path, timeout=30) as db:
        db.execute("PRAGMA journal_mode=WAL")
        db.execute("PRAGMA synchronous=FULL")
        db.executescript("""
            CREATE TABLE IF NOT EXISTS checkpoints (
                id INTEGER PRIMARY KEY,
                source_commit TEXT NOT NULL,
                evidence TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS reviewed_artifacts (
                lane TEXT PRIMARY KEY,
                sha256 TEXT NOT NULL,
                state TEXT NOT NULL CHECK(state = 'structurally_valid'),
                source_commit TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS attempts (
                id TEXT PRIMARY KEY,
                lane TEXT NOT NULL,
                job_id TEXT,
                status TEXT NOT NULL CHECK(status IN ('running','settled','unknown'))
            );
        """)
        db.execute("BEGIN IMMEDIATE")
        db.execute("INSERT INTO checkpoints(source_commit,evidence) VALUES (?,?)", (report["source_commit"], json.dumps(report, sort_keys=True)))
        # Drift/invalid output does not replace prior accepted structural evidence.
        if not report["issues"]:
            for artifact in report["artifacts"]:
                if artifact["state"] == "structurally_valid":
                    db.execute("INSERT INTO reviewed_artifacts VALUES (?,?,?,?) ON CONFLICT(lane) DO UPDATE SET sha256=excluded.sha256,state=excluded.state,source_commit=excluded.source_commit", (artifact["lane"], artifact["sha256"], artifact["state"], report["source_commit"]))
        db.commit()


def recover(db_path):
    if not db_path.is_file():
        return {"state": "no_checkpoint", "automatic_dispatch": False}
    with sqlite3.connect(db_path) as db:
        # Unknown native job outcomes must be reconciled through DSH job tools.
        db.execute("UPDATE attempts SET status='unknown' WHERE status='running'")
        last = db.execute("SELECT source_commit,evidence FROM checkpoints ORDER BY id DESC LIMIT 1").fetchone()
        artifacts = db.execute("SELECT lane,sha256,state,source_commit FROM reviewed_artifacts ORDER BY lane").fetchall()
        attempts = db.execute("SELECT id,lane,job_id,status FROM attempts ORDER BY id").fetchall()
    return {"last_checkpoint": None if last is None else {"source_commit": last[0], "evidence": json.loads(last[1])}, "preserved_artifacts": artifacts, "attempts": attempts, "automatic_dispatch": False, "next_action": "Check current source/report hashes; reconcile unknown job IDs with DSH before redispatch."}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["verify", "checkpoint", "recover"])
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--database", type=Path)
    args = parser.parse_args()
    root = args.root.resolve()
    database = args.database or root / ".campaign-state/reconciliation.sqlite"
    if args.action == "recover":
        result = recover(database)
    else:
        result = inspect(root, root / "docs/agent-map")
        if args.action == "checkpoint":
            checkpoint(database, result)
    print(json.dumps(result, ensure_ascii=False, indent=2))
    if result.get("issues"):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
