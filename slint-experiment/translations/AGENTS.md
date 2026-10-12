# Slint Translation Catalog & Authoring Guide (`slint-experiment/translations/`)

Guide for inspecting, maintaining, and verifying internationalization (i18n) translation catalogs and `@tr` string coupling for the `slint-experiment` crate (`overlay-host` binary).

---

## 1. i18n Architecture & `build.rs` Setup

Suflyor UI uses Slint's built-in gettext translation system. English source strings defined in `.slint` markup are translated at runtime into Russian via bundled `.po` catalogs.

- **Catalog Location:**
  `slint-experiment/translations/ru/LC_MESSAGES/slint-replay.po`
- **Compiler Configuration (`slint-experiment/build.rs`):**
  - **Bundled Translations:** `slint_build::CompilerConfiguration::new().with_bundled_translations("translations")` discovers and embeds catalog files under `translations/<lang>/LC_MESSAGES/*.po`.
  - **Context-Free Lookup (MANDATORY):** `config.with_default_translation_context(slint_build::DefaultTranslationContext::None)` is required. By default, Slint keys `@tr("...")` lookups by component name (e.g. `msgctxt="OverlayBarWindow"`). Because `slint-replay.po` uses context-free bare `msgid` keys, `DefaultTranslationContext::None` forces Slint to perform bare `msgid` matching.
- **Runtime Language Selection:**
  Language switching is driven by `slint::select_bundled_translation("ru")` or `slint::select_bundled_translation("en")`. Selecting `"en"` or an unmapped string falls back to the English source `msgid`.

---

## 2. Key Invariants & Matching Rules

1. **English `@tr` Source Rule:**
   Every user-facing display string in `.slint` components (`text`, `placeholder-text`, `title`, `accessible-label`) MUST be wrapped in English `@tr("...")`. Hardcoded Cyrillic strings in `.slint` files are strictly forbidden.
2. **Exact `msgid` Matching:**
   The `msgid` entry in `slint-replay.po` must match the literal argument of `@tr("...")` EXACTLY, including case, whitespace, punctuation, and placeholder brackets (e.g., `"{}"` or `"#{}"`). Any mismatch causes lookup failure and silent fallback to English.
3. **No Context (`msgctxt`):**
   Entries in `slint-replay.po` are plain `msgid` / `msgstr` pairs. Do not add `msgctxt` lines unless build configuration is updated accordingly.
4. **Technical & Dynamic Non-Translatable Text:**
   Machine tokens, URLs (`http://...`), raw numbers, single-word technical labels (e.g. `"AI"`, `"STT"`, `"px"`), or dynamic strings constructed in Rust logic must NOT be wrapped in `@tr`.

---

## 3. Russian PO Catalog Structure (`slint-replay.po`)

The catalog file is UTF-8 encoded and follows GNU gettext format:

```po
msgid ""
msgstr ""
"Content-Type: text/plain; charset=UTF-8\n"
"Language: ru\n"

# === overlay_bar.slint ===

msgid "ask"
msgstr "спросить"

msgid "+ tile ({})"
msgstr "+ тайл ({})"
```

Catalog entries are organized into visual section headers corresponding to `.slint` files (e.g., `# === overlay_bar.slint ===`, `# === tile.slint ===`, `# === palette.slint ===`, `# === settings_panel.slint ===`).

---

## 4. Translation Workflow

### Adding a New User-Facing UI String
1. Wrap the new English UI text in `@tr("...")` inside the appropriate `.slint` file in `ui/`.
2. Open `slint-experiment/translations/ru/LC_MESSAGES/slint-replay.po`.
3. Add a matching `msgid` / `msgstr` pair under the corresponding section header.
4. Run `cargo test --manifest-path slint-experiment/Cargo.toml --test i18n_guard` to verify translation parity.

### Modifying an Existing UI String
1. Update `@tr("Updated English text")` in `.slint`.
2. Update the corresponding `msgid` (and matching `msgstr`) in `slint-replay.po` in lockstep.
3. Run `i18n_guard` to catch any remaining references to the old `msgid`.

---

## 5. Verification Commands

Run targeted static guard checks to validate translation parity:

- **i18n Parity Guard Test:**
  ```powershell
  cargo test --manifest-path slint-experiment/Cargo.toml --test i18n_guard
  ```
  *What `i18n_guard.rs` enforces:*
  - Scans all `ui/*.slint` files for unwrapped user-facing string literals in `text`, `placeholder-text`, `title`, and `accessible-label` properties.
  - Verifies that every English `@tr("...")` string in `ui/*.slint` has an exact matching `msgid` entry in `slint-replay.po`.

- **Crate Compilation Check:**
  ```powershell
  cargo check --bin overlay-host --manifest-path slint-experiment/Cargo.toml
  ```

- **Native Gate Integration:**
  `scripts/git-gate-native.ps1` runs `i18n_guard` automatically on all targeted UI and full CI gate runs.
