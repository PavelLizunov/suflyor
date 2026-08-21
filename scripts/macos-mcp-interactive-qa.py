#!/usr/bin/env python3
"""
Automated Slint MCP Visual & Interactive QA Suite for Suflyor macOS Port.

Prerequisites:
  Build overlay-host with `--features ui-mcp` and run with `SLINT_MCP_PORT=9124`.

Usage:
  python3 scripts/macos-mcp-interactive-qa.py [mcp_port]
"""

import base64
import json
import sys
import time
import urllib.request

PORT = sys.argv[1] if len(sys.argv) > 1 else "9124"
MCP_URL = f"http://127.0.0.1:{PORT}/mcp"


def call_mcp(method, params=None, id_num=1):
    payload = {"jsonrpc": "2.0", "id": id_num, "method": method}
    if params:
        payload["params"] = params
    req = urllib.request.Request(
        MCP_URL,
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            "Accept": "application/json, text/event-stream",
        },
    )
    with urllib.request.urlopen(req) as resp:
        return json.loads(resp.read().decode("utf-8"))


def list_window_handles():
    res = call_mcp("tools/call", {"name": "list_windows", "arguments": {}}, 2)
    win_text = res["result"]["content"][0]["text"]
    return json.loads(win_text)["windowHandles"]


def capture_all_windows(label_prefix):
    handles = list_window_handles()
    print(f"[{label_prefix}] Total active windows: {len(handles)}")
    saved = []
    for idx, handle in enumerate(handles):
        shot = call_mcp(
            "tools/call",
            {"name": "take_screenshot", "arguments": {"windowHandle": handle}},
            100 + idx,
        )
        content = shot.get("result", {}).get("content", [])
        for item in content:
            if item.get("type") == "image" or "data" in item:
                b64_data = item.get("data", "")
                if b64_data:
                    filename = f"/tmp/mcp_qa_{label_prefix}_win{idx+1}.png"
                    with open(filename, "wb") as f:
                        f.write(base64.b64decode(b64_data))
                    print(f"  Saved screenshot: {filename} ({len(b64_data)} b64 bytes)")
                    saved.append(filename)
    return saved


def main():
    print("=== Suflyor macOS MCP Live QA Suite ===")
    init_res = call_mcp(
        "initialize",
        {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "suflyor-mac-qa", "version": "1"},
        },
        1,
    )
    print("MCP Initialized:", init_res.get("result", {}).get("serverInfo"))

    main_win = list_window_handles()[0]
    win_props = call_mcp(
        "tools/call",
        {
            "name": "get_window_properties",
            "arguments": {"windowHandle": main_win},
        },
        2,
    )
    root_handle = json.loads(win_props["result"]["content"][0]["text"])[
        "rootElementHandle"
    ]

    tree_res = call_mcp(
        "tools/call",
        {
            "name": "get_element_tree",
            "arguments": {"elementHandle": root_handle, "maxElements": 150},
        },
        3,
    )
    tree = json.loads(tree_res["result"]["content"][0]["text"])

    touch_map = {}
    for elem in tree.get("elements", []):
        types = [t.get("typeName", "") for t in elem.get("typeNamesAndIds", [])]
        if "TouchArea" in types:
            x = int(elem.get("absolutePosition", {}).get("x", 0))
            touch_map[x] = elem["handle"]

    print("\nDiscovered Overlay Bar TouchAreas:", touch_map)

    # 1. Capture Main Overlay Bar
    print("\n1. Capturing Main Overlay Bar...")
    capture_all_windows("01_overlay_bar")

    # 2. Click Settings (x=829)
    if 829 in touch_map:
        print("\n2. Clicking Settings chip...")
        call_mcp(
            "tools/call",
            {
                "name": "click_element",
                "arguments": {"elementHandle": touch_map[829]},
            },
            10,
        )
        time.sleep(1)
        capture_all_windows("02_settings")

    # 3. Click Archive (x=594)
    if 594 in touch_map:
        print("\n3. Clicking Archive chip...")
        call_mcp(
            "tools/call",
            {
                "name": "click_element",
                "arguments": {"elementHandle": touch_map[594]},
            },
            11,
        )
        time.sleep(1)
        capture_all_windows("03_archive")

    # 4. Click Help (x=865)
    if 865 in touch_map:
        print("\n4. Clicking Help chip...")
        call_mcp(
            "tools/call",
            {
                "name": "click_element",
                "arguments": {"elementHandle": touch_map[865]},
            },
            12,
        )
        time.sleep(1)
        capture_all_windows("04_help")

    # 5. Click Ask (x=684)
    if 684 in touch_map:
        print("\n5. Clicking Ask chip...")
        call_mcp(
            "tools/call",
            {
                "name": "click_element",
                "arguments": {"elementHandle": touch_map[684]},
            },
            13,
        )
        time.sleep(1)
        capture_all_windows("05_ask")

    print("\n=== QA Suite Completed Successfully ===")


if __name__ == "__main__":
    main()
