# Config, UI compilation, translation and assets: bounded source contract

**Evidence:** frozen baseline `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`, principal code paths plus [reproducible declaration/resource inventory](<../schema/README.md>). No live config/credentials, Rust/Slint compile, app UI or native packaging. The complete Config consumer/UI binding graph is still open.

## Config schema and default mechanisms

[Config declaration](<../../../overlay-backend/src/config.rs#L29-L499>) derives Deserialize/Serialize/**Default**, with struct `#[serde(default)]` and selected field-specific default helpers. Inventory lists 84 source fields/types/attributes. [Explicit defaults](<../../../overlay-backend/src/config.rs#L617-L706>) is a separate hand-authored constructor with 84 initializers. Derived primitive defaults, Serde missing-field overrides and explicit fresh-config defaults are not interchangeable; helper source ranges are linked without executing or publishing literal default endpoints.

[Load](<../../../overlay-backend/src/config.rs#L1360-L1441>) handles UTF8 BOM; path/read/parse failure returns `Config::defaults()`, but valid config parses via Serde then migrates endpoint/model/macOS legacy state. Missing/invalid config doesn't unconditionally save new config. [Normalization](<../../../overlay-backend/src/config.rs#L1380-L1397>) repairs context and normalizes STT provider. Main applies further state normalization; inventory does not claim all repairs/callers covered.

[Save](<../../../overlay-backend/src/config.rs#L1561-L1625>) serializes pretty JSON to temporary file, Unix permissions 0600 and sync, Windows removes existing destination before rename. It is not a guaranteed atomic-replace path on every platform; crash/fault behavior was not run. [Backup redaction](<../../../overlay-backend/src/config.rs#L1627-L1639>) blanks legacy bridge/Groq/vision/Whisper bearer fields. [Portable export](<../../../overlay-backend/src/config.rs#L1658-L1669>) includes entered secrets by design. Direct-provider keys stay outside Config in credential storage; do not infer all config/export/backup paths are secret-free. None were opened by research.

[UI language](<../../../overlay-backend/src/config.rs#L429-L447>) and AI answer-language fields are distinct; `default_ui_language()` returns ru [helper](<../../../overlay-backend/src/config.rs#L1246-L1251>). Old React/live-language comments are historical, not current behavior.

## Single Slint compilation graph and declarations

[Build](<../../../slint-experiment/build.rs#L26-L32>) bundles translations with `DefaultTranslationContext::None`, compiles `ui/index.slint` once and conditionally emits QA/development debug metadata. [Root](<../../../slint-experiment/ui/index.slint#L1-L53>) imports/re-exports components/globals, including spike/replay windows alongside production windows. Inventory transitive graph reaches all 23 selected `ui/*.slint` files via 66 repository imports and 14 external std-widgets imports. Reachable syntax is not proof an exported window is used at runtime.

589 properties/304 callbacks/421 callback events are navigation declarations, not resolved Rust generated APIs. Hyphen to underscore naming and equal tokens are not sufficient to prove callback wiring, property read/update ownership, reset correctness, data type equivalence or effective window behavior. Existing [reset guard](<../../../slint-experiment/tests/settings_reset_guard.rs#L32-L101>) does naming-convention source scans for transient Settings strings; it doesn't execute async lifecycle nor cover all boolean/runtime properties. Selected [Settings paths](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L105-L166>) refresh token/status/diagnostic state on reuse. Full per-key consumer/state audit remains open.

## Translation lookup versus dynamic Rust text

[Startup](<../../../slint-experiment/src/bin/overlay_host_windows.rs#L764-L778>) selects stored `ui_language` after overlay construction and logs unsupported lookup; [Settings language callback](<../../../slint-experiment/src/bin/overlay_host/settings_controller.rs#L550-L609>) selects ru/en, persists config **after** successful selection, refreshes tray and reopens visible help/palette to regenerate Rust labels. Save failure only logs; live translation may differ from persisted config after restart. Other Rust strings use explicit language helpers/raw messages; catalog membership doesn't establish coverage for them.

854 selected `@tr` occurrences have 635 distinct context-free decoded literal msgids. All have current catalog members; five use [duplicated Installing…](<../../../slint-experiment/translations/ru/LC_MESSAGES/slint-replay.po#L1008-L1009>) with [different second translation](<../../../slint-experiment/translations/ru/LC_MESSAGES/slint-replay.po#L1482-L1483>). Both source entries retained in inventory, not silently merged. Slint compilation/lookup duplicate precedence was not tested. No translation edited or automatically deleted.

The [existing Rust i18n guard](<../../../slint-experiment/tests/i18n_guard.rs#L15-L72>) parses gettext-like source and uses a HashSet of msgids, ignoring duplicate translation values; its [source checks](<../../../slint-experiment/tests/i18n_guard.rs#L310-L395>) enforce literal membership/non-technical display-string wrapping with a lexer/allowlist. It does not establish Russian wording/placeholder quality/duplicate resolution/UI rendering. **Cargo guard was read, not run.** This research uses pinned Babel PO parsing plus Slint CST, retaining context/plural unsupported-match boundaries. Zero current context/plural/fuzzy/empty entries are observed, not a claim these forms cannot occur.

104 catalog keys have no selected direct `@tr` literal occurrence. They are **not proven dead**, and unused keys are not evidence of missing language support. Compiler extraction/context/plural/dynamic usage still require separate acceptance.

## Assets and native embedding

58 asset files are frozen-hashed, 132 static `@image-url` occurrences resolve within snapshot. SVG XML/root metadata parsed; all 50 icon roots meet viewBox/stroke-width metadata. [Icon guard](<../../../slint-experiment/tests/icon_guard.rs#L7-L170>) checks an exact named set/header/body constraints, but was not executed here. XML attributes are less evidence than the full Rust guard and not rendering/contrast/tofu proof.

[Windows embedding](<../../../slint-experiment/build.rs#L55-L68>) attempts winresource icon compilation and logs warning on failure rather than making icon-embed success a build requirement. [macOS build seam](<../../../slint-experiment/build.rs#L34-L53>) compiles AppKit bridges; asset packaging/installer references require separate native/script inventory. Static resource existence never proves launcher icon identity or final bundle contents.

## Unaccepted behavior

Config consumers/migrations/credentials/export failures, all Rust UI setter/callback chains, template/interpolation/copy/placeholder translation quality, icon/raster layers/font/visual geometry, native compilation/packaging, source hypothesis reproductions and independent Gemini/Opus acceptance remain open. This contract adds source evidence without changing original 39/75/5 claim counts or production code.
