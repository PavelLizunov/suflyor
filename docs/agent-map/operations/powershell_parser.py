"""Hash-verified portable PowerShell parser-only route; input AST never executed."""
import hashlib
import json
import os
import subprocess
from pathlib import Path

VERSION = "7.4.13"


def parser_command(source, label):
    configured = os.environ.get("SUFLYOR_RESEARCH_POWERSHELL")
    if not configured:
        raise FileNotFoundError("SUFLYOR_RESEARCH_POWERSHELL must name verified ignored portable runtime")
    base = Path(configured).resolve()
    receipt = json.loads(Path(__file__).resolve().parents[1].joinpath("reconciliation/powershell-parser-provenance.json").read_text())
    for relative, expected in receipt["runtime_manifest"].items():
        part = Path(relative)
        if part.is_absolute() or ".." in part.parts:
            raise ValueError("unsafe runtime manifest path")
        file = base / part
        if file.is_symlink() or hashlib.sha256(file.read_bytes()).hexdigest() != expected:
            raise ValueError("PowerShell runtime hash mismatch: " + relative)
    command = [str(base / "pwsh"), "-NoLogo", "-NoProfile", "-NonInteractive", "-File",
               str(Path(__file__).with_name("powershell_worker.ps1").resolve()),
               "-SourceFile", str(source.resolve()), "-SourceLabel", label]
    return command


def parse(source, label):
    try:
        command = parser_command(source, label)
    except (OSError, ValueError, KeyError):
        return {"declarations": [], "parse_errors": [{"kind": "canonical_runtime_unavailable_or_hash_mismatch"}], "has_parse_error": True, "native_parser_failure": True}
    env = dict(os.environ, POWERSHELL_TELEMETRY_OPTOUT="1", POWERSHELL_UPDATECHECK="Off", DOTNET_CLI_TELEMETRY_OPTOUT="1")
    try:
        result = subprocess.run(command, env=env, text=True, capture_output=True, timeout=30)
    except (OSError, subprocess.TimeoutExpired):
        return {"declarations": [], "parse_errors": [{"kind": "canonical_parser_process_unavailable_or_timeout"}], "has_parse_error": True, "native_parser_failure": True}
    if result.returncode:
        return {"declarations": [], "parse_errors": [{"kind": "canonical_parser_exit", "returncode": result.returncode}], "has_parse_error": True, "native_parser_failure": True}
    try:
        receipt = json.loads(result.stdout)
        if receipt["script_executed"] is not False or not isinstance(receipt["declarations"], list) or not isinstance(receipt["parse_errors"], list) or not isinstance(receipt["has_parse_error"], bool):
            raise ValueError("invalid receipt")
        return receipt
    except (ValueError, KeyError, TypeError):
        return {"declarations": [], "parse_errors": [{"kind": "invalid_canonical_parser_receipt"}], "has_parse_error": True, "native_parser_failure": True}
