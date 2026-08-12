import { describe, expect, it } from "vitest";

import { createLatestRequestGate } from "./latestRequest";

describe("createLatestRequestGate", () => {
  it("keeps only the newest request current", () => {
    const gate = createLatestRequestGate();
    const isFirstCurrent = gate.begin();
    const isSecondCurrent = gate.begin();

    expect(isFirstCurrent()).toBe(false);
    expect(isSecondCurrent()).toBe(true);
  });

  it("can invalidate the active request without starting another", () => {
    const gate = createLatestRequestGate();
    const isCurrent = gate.begin();

    gate.invalidate();

    expect(isCurrent()).toBe(false);
  });
});
