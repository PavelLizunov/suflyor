#!/usr/bin/env python3
"""Read-only source verification and transactional reconciliation checkpoints.

This utility never dispatches agents, restarts DSH, or modifies application source.
"""
import argparse
import hashlib
import json
import re
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
    portability_path = map_root / "reconciliation/source-portability.json"
    portability = load(portability_path) if portability_path.is_file() else {"entries": []}
    text_forms = {row["path"]: row for row in portability.get("entries", [])}
    accepted_git_text_forms = []
    for entry in snapshot["source_files"] + snapshot["grok_reports"]:
        path = root / entry["path"]
        if not path.is_file() and entry in snapshot["grok_reports"]:
            # Raw local reports are deliberately not published. A privacy-redacted
            # copy is a portable substitute, but never masquerades as raw bytes.
            portable = map_root / "reconciliation/grok-redaction-provenance.json"
            copies = load(portable) if portable.is_file() else []
            copy = next((row for row in copies if row["original_path"] == entry["path"] and row["original_sha256"] == entry["sha256"]), None)
            candidate = root / copy["redacted_path"] if copy else None
            if candidate and candidate.is_file() and digest(candidate) == copy["redacted_sha256"]:
                continue
            issues.append({"kind": "missing_raw_and_portable_report", "path": entry["path"]})
        elif not path.is_file():
            issues.append({"kind": "missing_input", "path": entry["path"]})
        elif digest(path) != entry["sha256"]:
            form = text_forms.get(entry["path"], {})
            if (form.get("source_commit") == snapshot["source_commit"]
                    and form.get("frozen_worktree_sha256") == entry["sha256"]
                    and digest(path) == form.get("git_blob_sha256")):
                accepted_git_text_forms.append(entry["path"])
            else:
                issues.append({"kind": "input_drift", "path": entry["path"]})
    register = load(map_root / "reconciliation/candidates.json")
    # Re-bind canonical IDs to preserved source claims on every verification.
    # The local raw report or its provenance-verified redacted copy is authoritative.
    original_claims = {}
    redaction_path = map_root / "reconciliation/grok-redaction-provenance.json"
    redaction_rows = load(redaction_path) if redaction_path.is_file() else []
    for report in snapshot["grok_reports"]:
        raw = root / report["path"]
        redacted = map_root / "reconciliation/grok-redacted" / Path(report["path"]).name
        source = redacted if redacted.is_file() else raw
        if redacted.is_file():
            proof = next((row for row in redaction_rows if row["original_path"] == report["path"] and row["original_sha256"] == report["sha256"]), None)
            if not proof or root / proof["redacted_path"] != redacted or digest(redacted) != proof["redacted_sha256"]:
                issues.append({"kind": "redacted_report_drift", "path": report["path"]})
        if not source.is_file():
            continue
        text = source.read_text(encoding="utf-8")
        headings = list(re.finditer(r"^###\s+(.*)$", text, re.MULTILINE))
        matches = list(re.finditer(r"^- \*\*Finding / Hypothesis:\*\*\s*(.*)$", text, re.MULTILINE))
        for index, match in enumerate(matches, 1):
            title = next((h.group(1) for h in reversed(headings) if h.start() < match.start()), Path(report["path"]).stem)
            original_claims[Path(report["path"]).stem + f"-C{index:02d}"] = (title, match.group(1))
    if original_claims:
        for row in register:
            original = original_claims.get(row["id"])
            canonical_text = (str(row.get("title", "")).rstrip(), str(row.get("original_claim_redacted", "")).rstrip())
            if original is None or (original[0].rstrip(), original[1].rstrip()) != canonical_text:
                issues.append({"kind": "canonical_original_claim_mismatch", "id": row["id"]})
    ids = [row["id"] for row in register]
    if len(ids) != len(set(ids)):
        issues.append({"kind": "duplicate_register_ids"})
    if len(ids) != snapshot["candidate_count"]:
        issues.append({"kind": "register_count_mismatch"})
    artifacts = []
    observed = set()
    by_id = {row["id"]: row for row in register}
    # Canonical coordinator records are distinct from failed worker proposals.
    reviewed_rows = [row for row in register if row.get("coordinator_review") == "original_claim_inspected"]
    coordinator_counts = {}
    for row in reviewed_rows:
        status = row.get("status")
        if status not in STATUSES:
            issues.append({"kind": "coordinator_invalid_status", "id": row["id"]})
        else:
            coordinator_counts[status] = coordinator_counts.get(status, 0) + 1
        if not row.get("verification") or not row.get("remaining_check") or not row.get("counterevidence"):
            issues.append({"kind": "coordinator_missing_boundary", "id": row["id"]})
        refs = row.get("source_references", [])
        if not refs:
            issues.append({"kind": "coordinator_missing_reference", "id": row["id"]})
        for ref in refs:
            rel = Path(ref["path"])
            if rel.is_absolute() or ".." in rel.parts or not (root / rel).is_file():
                issues.append({"kind": "coordinator_invalid_reference", "id": row["id"]})
                continue
            size = len((root / rel).read_text(encoding="utf-8", errors="replace").splitlines())
            start, end = ref["start_line"], ref["end_line"]
            if not isinstance(start, int) or not isinstance(end, int) or not 1 <= start <= end <= size:
                issues.append({"kind": "coordinator_reference_range", "id": row["id"]})
    coordinator_path = map_root / "reconciliation/coordinator-checks.json"
    if reviewed_rows and not coordinator_path.is_file():
        issues.append({"kind": "coordinator_missing_receipt"})
    if coordinator_path.is_file():
        coordinator = load(coordinator_path)
        checked_ids = coordinator.get("checked_original_candidate_ids", [])
        if coordinator.get("baseline") != snapshot["source_commit"] or len(checked_ids) != len(set(checked_ids)) or set(checked_ids) != {row["id"] for row in reviewed_rows}:
            issues.append({"kind": "coordinator_receipt_mismatch"})
        if coordinator.get("counts") != {status: sum(row.get("status") == status for row in register) for status in {row.get("status") for row in register}}:
            issues.append({"kind": "coordinator_count_mismatch"})
    feature_path = map_root / "features/contracts.json"
    features = load(feature_path).get("features", []) if feature_path.is_file() else []
    feature_ids = [row["id"] for row in features]
    if len(feature_ids) != len(set(feature_ids)):
        issues.append({"kind": "duplicate_feature_ids"})
    for feature in features:
        if feature.get("baseline") != snapshot["source_commit"] or feature.get("native_verification") != "not_run" or feature.get("independent_acceptance") is not False:
            issues.append({"kind": "feature_acceptance_boundary", "id": feature["id"]})
        contract_rel = Path(feature["contract"])
        if contract_rel.is_absolute() or ".." in contract_rel.parts or not (root / contract_rel).is_file():
            issues.append({"kind": "missing_feature_contract", "id": feature["id"]})
        for ref in feature.get("source_references", []):
            rel = Path(ref["path"])
            target = root / rel
            is_directory = ref.get("reference_kind") == "source_directory"
            exists = target.is_dir() if is_directory else target.is_file()
            if rel.is_absolute() or ".." in rel.parts or not exists or (is_directory and "start_line" in ref):
                issues.append({"kind": "feature_invalid_reference", "id": feature["id"]})
                continue
            if "start_line" in ref:
                size = len((root / rel).read_text(encoding="utf-8", errors="replace").splitlines())
                if not 1 <= ref["start_line"] <= ref["end_line"] <= size:
                    issues.append({"kind": "feature_reference_range", "id": feature["id"]})
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
    return {"source_commit": snapshot["source_commit"], "candidate_count": len(register), "accepted_git_text_forms": accepted_git_text_forms, "reported_candidates": len(observed), "coordinator_inspected_candidates": len(reviewed_rows), "coordinator_status_counts": coordinator_counts, "coordinator_register_sha256": digest(map_root / "reconciliation/candidates.json"), "source_feature_contracts": len(features), "feature_contracts_complete": False, "artifacts": artifacts, "issues": issues, "semantic_acceptance": "not_established_by_this_utility", "independent_acceptance": False}


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
        result["current_validation"] = inspect(root, root / "docs/agent-map")
        result["issues"] = result["current_validation"]["issues"]
    else:
        result = inspect(root, root / "docs/agent-map")
        if args.action == "checkpoint":
            checkpoint(database, result)
    print(json.dumps(result, ensure_ascii=False, indent=2))
    last_checkpoint = result.get("last_checkpoint") or {}
    if result.get("issues") or (args.action == "recover" and last_checkpoint.get("evidence", {}).get("issues")):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
