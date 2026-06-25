import { describe, expect, it } from "vitest";

import { attachmentRenderKind, canRequestAttachmentPreview } from "./media";
import type { Attachment, AttachmentPreview } from "../models";

function attachment(kind: Attachment["kind"], filename = "media.bin"): Attachment {
  return {
    id: "attachment-1",
    archive_path: filename,
    filename,
    kind,
    size_bytes: 100,
  };
}

function preview(mediaType: string): AttachmentPreview {
  return {
    mediaType,
    dataUrl: `data:${mediaType};base64,Zm9v`,
    sizeBytes: 3,
  };
}

describe("media render helpers", () => {
  it("prefers returned media type over attachment category", () => {
    expect(attachmentRenderKind(attachment("unknown"), preview("image/webp"))).toBe("image");
    expect(attachmentRenderKind(attachment("unknown"), preview("audio/ogg"))).toBe("audio");
    expect(attachmentRenderKind(attachment("unknown"), preview("video/mp4"))).toBe("video");
    expect(attachmentRenderKind(attachment("unknown"), preview("application/pdf"))).toBe(
      "document",
    );
  });

  it("falls back to attachment kind while media loads or is unavailable", () => {
    expect(attachmentRenderKind(attachment("photo"), null)).toBe("image");
    expect(attachmentRenderKind(attachment("sticker"), null)).toBe("image");
    expect(attachmentRenderKind(attachment("audio"), null)).toBe("audio");
    expect(attachmentRenderKind(attachment("video"), null)).toBe("video");
    expect(attachmentRenderKind(attachment("unknown"), null)).toBe("file");
  });

  it("limits preview requests to known exported media categories", () => {
    expect(canRequestAttachmentPreview(attachment("photo"))).toBe(true);
    expect(canRequestAttachmentPreview(attachment("audio"))).toBe(true);
    expect(canRequestAttachmentPreview(attachment("video"))).toBe(true);
  });
});
