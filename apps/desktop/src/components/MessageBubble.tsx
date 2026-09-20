import { File } from "lucide-react";
import { memo, useEffect, useState } from "react";

import { attachmentLabel, displayTimestamp, isOutgoingMessage } from "../domain/chat";
import { attachmentRenderKind, canRequestAttachmentPreview } from "../domain/media";
import type { Attachment, AttachmentPreview, LoadedChatSource, Message } from "../models";
import { attachmentPreviewLoader } from "../services/attachmentPreview";
import { isDesktopRuntime } from "../services/desktop";
import { TEST_IDS } from "../testing/testIds";

export const MessageBubble = memo(function MessageBubble({
  message,
  source,
  attachmentMap,
  onOpenImagePreview,
}: {
  message: Message;
  source: LoadedChatSource | null;
  attachmentMap: Map<string, Attachment>;
  onOpenImagePreview: (preview: { dataUrl: string; alt: string; caption: string }) => void;
}) {
  const outgoing = isOutgoingMessage(message);
  const attachments = message.attachment_ids
    .map((id) => attachmentMap.get(id))
    .filter((attachment): attachment is Attachment => Boolean(attachment));

  return (
    <article
      className={`message-row${outgoing ? " outgoing" : " incoming"}`}
      data-testid={TEST_IDS.messageBubble}
    >
      <div className="message-bubble">
        {!outgoing && message.sender ? <span className="message-sender">{message.sender}</span> : null}
        {attachments.length > 0 ? (
          <div className="attachment-stack">
            {attachments.map((attachment) => (
              <AttachmentBlock
                key={attachment.id}
                attachment={attachment}
                source={source}
                onOpenImagePreview={onOpenImagePreview}
              />
            ))}
          </div>
        ) : null}
        {message.body ? <p>{message.body}</p> : null}
        <span className="message-time">
          {displayTimestamp(message.timestamp.raw)}
        </span>
      </div>
    </article>
  );
});

function AttachmentBlock({
  attachment,
  source,
  onOpenImagePreview,
}: {
  attachment: Attachment;
  source: LoadedChatSource | null;
  onOpenImagePreview: (preview: { dataUrl: string; alt: string; caption: string }) => void;
}) {
  const [preview, setPreview] = useState<AttachmentPreview | null>(attachment.preview ?? null);
  const [previewState, setPreviewState] = useState<"idle" | "loading" | "unavailable">("idle");
  const renderKind = attachmentRenderKind(attachment, preview);

  useEffect(() => {
    let cancelled = false;

    async function loadPreview() {
      if (attachment.preview) {
        setPreview(attachment.preview);
        setPreviewState("idle");
        return;
      }

      if (!source || !isDesktopRuntime() || !canRequestAttachmentPreview(attachment)) {
        setPreview(null);
        setPreviewState("unavailable");
        return;
      }

      setPreviewState("loading");
      try {
        const nextPreview = await attachmentPreviewLoader.load(source, attachment);
        if (!cancelled) {
          setPreview(nextPreview);
          setPreviewState(nextPreview ? "idle" : "unavailable");
        }
      } catch {
        if (!cancelled) {
          setPreviewState("unavailable");
        }
      }
    }

    loadPreview();
    return () => {
      cancelled = true;
    };
  }, [attachment, source]);

  if (preview && renderKind === "image") {
    return (
      <figure className="attachment-preview" data-testid={TEST_IDS.mediaBlock}>
        <button
          className="attachment-image-button"
          type="button"
          onClick={(event) => {
            // Safari does not focus buttons on pointer activation. Keep a return target for the dialog.
            event.currentTarget.focus({ preventScroll: true });
            onOpenImagePreview({
              dataUrl: preview.dataUrl,
              alt: attachment.filename,
              caption: attachment.filename,
            });
          }}
          aria-label={`Open ${attachment.filename}`}
        >
          <img src={preview.dataUrl} alt={attachment.filename} />
        </button>
        <figcaption>{attachment.filename}</figcaption>
      </figure>
    );
  }

  if (preview && renderKind === "audio") {
    return (
      <figure className="attachment-player" data-testid={TEST_IDS.mediaBlock}>
        <audio controls src={preview.dataUrl} preload="metadata" />
        <figcaption>{attachment.filename}</figcaption>
      </figure>
    );
  }

  if (preview && renderKind === "video") {
    return (
      <figure className="attachment-video" data-testid={TEST_IDS.mediaBlock}>
        <video controls src={preview.dataUrl} preload="metadata" />
        <figcaption>{attachment.filename}</figcaption>
      </figure>
    );
  }

  if (preview && renderKind === "document") {
    return (
      <a
        className="attachment-document"
        href={preview.dataUrl}
        download={attachment.filename}
        data-testid={TEST_IDS.mediaBlock}
      >
        <File />
        <span>{attachment.filename}</span>
      </a>
    );
  }

  return (
    <div className="attachment-chip" data-testid={TEST_IDS.mediaBlock}>
      <span>{previewState === "loading" ? "Loading media" : attachmentLabel(attachment.kind)}</span>
    </div>
  );
}
