// Single source of truth for "current time" in the frontend.
//
// Per methodology spec § Principles: "No Date.now() in logic — only
// inject." This module is the ONE blessed Date.now() callsite. Tests
// can swap the underlying clock via setMockClock() to make time-dependent
// code deterministic.
//
// Usage:
//   import { now } from "./clock";
//   const elapsed = now() - sessionStartMs;
//
// In a test:
//   import { setMockClock, restoreRealClock } from "./clock";
//   setMockClock(1_700_000_000_000);  // freeze time
//   ...
//   restoreRealClock();
//
// The ESLint `no-restricted-syntax` rule blocks `Date.now()` everywhere
// except this file (see eslint.config.js).

let mockClock: number | null = null;

/**
 * Returns the current time in milliseconds since the Unix epoch.
 * In production this is `Date.now()`. In tests, swap via `setMockClock()`.
 */
export function now(): number {
  if (mockClock !== null) return mockClock;
  // eslint-disable-next-line no-restricted-syntax -- the ONE blessed Date.now() call
  return Date.now();
}

/** Freeze time for tests. Pass an absolute ms-epoch value. */
export function setMockClock(ms: number): void {
  mockClock = ms;
}

/** Advance the mocked clock by `deltaMs`. No-op if no mock active. */
export function advanceMockClock(deltaMs: number): void {
  if (mockClock !== null) mockClock += deltaMs;
}

/** Restore real `Date.now()` semantics. Call in test `afterEach`. */
export function restoreRealClock(): void {
  mockClock = null;
}
