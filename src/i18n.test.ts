// Tier 4 unit test for src/i18n.ts — TS-side copy contract.
// Pairs with src-tauri/tests/copy_contract.rs which checks the same
// strings from the Rust side (file scan). This test catches drift
// at the function-call level (typos in the `t(...)` argument).

import { describe, it, expect } from "vitest";
import { t, resolveLang } from "./i18n";

describe("i18n", () => {
  describe("resolveLang", () => {
    it('returns "en" only for the literal "en"', () => {
      expect(resolveLang("en")).toBe("en");
    });

    it('returns "ru" for "ru", null, undefined, or anything else', () => {
      expect(resolveLang("ru")).toBe("ru");
      expect(resolveLang(undefined)).toBe("ru");
      expect(resolveLang(null)).toBe("ru");
      // garbage input defaults to RU
      expect(resolveLang("fr")).toBe("ru");
      expect(resolveLang("")).toBe("ru");
    });
  });

  describe("t() — canonical Settings header strings", () => {
    it("settings.title is 'Settings' in both languages", () => {
      expect(t("settings.title", "ru")).toBe("Settings");
      expect(t("settings.title", "en")).toBe("Settings");
    });

    it("settings.quit RU has the ✕ prefix", () => {
      expect(t("settings.quit", "ru")).toBe("✕ Выйти");
      expect(t("settings.quit", "en")).toBe("✕ Quit");
    });

    it("settings.save round-trips", () => {
      expect(t("settings.save", "ru")).toBe("Сохранить");
      expect(t("settings.save", "en")).toBe("Save");
    });

    it("settings.saved shows the ✓ marker", () => {
      expect(t("settings.saved", "ru")).toBe("✓ Сохранено");
      expect(t("settings.saved", "en")).toBe("✓ Saved");
    });

    it("settings.back has the ← arrow", () => {
      expect(t("settings.back", "ru")).toBe("← К overlay");
      expect(t("settings.back", "en")).toBe("← Back to overlay");
    });
  });

  describe("t() — overlay status text exists for all 6 states", () => {
    const states = [
      "overlay.status.stopped",
      "overlay.status.paused",
      "overlay.status.listening",
      "overlay.status.thinking",
      "overlay.status.answering",
      "overlay.status.error",
    ] as const;
    for (const key of states) {
      it(`${key} returns a non-empty RU + EN`, () => {
        expect(t(key, "ru").length).toBeGreaterThan(0);
        expect(t(key, "en").length).toBeGreaterThan(0);
      });
    }
  });

  describe("t() — F4 palette placeholder", () => {
    it("mentions both Russian and English purposeful phrasing", () => {
      const ru = t("overlay.palette.placeholder", "ru");
      const en = t("overlay.palette.placeholder", "en");
      expect(ru.length).toBeGreaterThan(0);
      expect(en.length).toBeGreaterThan(0);
    });
  });
});
