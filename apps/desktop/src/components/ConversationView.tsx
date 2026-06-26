import { Download, File, X } from "lucide-react";
import {
  type ChangeEvent,
  type FormEvent,
  memo,
  type RefObject,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import {
  attachmentLabel,
  displayTimestamp,
  isOutgoingMessage,
  messageCountLabel,
  messageFilterResultLabel,
  messageWindowNotice,
} from "../domain/chat";
import { attachmentRenderKind, canRequestAttachmentPreview } from "../domain/media";
import { DEFAULT_SOURCE_KIND, sourceProfile } from "../domain/source";
import {
  createTrailingVirtualTimelineWindow,
  createVirtualTimelineWindow,
} from "../domain/virtualTimeline";
import type {
  Attachment,
  AttachmentPreview,
  ChatImport,
  LoadedChatSource,
  Message,
} from "../models";
import { attachmentPreviewLoader } from "../services/attachmentPreview";
import { isDesktopRuntime } from "../services/desktop";
import { TEST_IDS } from "../testing/testIds";
import type { ConversationBackupSearchStatus, ExportState } from "../viewState";
import { Avatar } from "./Avatar";

export function ConversationView({
  imported,
  source,
  title,
  query,
  selectedDate,
  visibleMessages,
  hiddenEarlierCount,
  attachmentMap,
  backupSearchStatus,
  messageLimitStep,
  onExportHtml,
  onShowEarlier,
  onSelectedDateChange,
  exportState,
  timelineIdentity,
}: {
  imported: ChatImport;
  source: LoadedChatSource | null;
  title: string;
  query: string;
  selectedDate: string;
  visibleMessages: Message[];
  hiddenEarlierCount: number;
  attachmentMap: Map<string, Attachment>;
  backupSearchStatus: ConversationBackupSearchStatus;
  messageLimitStep: number;
  onExportHtml: () => void;
  onShowEarlier: () => void;
  onSelectedDateChange: (value: string) => void;
  exportState: ExportState;
  timelineIdentity: string;
}) {
  const profile = sourceProfile(source?.kind ?? DEFAULT_SOURCE_KIND);
  const canExportHtml = profile.supportsHtmlExport;
  const [imagePreview, setImagePreview] = useState<{
    dataUrl: string;
    alt: string;
    caption: string;
  } | null>(null);
  const conversationViewRef = useRef<HTMLDivElement>(null);
  const messageCanvasRef = useRef<HTMLDivElement>(null);
  const hasActiveFilters = Boolean(query.trim() || selectedDate);
  const importWindowNotice = messageWindowNotice(imported);
  const backupSearchBannerLabel =
    backupSearchStatus.status === "loading"
      ? "Searching backup..."
      : backupSearchStatus.status === "ready"
        ? `${backupSearchStatus.isTruncated ? "Latest " : ""}${visibleMessages.length.toLocaleString()} backup matches`
        : backupSearchStatus.status === "error"
          ? "Loaded-message matches"
          : null;
  const handleDateFilterChange = (
    event: ChangeEvent<HTMLInputElement> | FormEvent<HTMLInputElement>,
  ) => {
    onSelectedDateChange(event.currentTarget.value);
  };
  const bannerLabel = exportState.status === "exporting"
    ? "Exporting..."
    : backupSearchBannerLabel
      ? backupSearchBannerLabel
    : hasActiveFilters
      ? messageFilterResultLabel(visibleMessages.length, imported)
      : importWindowNotice
        ? "Recent messages loaded"
        : "Ready";

  useEffect(() => {
    const canvas = messageCanvasRef.current;
    if (!canvas) {
      return;
    }

    canvas.scrollTop = canvas.scrollHeight;
  }, [timelineIdentity]);

  useLayoutEffect(() => {
    if (!window.matchMedia("(max-width: 760px)").matches) {
      return;
    }

    conversationViewRef.current?.scrollIntoView({ block: "start" });
  }, [imported.transcript_name, source?.chatId, source?.handle]);

  return (
    <div className="conversation-view" ref={conversationViewRef}>
      <header
        className="conversation-header"
        data-testid={TEST_IDS.conversationHeader}
        data-tauri-drag-region=""
      >
        <div className="header-identity">
          <Avatar title={title} />
          <div>
            <h2 data-testid={TEST_IDS.chatTitle}>{title}</h2>
            <span>
              {messageCountLabel(imported)} ·{" "}
              {imported.attachments.length.toLocaleString()} media files
            </span>
          </div>
        </div>
        <div className="header-actions">
          <button
            className="icon-button"
            type="button"
            onClick={onExportHtml}
            disabled={exportState.status === "exporting" || !canExportHtml}
            aria-label={canExportHtml ? "Export chat to HTML" : "HTML export is not available for this source"}
            data-testid={TEST_IDS.exportButton}
            title={canExportHtml ? "Export chat to HTML" : "HTML export is not available for this source"}
          >
            <Download />
          </button>
        </div>
      </header>
      <div className="theme-banner">
        <span>{profile.bannerLabel}</span>
        <div className="banner-tools">
          <label className="date-filter">
            <input
              type="date"
              value={selectedDate}
              onInput={handleDateFilterChange}
              onChange={handleDateFilterChange}
              aria-label="Filter messages by date"
              data-testid={TEST_IDS.dateFilterInput}
            />
          </label>
          {selectedDate ? (
            <button
              className="date-filter-clear"
              type="button"
              onClick={() => onSelectedDateChange("")}
              aria-label="Clear date filter"
            >
              <X />
            </button>
          ) : null}
          <strong className={`banner-state ${exportState.status}`}>{bannerLabel}</strong>
        </div>
      </div>
      <div className="message-canvas" ref={messageCanvasRef} data-testid={TEST_IDS.messageCanvas}>
        <div className="sync-pill">{profile.viewingLabel}</div>
        {exportState.message ? (
          <div className={`export-toast ${exportState.status}`}>{exportState.message}</div>
        ) : null}
        {backupSearchStatus.message ? (
          <div className={`scope-notice ${backupSearchStatus.status}`}>
            {backupSearchStatus.message}
          </div>
        ) : null}
        {importWindowNotice && backupSearchStatus.status === "idle" ? (
          <div className="scope-notice">
            {importWindowNotice}. Search and export use the loaded recent messages.
          </div>
        ) : null}
        <div className="day-pill">Today</div>
        {hiddenEarlierCount > 0 ? (
          <button
            className="show-earlier"
            type="button"
            onClick={onShowEarlier}
            data-testid={TEST_IDS.showEarlierButton}
          >
            Show {Math.min(hiddenEarlierCount, messageLimitStep).toLocaleString()} earlier messages
          </button>
        ) : null}
        {visibleMessages.length > 0 ? (
          <VirtualizedMessageTimeline
            messages={visibleMessages}
            source={source}
            attachmentMap={attachmentMap}
            scrollParentRef={messageCanvasRef}
            onOpenImagePreview={setImagePreview}
          />
        ) : (
          <div className="no-results">
            {hasActiveFilters ? "No messages match these filters." : "No messages to show."}
          </div>
        )}
      </div>
      {imagePreview ? (
        <ImagePreviewModal preview={imagePreview} onClose={() => setImagePreview(null)} />
      ) : null}
    </div>
  );
}

function VirtualizedMessageTimeline({
  messages,
  source,
  attachmentMap,
  scrollParentRef,
  onOpenImagePreview,
}: {
  messages: Message[];
  source: LoadedChatSource | null;
  attachmentMap: Map<string, Attachment>;
  scrollParentRef: RefObject<HTMLDivElement | null>;
  onOpenImagePreview: (preview: { dataUrl: string; alt: string; caption: string }) => void;
}) {
  const listRef = useRef<HTMLDivElement>(null);
  const [virtualWindow, setVirtualWindow] = useState(() =>
    createTrailingVirtualTimelineWindow(messages.length),
  );

  const updateVirtualWindow = useCallback(() => {
    const scrollParent = scrollParentRef.current;
    const list = listRef.current;
    if (!scrollParent || !list) {
      setVirtualWindow(createTrailingVirtualTimelineWindow(messages.length));
      return;
    }

    setVirtualWindow(createVirtualTimelineWindow({
      itemCount: messages.length,
      scrollTop: scrollParent.scrollTop,
      viewportHeight: scrollParent.clientHeight,
      listTop: list.offsetTop,
    }));
  }, [messages.length, scrollParentRef]);

  useLayoutEffect(() => {
    updateVirtualWindow();
  }, [updateVirtualWindow]);

  useEffect(() => {
    const scrollParent = scrollParentRef.current;
    if (!scrollParent) {
      return;
    }

    let animationFrame = 0;
    const scheduleUpdate = () => {
      cancelAnimationFrame(animationFrame);
      animationFrame = requestAnimationFrame(updateVirtualWindow);
    };

    scrollParent.addEventListener("scroll", scheduleUpdate, { passive: true });
    window.addEventListener("resize", scheduleUpdate);
    scheduleUpdate();

    return () => {
      cancelAnimationFrame(animationFrame);
      scrollParent.removeEventListener("scroll", scheduleUpdate);
      window.removeEventListener("resize", scheduleUpdate);
    };
  }, [scrollParentRef, updateVirtualWindow]);

  const renderedMessages = useMemo(
    () => messages.slice(virtualWindow.startIndex, virtualWindow.endIndex),
    [messages, virtualWindow.endIndex, virtualWindow.startIndex],
  );

  return (
    <div
      className="virtual-message-list"
      ref={listRef}
      data-testid={TEST_IDS.virtualMessageList}
      data-total-messages={messages.length}
      data-rendered-messages={virtualWindow.renderedCount}
    >
      <div
        className="virtual-message-spacer"
        style={{ height: virtualWindow.beforeHeight }}
        aria-hidden="true"
      />
      {renderedMessages.map((message) => (
        <MessageBubble
          key={message.id}
          message={message}
          source={source}
          onOpenImagePreview={onOpenImagePreview}
          attachmentMap={attachmentMap}
        />
      ))}
      <div
        className="virtual-message-spacer"
        style={{ height: virtualWindow.afterHeight }}
        aria-hidden="true"
      />
    </div>
  );
}

const MessageBubble = memo(function MessageBubble({
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
          onClick={() =>
            onOpenImagePreview({
              dataUrl: preview.dataUrl,
              alt: attachment.filename,
              caption: attachment.filename,
            })
          }
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

function ImagePreviewModal({
  preview,
  onClose,
}: {
  preview: { dataUrl: string; alt: string; caption: string };
  onClose: () => void;
}) {
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onClose();
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  return (
    <div className="preview-modal" role="dialog" aria-modal="true" aria-label={preview.caption}>
      <button className="preview-backdrop" type="button" onClick={onClose} aria-label="Close preview" />
      <figure className="preview-frame">
        <button className="preview-close" type="button" onClick={onClose} aria-label="Close preview">
          <X />
        </button>
        <img src={preview.dataUrl} alt={preview.alt} />
        <figcaption>{preview.caption}</figcaption>
      </figure>
    </div>
  );
}
