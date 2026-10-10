#!/usr/bin/env python3
"""Drive the Slint MCP server of a `--features ui-mcp` overlay-host build.

Standard library only. The server speaks JSON-RPC at http://127.0.0.1:<port>/mcp
(start the app with SLINT_MCP_PORT=<port>; build it with SLINT_EMIT_DEBUG_INFO=1).
When the app runs on another machine, forward the port first, e.g.
    ssh -N -L 19123:127.0.0.1:9123 <host>
and pass --url http://127.0.0.1:19123/mcp (or set SLINT_MCP_URL).

Commands (each prints JSON or plain lines; images are written as PNG files):
    tools                               list the server's tools
    windows                             every window: handle, position, size, visible
    tree WINDOW                         flat element list of a window (role, label, value, checked state, box)
    shot WINDOW OUT.png                 screenshot of a window (true colours)
    shot-element WINDOW QUERY OUT.png   one element, cropped from the window screenshot
    shot-elements WINDOW OUTDIR         every labelled or interactive element, one PNG each,
                                        plus index.tsv (file, role, label, value, state, x, y, w, h)
    click WINDOW QUERY                  left click on one element
    set WINDOW QUERY VALUE              set the accessible value (text input, slider, combobox)
    scroll WINDOW QUERY DY [DX]         mouse wheel over an element (needs Slint 1.18 or newer)
    key WINDOW TEXT                     type text or one key into a window
    call TOOL JSON                      any tool with raw arguments

WINDOW is a handle "index" or "index:generation" as printed by `windows`, or one of
the names bar, settings, palette, archive, transcript, help (matched by window size).
QUERY selects one element: "label=Settings", "role=Button,label=Close", "value=English",
"contains=knowledge", "checked=true", "id=App::my-button", "#12" (index in the tree listing). Add
",nth=2" when several match.

As a module: `import slint_mcp as m; m.connect(url); m.windows(); m.tree(w); ...`
"""
import base64
import json
import os
import struct
import sys
import urllib.request
import zlib

URL = os.environ.get("SLINT_MCP_URL", "http://127.0.0.1:9123/mcp")
_next_id = [0]
_ready = [False]

# Window sizes the host uses, for the short window names. (width, height); None = any.
KNOWN_WINDOWS = {
    "settings": (720, 600),
    "archive": (720, 540),
    "transcript": (720, 560),
    "palette": (520, 420),
    "help": (640, 680),
}


def connect(url=None):
    """Point the client at a server and run the MCP handshake."""
    global URL
    if url:
        URL = url
    _ready[0] = False
    _rpc("initialize", {
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name": "suflyor-slint-mcp", "version": "1"},
    })
    try:
        _rpc("notifications/initialized", {}, notify=True)
    except OSError:
        pass
    _ready[0] = True


