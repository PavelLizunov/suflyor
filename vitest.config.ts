// Tier 4 Vitest config — TS unit + component tests.
// Run via `npm test` (one-shot) or `npm run test:watch`.
//
// Per methodology spec § 5: component layer = "Vitest + Testing Library /
// mockIPC". This is the TS half of the test pyramid; the Rust half stays
// in `cargo test --lib + tests/copy_contract.rs`.

import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/__tests__/setup.ts"],
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
    coverage: {
      provider: "v8",
      reporter: ["text", "html"],
      exclude: ["node_modules/", "src/__tests__/setup.ts", "src/main.tsx"],
      // Spec § 7: ≥ 90% lines, ≥ 85% branches required for merge.
      // overlay-mvp baseline starts much lower; treat as aspirational
      // for now and document in CLAUDE.md § Tier 4 status.
      thresholds: {
        lines: 10,
        branches: 10,
      },
    },
  },
});
