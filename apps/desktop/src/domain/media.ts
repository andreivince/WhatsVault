import type { Attachment, AttachmentPreview } from "../models";

export type AttachmentRenderKind = "image" | "audio" | "video" | "document" | "file";

export function attachmentRenderKind(
  attachment: Attachment,
  preview: AttachmentPreview | null,
): AttachmentRenderKind {
  if (preview?.mediaType.startsWith("image/")) {
    return "image";
  }
  if (preview?.mediaType.startsWith("audio/")) {
    return "audio";
  }
  if (preview?.mediaType.startsWith("video/")) {
    return "video";
  }
  if (preview?.mediaType === "application/pdf") {
    return "document";
  }

  switch (attachment.kind) {
    case "audio":
      return "audio";
    case "gif":
    case "photo":
    case "sticker":
      return "image";
    case "video":
      return "video";
    default:
      return "file";
  }
}

export function canRequestAttachmentPreview(attachment: Attachment): boolean {
  return ["audio", "gif", "photo", "sticker", "video", "unknown"].includes(attachment.kind);
}
