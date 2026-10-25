#!/usr/bin/env python3
"""Build omarchy-plugin/locales/ru.json from the Slint translation catalog.

The QML front end keeps the English msgids of the .slint files, so one catalog
serves both. Entries without a translation are left out (the msgid shows).
Usage: tools/remote-ui/po2json.py [--check]
"""
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
PO = ROOT / "slint-experiment/translations/ru/LC_MESSAGES/slint-replay.po"
OUT = ROOT / "omarchy-plugin/locales/ru.json"


def unquote(line):
    return json.loads(line)  # .po string literals use the same escapes as JSON here


def parse(text):
    entries, key, field = {}, None, None
    current = {"msgid": "", "msgstr": ""}

    def flush():
        if current["msgid"] and current["msgstr"]:
            entries[current["msgid"]] = current["msgstr"]

    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("msgctxt") or line.startswith("msgid_plural"):
            raise SystemExit(f"unsupported .po entry: {line}")
        if line.startswith("msgid "):
            flush()
            current = {"msgid": unquote(line[6:]), "msgstr": ""}
            field = "msgid"
        elif line.startswith("msgstr "):
            current["msgstr"] = unquote(line[7:])
            field = "msgstr"
        elif line.startswith('"') and field:
            current[field] += unquote(line)
    flush()
    return entries


def main():
    entries = parse(PO.read_text(encoding="utf-8"))
    text = json.dumps(entries, ensure_ascii=False, indent=1, sort_keys=True) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUT} is stale; run tools/remote-ui/po2json.py")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(OUT, len(entries), "entries")


if __name__ == "__main__":
    main()
