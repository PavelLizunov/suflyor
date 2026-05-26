// Tier 4 unit test for src/clock.ts — proves the mock-clock injection
// works so downstream tests can freeze time deterministically.

import { describe, it, expect } from "vitest";
import { now, setMockClock, advanceMockClock, restoreRealClock } from "./clock";

describe("clock", () => {
  it("returns real time by default", () => {
    restoreRealClock();
    const t1 = now();
    const t2 = now();
    // Real clock is monotonic non-decreasing.
    expect(t2).toBeGreaterThanOrEqual(t1);
    // And reasonable: in the year 2026 the epoch ms should be ≥ 1.7e12.
    expect(t1).toBeGreaterThan(1_700_000_000_000);
  });

  it("freezes at the value passed to setMockClock", () => {
    setMockClock(1_000_000_000_000);
    expect(now()).toBe(1_000_000_000_000);
    expect(now()).toBe(1_000_000_000_000); // still frozen after 2nd call
  });

  it("advanceMockClock moves the mocked clock forward", () => {
    setMockClock(1_000_000_000_000);
    advanceMockClock(500);
    expect(now()).toBe(1_000_000_000_500);
    advanceMockClock(2_000);
    expect(now()).toBe(1_000_000_002_500);
  });

  it("advanceMockClock is a no-op without an active mock", () => {
    restoreRealClock();
    const before = now();
    advanceMockClock(10_000);
    const after = now();
    // After call may differ by a few ms but NOT by 10 s.
    expect(after - before).toBeLessThan(1_000);
  });

  it("restoreRealClock reverts to Date.now()", () => {
    setMockClock(1);
    expect(now()).toBe(1);
    restoreRealClock();
    expect(now()).toBeGreaterThan(1_700_000_000_000);
  });

  it("setUp afterEach hook in setup.ts cleared any prior mock", () => {
    // If the setup.ts afterEach didn't fire, a prior test's setMockClock
    // would leak into this test. Verify clean state.
    expect(now()).toBeGreaterThan(1_700_000_000_000);
  });
});
