// Tier 2 ESLint config — strict-type-checked baseline from the suflyor
// GUI strictness spec. Run via `npx eslint src`. Wired into git-gate
// future revision; for now manually invoked.
//
// Rules of note:
//   - no-explicit-any            : `any` is an error
//   - no-non-null-assertion       : `x!` is an error
//   - consistent-type-assertions  : `x as T` is an error (use `satisfies` or
//                                   narrow via typeguards)
//   - switch-exhaustiveness-check : every union member must have a case
//   - react-hooks/exhaustive-deps : useEffect deps must be exhaustive
//   - no-restricted-syntax        : Date.now() / Math.random() forbidden;
//                                   inject clock / RNG so logic is testable

import js from "@eslint/js";
import tseslint from "typescript-eslint";
import reactPlugin from "eslint-plugin-react";
import reactHooks from "eslint-plugin-react-hooks";

export default tseslint.config(
  // 1. ignore generated / vendored stuff
  {
    ignores: [
      "dist/**",
      "src-tauri/**",
      "node_modules/**",
      "scripts/**",
      "docs/**",
      "*.config.js",
      "*.config.ts",
    ],
  },
  // 2. baseline for all TS/TSX
  js.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,
  {
    files: ["src/**/*.{ts,tsx}"],
    languageOptions: {
      parserOptions: {
        project: "./tsconfig.json",
        tsconfigRootDir: import.meta.dirname,
      },
      globals: {
        window: "readonly",
        document: "readonly",
        console: "readonly",
        navigator: "readonly",
        setTimeout: "readonly",
        clearTimeout: "readonly",
        setInterval: "readonly",
        clearInterval: "readonly",
        requestAnimationFrame: "readonly",
        cancelAnimationFrame: "readonly",
        URL: "readonly",
        URLSearchParams: "readonly",
        ResizeObserver: "readonly",
        HTMLElement: "readonly",
        HTMLInputElement: "readonly",
        HTMLTextAreaElement: "readonly",
        HTMLButtonElement: "readonly",
        HTMLAnchorElement: "readonly",
        HTMLImageElement: "readonly",
        HTMLDivElement: "readonly",
        HTMLStyleElement: "readonly",
        Element: "readonly",
        Event: "readonly",
        MouseEvent: "readonly",
        KeyboardEvent: "readonly",
        FocusEvent: "readonly",
        DragEvent: "readonly",
        WheelEvent: "readonly",
        Notification: "readonly",
        FileList: "readonly",
        File: "readonly",
        Blob: "readonly",
        URL: "readonly",
        FormData: "readonly",
        atob: "readonly",
        btoa: "readonly",
        fetch: "readonly",
      },
    },
    plugins: {
      react: reactPlugin,
      "react-hooks": reactHooks,
    },
    settings: { react: { version: "detect" } },
    rules: {
      // ── core safety overrides (spec) ────────────────────────────────
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/no-non-null-assertion": "error",
      "@typescript-eslint/consistent-type-assertions": [
        "error",
        { assertionStyle: "never" },
      ],
      "@typescript-eslint/switch-exhaustiveness-check": "error",

      // ── react hooks ──────────────────────────────────────────────────
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "error",

      // ── determinism (per spec § 5 of methodology) ────────────────────
      "no-restricted-syntax": [
        "error",
        {
          selector:
            "CallExpression[callee.object.name='Date'][callee.property.name='now']",
          message:
            "Inject a clock instead of calling Date.now() directly — see methodology spec § Principles.",
        },
        {
          selector:
            "CallExpression[callee.object.name='Math'][callee.property.name='random']",
          message:
            "Inject a RNG instead of calling Math.random() directly — see methodology spec § Principles.",
        },
      ],

      // ── strict-type-checked overrides where it's too noisy on this
      //    codebase. Document EACH downgrade with rationale.

      // Overlay event handlers fire-and-forget invoke() routinely; ignoreVoid
      // covers the `void promise()` idiom used throughout.
      "@typescript-eslint/no-floating-promises": ["error", { ignoreVoid: true }],
      // React JSX boolean event handlers returning Promise<void> are fine.
      "@typescript-eslint/no-misused-promises": [
        "error",
        { checksVoidReturn: { attributes: false } },
      ],
      // The Replay viewer renders `${number}` etc in template strings against
      // JSON-parsed unknown values; the helpers `asStr/asNum` already coerce
      // safely. Don't make every template literal a type-conversion ceremony.
      "@typescript-eslint/restrict-template-expressions": "off",
      // `if (x.length > 0)` after a noUncheckedIndexedAccess access is
      // sometimes flagged as "unnecessary". Keep as off — defensive checks
      // are intentional, especially in event handlers reading user input.
      "@typescript-eslint/no-unnecessary-condition": "off",
      // Empty `.catch(() => {})` for fire-and-forget IPC is a deliberate
      // pattern in this codebase (transcript export, mic-mute hint, etc.).
      "@typescript-eslint/no-empty-function": "off",
      // `||` vs `??` is a style preference here; `||` is used intentionally
      // for "or default" on empty strings too. Off.
      "@typescript-eslint/prefer-nullish-coalescing": "off",
      // Tauri event payloads use `void` legitimately; rule overfires here.
      "@typescript-eslint/no-invalid-void-type": "off",
    },
  },
  // 3. tests — relax a couple of rules; tests may use unwrap-style asserts.
  {
    files: ["src/**/*.test.{ts,tsx}", "src/__tests__/**/*.{ts,tsx}"],
    rules: {
      "@typescript-eslint/no-explicit-any": "off",
      "@typescript-eslint/no-non-null-assertion": "off",
    },
  },
);
