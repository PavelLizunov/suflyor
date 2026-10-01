# Frozen config/UI/translation/assets inventory

[Machine-readable inventory](inventory.json) is declaration/resource navigation at baseline `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`, **not a complete config-consumer/callback graph, evaluated defaults, translation-quality audit, compiler bundle or native UI acceptance**. Every inspected input is source-hash linked; no live config, model, user log or credential read.

## Measured scope

- 83 frozen inputs: Config source, Russian PO, 23 host-directory Slint sources (including replay/spike exports) and 58 non-document asset files. Two experimental Slint files are out of this inventory; retained in syntax index.
- 84 `Config` fields: types, Serde attributes, 30 helper-default links and 84 `Config::defaults()` initializer source ranges. Literal initializer values/comments are deliberately **not copied**; private default endpoints never enter generated research. Derived `Default`, Serde field helpers and explicit `defaults()` remain distinct, not evaluated.
- 2,440 production Slint navigation nodes: 589 property declarations, 304 callbacks, 421 callback events, 50 components, four globals, six function declarations, 80 imports, 854 `@tr` occurrences and 132 image references. Not a count of features, bindings or Rust callers.
- All 23 selected Slint files are transitively import-reachable from the single compilation root. Of 80 import statements, 66 target frozen repository paths and 14 `std-widgets.slint` imports are external standard library edges. No static image/import frozen-target misses. This does not prove Rust exported entrypoints or runtime reachability.
- 854 translation occurrences resolve to 635 unique context-free literal source keys. 849 occurrences have one exact catalog member; five reference duplicated `Installing…`. PO source has **740 entries /739 unique keys**, zero context/plural/fuzzy/empty translations. The duplicate messages have different Russian strings at [1008](<../../../slint-experiment/translations/ru/LC_MESSAGES/slint-replay.po#L1008-L1009>) and [1482](<../../../slint-experiment/translations/ru/LC_MESSAGES/slint-replay.po#L1482-L1483>). This is an observed catalog ambiguity, not tested Slint compiler duplicate precedence.
- 104 unique catalog keys have no direct selected Slint `@tr` literal occurrence; **not proven dead/unnecessary translations**, and no deletion recommended solely from this inventory.
- 58 assets hashed; SVG XML/root attributes/tag sets recorded. All 50 icon-root SVGs have 16×16 viewBox and stroke width 1.6. No SVG rendering, raster dimension/icon layer analysis, brand/contrast/tofu or native embedding acceptance.

## Parser choice and reproduction

**Verdict: Extend — existing pinned Rust/Slint parsers plus installed Babel PO parser and stdlib XML.** No regex reimplementation of gettext. [Babel/pytz provenance](../reconciliation/schema-parser-provenance.json) and [hash lock](../operations/schema-requirements.txt) pin Babel 2.10.3/pytz 2024.1; installed Babel PO-reader source hash matched upstream wheel exactly, licences inspected, isolated offline `--require-hashes` installation tested. CaptureCatalog preserves each duplicate message before Babel's normal merge. Obsolete entries excluded, invalid catalog/version/reader-hash fails closed. Python 3.12 tested; Babel emits a `cgi` deprecation warning, **Python 3.13 not accepted**.

Original [syntax setup](../syntax/README.md) supplies Tree-sitter binding 0.25.2, Rust grammar 0.24.2 and hash-gated Slint ABI15 library. Parser generation isolated in one subprocess/60s timeout; no SDK/build or source scripts executed.

```bash
python3 -m pip install --only-binary=:all: --no-deps --require-hashes \
  --target .campaign-state/schema-site \
  -r docs/agent-map/operations/schema-requirements.txt
# Original pinned parser targets/Slint library must already be supplied as documented.
export PYTHONPATH=.campaign-state/parser-site-stable:.campaign-state/language-site-locked:.campaign-state/schema-site
export SUFLYOR_RESEARCH_SLINT_GRAMMAR="$PWD/.campaign-state/slint-grammar/libtree_sitter_slint.so"
python3 -B docs/agent-map/operations/schema_inventory.py --output docs/agent-map/schema/inventory.json
python3 -B docs/agent-map/operations/schema_inventory.py --output docs/agent-map/schema/inventory.json --validate
python3 -B -m unittest discover -s docs/agent-map/operations -p 'test_*.py' -v
```

Generated inventory must be read before replacing it under file-observation policy. Packages stay in ignored targets; no app/global/DSH dependency change. Validation itself does not import native parser or Babel, but checks input hashes/node ranges/duplicate identity/default-evaluation boundary.

## Remaining limits

Literal decode accepts only JSON-compatible quoted Slint subset. Context/plural/interpolation/unknown escapes are retained unresolved, not false missing/exact-members. All current selected literal calls happen to decode; fixture coverage includes non-compatible cases. XML parsing is metadata only. Every Config consumer, migration/default interaction, Slint enum/struct/method/property bindings, dynamic Rust copy/translation, reset lifecycle and asset packaging relationships need further source/compiler/native evidence. This inventory never establishes an exhaustive audit percentage.