def _rpc(method, params=None, notify=False):
    _next_id[0] += 1
    body = {"jsonrpc": "2.0", "method": method}
    if params is not None:
        body["params"] = params
    if not notify:
        body["id"] = _next_id[0]
    request = urllib.request.Request(
        URL,
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json", "Accept": "application/json, text/event-stream"},
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        raw = response.read().decode("utf-8", "replace")
    if notify or not raw.strip():
        return None
    if raw.lstrip().startswith("{"):
        return json.loads(raw)
    for line in raw.splitlines():
        if line.startswith("data:"):
            return json.loads(line[5:].strip())
    raise RuntimeError("unexpected MCP reply: " + raw[:300])


def call(tool, arguments=None):
    """Call a tool. Returns (texts, images): decoded text blocks and raw PNG byte strings."""
    if not _ready[0]:
        connect()
    reply = _rpc("tools/call", {"name": tool, "arguments": arguments or {}})
    if reply is None:
        return [], []
    if "error" in reply:
        raise RuntimeError(f"{tool}: {reply['error']}")
    result = reply.get("result", {})
    texts, images = [], []
    for block in result.get("content", []):
        if block.get("type") == "text":
            texts.append(block["text"])
        elif block.get("type") == "image":
            images.append(base64.b64decode(block["data"]))
    if result.get("isError"):
        raise RuntimeError(f"{tool}: {' '.join(texts)[:500]}")
    return texts, images


def call_json(tool, arguments=None):
    texts, _ = call(tool, arguments)
    return json.loads(texts[0]) if texts and texts[0].strip() else {}


def tools():
    if not _ready[0]:
        connect()
    return _rpc("tools/list", {})["result"]["tools"]


def _handle(window):
    return {"index": window[0], "generation": window[1]}


def window_props(window):
    return call_json("get_window_properties", {"windowHandle": _handle(window)})


def windows():
    """Every window as a dict: handle, x, y, width, height, scale, visible."""
    out = []
    for h in call_json("list_windows").get("windowHandles", []):
        w = (h["index"], h["generation"])
        try:
            p = window_props(w)
        except RuntimeError:
            continue
        pos, size = p.get("position", {}), p.get("size", {})
        x, y = pos.get("x", 0), pos.get("y", 0)
        width, height = size.get("width", 0), size.get("height", 0)
        out.append({
            "handle": w, "x": x, "y": y, "width": width, "height": height,
            "scale": p.get("scaleFactor", 1) or 1,
            # The host parks hidden windows at -32000 and keeps a 1x1 anchor window.
            "visible": width > 1 and height > 1 and x > -30000 and y > -30000,
            "root": p.get("rootElementHandle"),
        })
    return out


def find_window(spec):
    """Resolve "2", "2:1", or a name such as "settings" to a window dict."""
    all_windows = windows()
    name = spec.lower()
    if name == "bar":
        bars = [w for w in all_windows if w["visible"] and w["width"] >= 600 and w["height"] <= 200]
        if bars:
            return bars[0]
    elif name in KNOWN_WINDOWS:
        width, height = KNOWN_WINDOWS[name]
        hits = [w for w in all_windows if w["visible"] and w["width"] == width and w["height"] == height]
        if hits:
            return hits[-1]
    else:
        index, _, generation = spec.partition(":")
        hits = [w for w in all_windows if w["handle"][0] == index and (not generation or w["handle"][1] == generation)]
        if hits:
            return hits[0]
    raise SystemExit(f"no window matches {spec!r}; open windows: "
                     + ", ".join(f"{w['handle'][0]}:{w['handle'][1]} {w['width']}x{w['height']}" for w in all_windows))


def tree(window, max_elements=4000):
    """Flat element list of a window, each with role, label, value, ids and its box."""
    root = window["root"]
    if not root:
        return []
    raw = call_json("get_element_tree", {"elementHandle": root, "maxElements": max_elements}).get("elements", [])
    out = []
    for n, e in enumerate(raw):
        pos, size = e.get("absolutePosition", {}), e.get("size", {})
        names = e.get("typeNamesAndIds", [])
        out.append({
            "n": n, "handle": e["handle"],
            "role": e.get("accessibleRole") or "",
            "label": e.get("accessibleLabel") or "",
            "value": e.get("accessibleValue") or "",
            # True/False for a checkable element (toggle, current tab), None otherwise.
            "checked": bool(e.get("accessibleChecked")) if e.get("accessibleCheckable") else None,
            "type": names[0].get("typeName", "") if names else "",
            "ids": [t.get("id") for t in names if t.get("id")],
            "x": pos.get("x", 0), "y": pos.get("y", 0),
            "width": size.get("width", 0), "height": size.get("height", 0),
            "opacity": e.get("computedOpacity", 1),
        })
    return out


def select(elements, query):
    """Elements that match a QUERY (see the module help)."""
    if query.startswith("#"):
        n = int(query[1:])
        return [e for e in elements if e["n"] == n]
    want, nth = {}, None
    for part in query.split(","):
        key, _, value = part.partition("=")
        if key == "nth":
            nth = int(value)
        else:
            want[key.strip()] = value
    hits = []
    for e in elements:
        if "label" in want and e["label"] != want["label"]:
            continue
        if "role" in want and e["role"].lower() != want["role"].lower():
            continue
        if "value" in want and e["value"] != want["value"]:
            continue
        if "checked" in want and e["checked"] != (want["checked"].lower() in ("1", "true", "yes", "on")):
            continue
        if "contains" in want and want["contains"].lower() not in (e["label"] + " " + e["value"]).lower():
            continue
        if "id" in want and want["id"] not in e["ids"]:
            continue
        if "type" in want and e["type"] != want["type"]:
            continue
        hits.append(e)
    if nth is not None:
        return hits[nth - 1:nth]
    return hits


def one(window, query):
    hits = select(tree(window), query)
    if len(hits) != 1:
        raise SystemExit(f"{query!r} matches {len(hits)} elements"
                         + "".join(f"\n  #{e['n']} {e['role']} {e['label']!r} {e['value']!r}" for e in hits[:12]))
    return hits[0]


def screenshot(window):
    """PNG bytes of a window, rendered by Slint itself."""
    _, images = call("take_screenshot", {"windowHandle": _handle(window["handle"])})
    if not images:
        raise RuntimeError("take_screenshot returned no image")
    return images[0]


def click(element):
    call("click_element", {"elementHandle": element["handle"]})


def set_value(element, value):
    call("set_element_value", {"elementHandle": element["handle"], "value": value})


def scroll(element, dy, dx=0):
    """Mouse wheel over an element. The tool exists from Slint 1.18 on."""
    names = {t["name"] for t in tools()}
    if "scroll_element" not in names:
        raise SystemExit("this build's MCP server has no scroll_element tool (it needs Slint 1.18 or newer)")
    call("scroll_element", {"elementHandle": element["handle"], "deltaX": dx, "deltaY": dy})


def key(window, text):
    call("dispatch_key_event", {"windowHandle": _handle(window["handle"]), "text": text})


# --- PNG without third-party packages: decode to RGBA rows, crop, encode. ---

def png_decode(data):
    """Return (width, height, rows) with rows as bytearrays of RGBA, 4 bytes per pixel."""
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not a PNG")
    pos, idat, palette, transparency = 8, [], None, None
    width = height = depth = colour = interlace = 0
    while pos < len(data):
        length, kind = struct.unpack(">I4s", data[pos:pos + 8])
        chunk = data[pos + 8:pos + 8 + length]
        pos += 12 + length
        if kind == b"IHDR":
            width, height, depth, colour, _, _, interlace = struct.unpack(">IIBBBBB", chunk)
        elif kind == b"PLTE":
            palette = chunk
        elif kind == b"tRNS":
            transparency = chunk
        elif kind == b"IDAT":
            idat.append(chunk)
        elif kind == b"IEND":
            break
    if depth != 8 or interlace or colour not in (0, 2, 3, 4, 6):
        raise ValueError(f"unsupported PNG: depth {depth}, colour type {colour}, interlace {interlace}")
    channels = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[colour]
    stride = width * channels
    raw = zlib.decompress(b"".join(idat))
    rows, previous, offset = [], bytearray(stride), 0
    for _ in range(height):
        kind = raw[offset]
        line = bytearray(raw[offset + 1:offset + 1 + stride])
        offset += 1 + stride
        if kind == 1:
            for i in range(channels, stride):
                line[i] = (line[i] + line[i - channels]) & 255
        elif kind == 2:
            for i in range(stride):
                line[i] = (line[i] + previous[i]) & 255
        elif kind == 3:
            for i in range(stride):
                left = line[i - channels] if i >= channels else 0
                line[i] = (line[i] + ((left + previous[i]) >> 1)) & 255
        elif kind == 4:
            for i in range(stride):
                a = line[i - channels] if i >= channels else 0
                b = previous[i]
                c = previous[i - channels] if i >= channels else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        previous = line
        if colour == 6:
            rows.append(line)
            continue
        rgba = bytearray(width * 4)
        for x in range(width):
            if colour == 2:
                rgba[4 * x:4 * x + 4] = bytes(line[3 * x:3 * x + 3]) + b"\xff"
            elif colour == 0:
                rgba[4 * x:4 * x + 4] = bytes([line[x]] * 3) + b"\xff"
            elif colour == 4:
                rgba[4 * x:4 * x + 4] = bytes([line[2 * x]] * 3) + bytes([line[2 * x + 1]])
            else:
                index = line[x]
                alpha = transparency[index] if transparency and index < len(transparency) else 255
                rgba[4 * x:4 * x + 4] = bytes(palette[3 * index:3 * index + 3]) + bytes([alpha])
        rows.append(rgba)
    return width, height, rows


def png_encode(width, height, rows):
    def chunk(kind, payload):
        return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF)
    body = b"".join(b"\x00" + bytes(row) for row in rows)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(body, 6)) + chunk(b"IEND", b""))


