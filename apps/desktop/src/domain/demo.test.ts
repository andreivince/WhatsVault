import { describe, expect, it } from "vitest";

import { createDemoImport } from "./demo";
import type { AttachmentPreview } from "../models";

describe("demo data", () => {
  it("keeps synthetic media previews attached to their demo attachments", () => {
    const demo = createDemoImport();
    const photo = demo.attachments.find((attachment) => attachment.id === "demo-photo");

    expect(photo).toBeDefined();
    expect((photo as { preview?: AttachmentPreview }).preview).toMatchObject({
      mediaType: "image/svg+xml",
      sizeBytes: expect.any(Number),
    });
    expect((photo as { preview?: AttachmentPreview }).preview?.dataUrl).toContain(
      "data:image/svg+xml;base64,",
    );
  });
});
