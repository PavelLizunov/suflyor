#!/usr/bin/env python3
"""Smoke test of suflyord without a shell: connect like the plugin, check the bar
arrives, open Settings, and save every message to a log.

Usage: smoke.py <socket> <log.jsonl>
"""
import json
import socket
import sys
import time

SOCK, LOG = sys.argv[1], sys.argv[2]


def connect(deadline):
    while time.time() < deadline:
        try:
            s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            s.connect(SOCK)
            return s
        except OSError:
            time.sleep(0.5)
    raise SystemExit(f"suflyord did not open {SOCK}")


def main():
    s = connect(time.time() + 120)
    s.settimeout(1.0)
    log = open(LOG, "w", encoding="utf-8")
    buf = b""
    windows = {}
    globals_ = {}

    def pump(seconds, until=None):
        nonlocal buf
        end = time.time() + seconds
        while time.time() < end:
            try:
                chunk = s.recv(65536)
            except socket.timeout:
                chunk = b""
            if chunk == b"" and not buf:
                if until and until():
                    return True
                continue
            buf += chunk
            while b"\n" in buf:
                line, buf = buf.split(b"\n", 1)
                log.write(line.decode() + "\n")
                m = json.loads(line)
                if m["t"] == "open":
                    windows[m["win"]] = {"type": m["type"], "visible": m.get("visible"), "props": dict(m.get("props", {}))}
                elif m["t"] == "prop" and m["win"] in windows:
                    windows[m["win"]]["props"][m["name"]] = m["value"]
                elif m["t"] in ("show", "hide") and m["win"] in windows:
                    windows[m["win"]]["visible"] = m["t"] == "show"
                elif m["t"] == "global":
                    globals_[m["global"] + "." + m["name"]] = m["value"]
                elif m["t"] == "close":
                    windows.pop(m["win"], None)
            if until and until():
                return True
        return False

    def send(m):
        s.sendall((json.dumps(m) + "\n").encode())

    send({"t": "hello", "shell": "smoke", "locale": "en"})
    bar = lambda: next((k for k, w in windows.items() if w["type"] == "OverlayBarWindow"), None)
    if not pump(60, bar):
        raise SystemExit("no OverlayBarWindow after 60 s")
    pump(5)
    b = windows[bar()]
    print("bar:", bar(), "visible", b["visible"], "status", repr(b["props"].get("status-text")))

    send({"t": "call", "win": bar(), "name": "open-settings-clicked", "args": [], "id": 1})
    settings = lambda: next((k for k, w in windows.items() if w["type"] == "SettingsWindow" and w["visible"]), None)
    if not pump(30, settings):
        raise SystemExit("Settings did not open after open-settings-clicked")
    pump(3)
    st = windows[settings()]["props"]
    print("settings:", settings(), "tab", st.get("active-tab"), "props", len(st))

    print("windows:", sorted({w["type"] for w in windows.values()}))
    if globals_.get("Suflyor.locale") not in ("ru", "en"):
        raise SystemExit(f"no interface language from suflyord: {globals_.get('Suflyor.locale')!r}")
    print("locale:", globals_["Suflyor.locale"])
    log.close()


if __name__ == "__main__":
    main()
