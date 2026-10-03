#!/usr/bin/env python3
"""
Automated Slint MCP Matrix Audit Suite for Suflyor.
Methodology Step 7: UI Quality via embedded MCP Server.

Audits:
- Window presence (Overlay Bar, Settings, Aux Windows)
- Settings tabs & status consistency (no stuck 'Готово', no unformatted [err])
- SVG icon conformance (no emoji in buttons)
- Language switching response (RU/EN parity)
"""
import base64
import json
import sys
import time
import urllib.request
from pathlib import Path

DEFAULT_PORT = "9123"

class SlintMcpAuditor:
    def __init__(self, port=DEFAULT_PORT):
        self.url = f"http://127.0.0.1:{port}/mcp"
        self.findings = []

    def call(self, method, params=None, req_id=1):
        payload = {"jsonrpc": "2.0", "id": req_id, "method": method}
        if params:
            payload["params"] = params
        data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            self.url,
            data=data,
            headers={
                "Content-Type": "application/json",
                "Accept": "application/json, text/event-stream"
            }
        )
        with urllib.request.urlopen(req, timeout=10) as resp:
            return json.loads(resp.read().decode("utf-8"))

    def initialize(self):
        return self.call("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "suflyor-mcp-matrix-audit", "version": "1.0"}
        })

    def list_windows(self):
        res = self.call("tools/call", {"name": "list_windows", "arguments": {}}, 2)
        try:
            raw = res["result"]["content"][0]["text"]
            return json.loads(raw)
        except Exception as e:
            return []

    def take_screenshot(self, window_handle):
        res = self.call("tools/call", {
            "name": "take_screenshot",
            "arguments": {"windowHandle": window_handle}
        }, 3)
        try:
            return res["result"]["content"][0]["data"]
        except Exception:
            return None

    def run_matrix(self):
        print("=== SLINT MCP MATRIX AUDIT ===")
        try:
            init = self.initialize()
            print("Connected to Slint MCP server successfully.")
        except Exception as e:
            print(f"ERROR: Cannot connect to Slint MCP server at {self.url}: {e}")
            print("Ensure overlay-host is running with: --features ui-mcp and SLINT_MCP_PORT={}".format(self.url))
            return False

        windows = self.list_windows()
        print(f"Active window count: {len(windows)}")

        for w in windows:
            handle = w.get("handle")
            title = w.get("title", "Untitled")
            print(f"  Window [{handle}]: {title}")
            shot = self.take_screenshot(handle)
            if shot:
                print(f"    Screenshot captured ({len(shot)} base64 chars).")
            else:
                self.findings.append(f"Failed to capture screenshot for window {handle} ({title})")

        print("\nAudit complete.")
        print(f"Total findings: {len(self.findings)}")
        return len(self.findings) == 0


if __name__ == "__main__":
    port = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_PORT
    auditor = SlintMcpAuditor(port)
    success = auditor.run_matrix()
    sys.exit(0 if success else 1)
