# Asset Guidelines & Rules (`slint-experiment/assets/`)

Local guide for inspecting, adding, and maintaining UI assets in `slint-experiment/assets/`.

---

## 1. SVG Vector Icon Convention (`assets/icons/*.svg`)

- **Canvas Grid:** Every icon in `assets/icons/*.svg` must specify `viewBox="0 0 16 16"`.
- **Stroke & Outline Standard:** Use `fill="none"` with `stroke-width="1.6"`, `stroke-linecap="round"`, and `stroke-linejoin="round"`.
- **Margin & Framing:** Keep vector elements centered with roughly 1.5 px of outer padding/breathing room.
- **Coloring:** Icons are monochrome stroke paths; color is applied dynamically in Slint via `colorize: Theme.accent` (or appropriate design token).
- **Static Guard Test:** The icon standard is strictly enforced by `icon_guard.rs`. Run:
  ```powershell
  cargo test --manifest-path slint-experiment/Cargo.toml --test icon_guard
  ```

---

## 2. Binary Asset Caution (`assets/icon.ico`, `assets/*.png`)

- **Binary Assets:** Root assets include binary images (`icon.ico`, `icon.png`, `brand-mark.png`, `brand-mark-light.png`, `icon-source.png`).
- **Build & Installer Coupling:** Binary assets are baked into executable identity (`winresource` in `build.rs`), installer bundles (NSIS setup), and window icons.
- **Modification Caution:** Do not edit or replace binary assets unless specifically required. Any updates must preserve required pixel dimensions, aspect ratios, and format requirements without introducing binary bloat or broken executable icon embedding.

---

## 3. No-Tofu Alternatives & Visual Verification

- **No-Tofu Rule:** The Skia rendering backend used by Slint on Windows renders non-standard Unicode symbols (e.g. `⚠️`, `✓`, `🎤`, `①`) as missing-glyph "tofu" boxes.
- **ASCII & Vector Fallbacks:**
  - For inline text status or buttons, use ASCII fallbacks: `[!]` for warnings, `[ok]` for checks, `1)` for step numbers.
  - For visual indicators, use vector SVG icons from `assets/icons/*.svg`.
- **Visual Verification:** Asset/icon integration in `.slint` UI components must be visually verified in the running application using the embedded Slint MCP server QA build (`cargo build --bin overlay-host --features ui-mcp ...` and `take_screenshot`).
