import { describe, expect, it } from "vitest";
import type { Message } from "../models";
import { messageDayLabel, messageDateRangeLabel } from "./chat";

const message = (raw: string): Message => ({
  id: raw, timestamp: { raw }, sender: null, body: "Synthetic message", attachment_ids: [],
});

describe("timeline date headings", () => {
  it("formats the source date without moving it across time zones", () => {
    expect(messageDayLabel(message("06/23/2026, 7:35 AM"), "en-US")).toBe("Jun 23, 2026");
    expect(messageDayLabel(message("2026-06-24T00:01:00Z"), "en-US")).toBe("Jun 24, 2026");
    expect(messageDayLabel(message("unavailable"), "en-US")).toBe("Date unavailable");
  });
  it("labels a multi-day window and omits the label for no messages", () => {
    const first = message("06/23/2026, 7:35 AM");
    const last = message("06/24/2026, 8:35 AM");
    expect(messageDateRangeLabel([first, last], "en-US")).toBe("Jun 23, 2026 to Jun 24, 2026");
    expect(messageDateRangeLabel([first, first], "en-US")).toBe("Jun 23, 2026");
    expect(messageDateRangeLabel([], "en-US")).toBeNull();
  });
});
