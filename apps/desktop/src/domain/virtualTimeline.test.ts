import { describe, expect, it } from "vitest";

import {
  createTrailingVirtualTimelineWindow,
  createVirtualTimelineWindow,
} from "./virtualTimeline";

describe("virtual timeline windowing", () => {
  it("returns an empty window for empty timelines", () => {
    expect(createVirtualTimelineWindow({
      itemCount: 0,
      scrollTop: 0,
      viewportHeight: 720,
    })).toEqual({
      startIndex: 0,
      endIndex: 0,
      beforeHeight: 0,
      afterHeight: 0,
      totalHeight: 0,
      renderedCount: 0,
    });
  });

  it("renders all rows when the timeline is smaller than the minimum window", () => {
    expect(createVirtualTimelineWindow({
      itemCount: 12,
      scrollTop: 0,
      viewportHeight: 720,
      estimatedItemHeight: 80,
      minimumRenderedItems: 36,
    })).toMatchObject({
      startIndex: 0,
      endIndex: 12,
      renderedCount: 12,
      beforeHeight: 0,
      afterHeight: 0,
    });
  });

  it("keeps only a bounded row window around the current scroll position", () => {
    const window = createVirtualTimelineWindow({
      itemCount: 5_000,
      scrollTop: 120_000,
      viewportHeight: 900,
      estimatedItemHeight: 75,
      overscanItems: 10,
      minimumRenderedItems: 40,
    });

    expect(window.renderedCount).toBeGreaterThanOrEqual(40);
    expect(window.renderedCount).toBeLessThan(80);
    expect(window.startIndex).toBeGreaterThan(1_500);
    expect(window.endIndex).toBeLessThan(1_700);
    expect(window.beforeHeight).toBe(window.startIndex * 75);
    expect(window.afterHeight).toBe((5_000 - window.endIndex) * 75);
  });

  it("starts at the latest rows for the initial bottom-anchored render", () => {
    const window = createTrailingVirtualTimelineWindow(900, {
      estimatedItemHeight: 72,
      overscanItems: 18,
      minimumRenderedItems: 36,
    });

    expect(window.renderedCount).toBe(36);
    expect(window.startIndex).toBe(864);
    expect(window.endIndex).toBe(900);
    expect(window.afterHeight).toBe(0);
  });
});
