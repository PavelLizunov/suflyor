# Settings provider geometry audit

Baseline source: b3211e39770960152f5a75e561e28162aab5ed3c.
The existing native QA executable was built from 76213c4bd7d3a5d047baef518e65ffcd82b42997; its source tree is identical to the baseline. Executable SHA-256: 1b87f67419f9a4fb30b32a86a7ee82def83821263132b380bc5880804328bc8f.

Task scope: remove overlap between the OpenAI connection test and the download card; investigate the reported local-capabilities caption clipping. Reuse the existing width-aware provider wrapper rather than add fixed heights. Preserve all routing and callbacks.

Native baseline: Windows, software renderer, Light Frost, RU, 720x600 logical pixels, 100% scale. OpenAI conditional group y144/h296; final button y431/h32; download card y448. The card overlaps 15px of the button.

Local-capabilities baseline: caption is contained with 12px bottom padding at 720px, 600px and 480px window widths after scrolling to the end. At 125% scale, 720x600 logical pixels, it is initially below the viewport. No internal caption clipping has been reproduced yet. Do not claim a DarkCheck defect from source shape alone.

Private PNGs and MCP trees are retained outside Git under ~/.hygiene/sufloyr/ui-geometry/. Before/after pairs must use the same language, size, scale, theme, synthetic provider fields and scroll endpoint. No model activation/download or external connection test is necessary for layout reproduction.

Checks: settings_provider_geometry on the Windows worker; selected native gate; exact-SHA MCP audit in EN/RU; independent read-only UI review. Shared primitives remain unchanged unless native evidence demonstrates an owning defect.