def crop(image, x, y, width, height):
    """Crop decoded (width, height, rows) to a box in pixels, clamped to the image."""
    full_width, full_height, rows = image
    left, top = max(0, int(x)), max(0, int(y))
    right, bottom = min(full_width, int(x + width + 0.999)), min(full_height, int(y + height + 0.999))
    if right <= left or bottom <= top:
        return None
    return right - left, bottom - top, [rows[r][4 * left:4 * right] for r in range(top, bottom)]


def element_box(window, element, image_width, pad=0):
    """The element's box in screenshot pixels. Element coordinates are logical and window-relative."""
    scale = image_width / window["width"] if window["width"] else 1
    return ((element["x"] - pad) * scale, (element["y"] - pad) * scale,
            (element["width"] + 2 * pad) * scale, (element["height"] + 2 * pad) * scale)


def shot_element(window, element, path, pad=4, image=None):
    """Write one element as a PNG (with `pad` logical pixels of context). Returns False if it is off-screen."""
    image = image or png_decode(screenshot(window))
    part = crop(image, *element_box(window, element, image[0], pad))
    if not part:
        return False
    with open(path, "wb") as f:
        f.write(png_encode(*part))
    return True


INTERACTIVE = {"button", "checkbox", "switch", "slider", "combobox", "textinput", "tab", "listitem", "spinbox"}


