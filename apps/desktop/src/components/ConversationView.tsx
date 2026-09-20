import { Download, X } from "lucide-react";
import {
  type ChangeEvent,
  type FormEvent,
  useLayoutEffect,
  useRef,
  useState,
} from "react";

import {
  messageCountLabel,
  messageDateRangeLabel,
  messageFilterResultLabel,
  messageWindowNotice,
} from "../domain/chat";
import { DEFAULT_SOURCE_KIND, sourceProfile } from "../domain/source";
import type {
  Attachment,
  ChatImport,
  LoadedChatSource,
  Message,
} from "../models";
import { TEST_IDS } from "../testing/testIds";
import type { ConversationBackupSearchStatus, ExportState } from "../viewState";
import { Avatar } from "./Avatar";
import { ImagePreviewModal } from "./ImagePreviewModal";
import { MessageTimeline } from "./MessageTimeline";

export function ConversationView({
  imported,
  loadError,
  isLoading,
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
  loadError: string | null;
  isLoading: boolean;
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
  const dateRangeLabel = messageDateRangeLabel(visibleMessages);
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
            disabled={isLoading || exportState.status === "exporting" || !canExportHtml}
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
      {loadError ? <div className="scope-notice error" role="alert">{loadError}</div> : null}
      {isLoading ? <div className="scope-notice loading" role="status">Opening conversation...</div> : null}
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
            {importWindowNotice}. {profile.loadedWindowDetail}
          </div>
        ) : null}
        {dateRangeLabel ? <div className="day-pill">{dateRangeLabel}</div> : null}
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
          <MessageTimeline
            timelineIdentity={timelineIdentity}
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
