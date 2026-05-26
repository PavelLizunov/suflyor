// Vitest global setup — runs before every test file.
// Wires up jest-dom matchers (toBeInTheDocument, etc.) and resets the
// frontend clock between tests so no test can leak time-state.

import "@testing-library/jest-dom/vitest";
import { afterEach } from "vitest";
import { restoreRealClock } from "../clock";

afterEach(() => {
  restoreRealClock();
});