def shot_elements(window, outdir, pad=4):
    """One PNG per labelled or interactive element of a window, plus index.tsv. Returns the row count."""
    os.makedirs(outdir, exist_ok=True)
    image = png_decode(screenshot(window))
    with open(os.path.join(outdir, "window.png"), "wb") as f:
        f.write(png_encode(*image))
    rows = []
    for e in tree(window):
        if not (e["label"] or e["value"] or e["role"].lower() in INTERACTIVE):
            continue
        if e["width"] < 1 or e["height"] < 1:
            continue
        slug = "".join(c if c.isalnum() else "-" for c in (e["label"] or e["value"] or e["type"]))[:40].strip("-")
        name = f"{e['n']:04d}-{(e['role'] or 'element').lower()}-{slug}.png"
        if shot_element(window, e, os.path.join(outdir, name), pad, image):
            state = "" if e["checked"] is None else ("checked" if e["checked"] else "unchecked")
            rows.append((name, e["role"], e["label"], e["value"], state, e["x"], e["y"], e["width"], e["height"]))
    with open(os.path.join(outdir, "index.tsv"), "w", encoding="utf-8") as f:
        f.write("file\trole\tlabel\tvalue\tstate\tx\ty\twidth\theight\n")
        for row in rows:
            f.write("\t".join(str(v).replace("\t", " ").replace("\n", " ") for v in row) + "\n")
    return len(rows)


def main(argv):
    args = list(argv)
    if "--url" in args:
        i = args.index("--url")
        connect(args[i + 1])
        del args[i:i + 2]
    if not args or args[0] in ("-h", "--help"):
        print(__doc__)
        return 0
    command, rest = args[0], args[1:]
    if command == "tools":
        for t in tools():
            properties = t.get("inputSchema", {}).get("properties", {})
            print(t["name"], "|", ", ".join(properties), "|", (t.get("description") or "").split(".")[0])
    elif command == "windows":
        for w in windows():
            print(f"{w['handle'][0]}:{w['handle'][1]}\t{w['width']}x{w['height']}\tat {w['x']},{w['y']}\t"
                  f"{'visible' if w['visible'] else 'hidden'}")
    elif command == "tree":
        for e in tree(find_window(rest[0])):
            if e["role"] or e["label"] or e["value"] or e["ids"]:
                state = "" if e["checked"] is None else ("checked" if e["checked"] else "unchecked")
                print(f"#{e['n']}\t{e['role']}\t{e['label']!r}\t{e['value']!r}\t{state}\t"
                      f"{e['x']:.0f},{e['y']:.0f} {e['width']:.0f}x{e['height']:.0f}\t{' '.join(e['ids'])}")
    elif command == "shot":
        with open(rest[1], "wb") as f:
            f.write(screenshot(find_window(rest[0])))
        print(rest[1])
    elif command == "shot-element":
        w = find_window(rest[0])
        print(rest[2] if shot_element(w, one(w, rest[1]), rest[2]) else "element is outside the window")
    elif command == "shot-elements":
        print(shot_elements(find_window(rest[0]), rest[1]), "elements written to", rest[1])
    elif command == "click":
        click(one(find_window(rest[0]), rest[1]))
    elif command == "set":
        set_value(one(find_window(rest[0]), rest[1]), rest[2])
    elif command == "scroll":
        scroll(one(find_window(rest[0]), rest[1]), float(rest[2]), float(rest[3]) if len(rest) > 3 else 0)
    elif command == "key":
        key(find_window(rest[0]), rest[1])
    elif command == "call":
        texts, images = call(rest[0], json.loads(rest[1]) if len(rest) > 1 else {})
        print("\n".join(texts) if texts else f"{len(images)} image(s)")
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
