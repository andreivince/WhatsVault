import {
  CircleEllipsis,
  Download,
  File,
  FileArchive,
  RefreshCw,
  Upload,
  X,
} from "lucide-react";
import { type ChangeEvent, type FormEvent, useEffect, useMemo, useRef, useState } from "react";

import {
  attachmentLabel,
  buildAttachmentMap,
  createChatSummary,
  createExportFilename,
  createMessageWindow,
  displayTimestamp,
  filterChats,
  filterMessages,
  filterMessagesByDate,
  isOutgoingMessage,
} from "./domain/chat";
import {
  createDemoBackupCandidates,
  createDemoBackupChats,
  createDemoBackupImport,
  createDemoImport,
  createDemoLargeBackupImport,
} from "./domain/demo";
import { attachmentRenderKind, canRequestAttachmentPreview } from "./domain/media";
import {
  backupMetadataLine,
  backupReadiness,
  createDemoChatSource,
  createLoadedBackupSource,
  DEFAULT_SOURCE_KIND,
  sourceProfile,
} from "./domain/source";
import type {
  Attachment,
  AttachmentPreview,
  Chat,
  ChatImport,
  IphoneBackupCandidate,
  LoadedChatSource,
  Message,
} from "./models";
import {
  exportLocalChatHtml,
  importIphoneBackupChat,
  isDesktopRuntime,
  listIphoneBackupChats,
  listIphoneBackups,
  openLocalChatSource,
  readLocalAttachmentPreview,
} from "./services/desktop";
import { TEST_IDS } from "./testing/testIds";

const INITIAL_MESSAGE_LIMIT = 420;
const MESSAGE_LIMIT_STEP = 420;

type LoadState = "idle" | "loading" | "ready" | "error";
type BackupScanState = "idle" | "loading" | "ready" | "error";
type BackupChatState = "idle" | "loading" | "ready" | "error";
type ExportState = {
  status: "idle" | "exporting" | "success" | "error";
  message: string | null;
};

export function App() {
  const [source, setSource] = useState<LoadedChatSource | null>(null);
  const [imported, setImported] = useState<ChatImport | null>(null);
  const [query, setQuery] = useState("");
  const [loadState, setLoadState] = useState<LoadState>("idle");
  const [backupScanState, setBackupScanState] = useState<BackupScanState>("idle");
  const [backupScanError, setBackupScanError] = useState<string | null>(null);
  const [backupCandidates, setBackupCandidates] = useState<IphoneBackupCandidate[]>([]);
  const [selectedBackup, setSelectedBackup] = useState<IphoneBackupCandidate | null>(null);
  const [backupChats, setBackupChats] = useState<Chat[]>([]);
  const [backupChatState, setBackupChatState] = useState<BackupChatState>("idle");
  const [backupChatError, setBackupChatError] = useState<string | null>(null);
  const [openingBackupChatId, setOpeningBackupChatId] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [selectedDate, setSelectedDate] = useState("");
  const [messageLimit, setMessageLimit] = useState(INITIAL_MESSAGE_LIMIT);
  const [exportState, setExportState] = useState<ExportState>({
    status: "idle",
    message: null,
  });
  const demoMode = useMemo(() => new URLSearchParams(window.location.search).get("demo"), []);

  useEffect(() => {
    if (demoMode === "1") {
      setImported(createDemoImport());
      setSource(createDemoChatSource());
      setLoadState("ready");
      setBackupScanState("ready");
      setBackupCandidates([]);
      return;
    }

    if (demoMode === "backups") {
      const backups = createDemoBackupCandidates();
      setLoadState("idle");
      setBackupScanState("ready");
      setBackupCandidates(backups);
      setSelectedBackup(backups[0] ?? null);
      setBackupChats(createDemoBackupChats());
      setBackupChatState("ready");
      return;
    }

    if (demoMode === "backup-chat") {
      const backups = createDemoBackupCandidates();
      const chats = createDemoBackupChats();
      const backup = backups[0] ?? null;
      const chat = chats[0] ?? null;

      setBackupScanState("ready");
      setBackupCandidates(backups);
      setSelectedBackup(backup);
      setBackupChats(chats);
      setBackupChatState("ready");

      if (backup && chat) {
        setSource(createLoadedBackupSource(backup, chat.id));
        setImported(createDemoBackupImport(chat));
        setLoadState("ready");
      }
      return;
    }

    if (demoMode === "large-chat") {
      const backups = createDemoBackupCandidates();
      const backup = backups[0] ?? null;

      setBackupScanState("ready");
      setBackupCandidates(backups);
      setSelectedBackup(backup);
      setBackupChatState("ready");

      if (backup) {
        setSource(createLoadedBackupSource(backup, "demo-large-chat"));
        setImported(createDemoLargeBackupImport());
        setLoadState("ready");
      }
      return;
    }

    refreshBackups();
  }, [demoMode]);

  const attachmentMap = useMemo(
    () => buildAttachmentMap(imported?.attachments ?? []),
    [imported?.attachments],
  );
  const chatSummary = useMemo(
    () => (imported ? createChatSummary(imported, source) : null),
    [imported, source],
  );
  const dateFilteredMessages = useMemo(
    () => filterMessagesByDate(imported?.messages ?? [], selectedDate),
    [imported?.messages, selectedDate],
  );
  const filteredMessages = useMemo(
    () => filterMessages(dateFilteredMessages, query),
    [dateFilteredMessages, query],
  );
  const visibleMessages = useMemo(() => {
    if (query.trim() || selectedDate) {
      return filteredMessages;
    }

    return createMessageWindow(filteredMessages, messageLimit);
  }, [filteredMessages, messageLimit, query, selectedDate]);
  const hiddenEarlierCount = Math.max(0, filteredMessages.length - visibleMessages.length);

  async function openSource() {
    setErrorMessage(null);
    setLoadState("loading");

    try {
      const result = await openLocalChatSource();
      if (!result) {
        setLoadState(imported ? "ready" : "idle");
        return;
      }

      setSource(result.source);
      setImported(result.imported);
      setSelectedBackup(null);
      setBackupChats([]);
      setBackupChatState("idle");
      setBackupChatError(null);
      setQuery("");
      setSelectedDate("");
      setMessageLimit(INITIAL_MESSAGE_LIMIT);
      setExportState({ status: "idle", message: null });
      setLoadState("ready");
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : String(error));
      setLoadState("error");
    }
  }

  async function refreshBackups() {
    setBackupScanError(null);
    setBackupScanState("loading");
    setSelectedBackup(null);
    setBackupChats([]);
    setBackupChatState("idle");
    setBackupChatError(null);

    try {
      const candidates = await listIphoneBackups();
      setBackupCandidates(candidates);
      setBackupScanState("ready");
    } catch (error) {
      setBackupScanError(error instanceof Error ? error.message : String(error));
      setBackupCandidates([]);
      setBackupScanState("error");
    }
  }

  async function selectBackup(backup: IphoneBackupCandidate) {
    if (backupReadiness(backup).tone !== "ready") {
      return;
    }

    setSelectedBackup(backup);
    setBackupChatError(null);
    setBackupChatState("loading");

    try {
      const chats = demoMode === "backups" || demoMode === "backup-chat"
        ? createDemoBackupChats()
        : await listIphoneBackupChats(backup);
      setBackupChats(chats);
      setBackupChatState("ready");
    } catch (error) {
      setBackupChats([]);
      setBackupChatError(error instanceof Error ? error.message : String(error));
      setBackupChatState("error");
    }
  }

  async function openBackupChat(backup: IphoneBackupCandidate, chat: Chat) {
    setErrorMessage(null);
    setOpeningBackupChatId(chat.id);
    setLoadState("loading");

    try {
      const result = demoMode === "backups" || demoMode === "backup-chat"
        ? {
            source: createLoadedBackupSource(backup, chat.id),
            imported: createDemoBackupImport(chat),
          }
        : await importIphoneBackupChat(backup, chat.id);

      setSource(result.source);
      setImported(result.imported);
      setQuery("");
      setSelectedDate("");
      setMessageLimit(INITIAL_MESSAGE_LIMIT);
      setExportState({ status: "idle", message: null });
      setLoadState("ready");
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : String(error));
      setLoadState("error");
    } finally {
      setOpeningBackupChatId(null);
    }
  }

  async function exportCurrentChat() {
    if (!source || !chatSummary) {
      setExportState({
        status: "error",
        message: "Open a local chat source before exporting HTML.",
      });
      return;
    }

    if (!sourceProfile(source.kind).supportsHtmlExport) {
      setExportState({
        status: "error",
        message: "HTML export is not available for this source yet.",
      });
      return;
    }

    setExportState({ status: "exporting", message: "Preparing HTML export..." });

    try {
      const result = await exportLocalChatHtml(
        source,
        createExportFilename(chatSummary.title),
        chatSummary.title,
      );
      if (!result) {
        setExportState({ status: "idle", message: null });
        return;
      }

      const skippedNote =
        result.skippedAttachmentCount > 0
          ? ` · ${result.skippedAttachmentCount.toLocaleString()} media files listed only`
          : "";

      setExportState({
        status: "success",
        message: `${result.embeddedAttachmentCount.toLocaleString()} media files embedded${skippedNote}`,
      });
    } catch (error) {
      setExportState({
        status: "error",
        message: error instanceof Error ? error.message : String(error),
      });
    }
  }

  return (
    <main className="app-shell" data-testid={TEST_IDS.appShell}>
      <ChatSidebar
        backupChatState={backupChatState}
        backupChats={backupChats}
        chatSummary={chatSummary}
        openingBackupChatId={openingBackupChatId}
        query={query}
        selectedBackup={selectedBackup}
        sourceKind={source?.kind ?? null}
        loadState={loadState}
        onQueryChange={setQuery}
        onOpenBackupChat={openBackupChat}
        onOpenSource={openSource}
      />
      <section className="conversation-shell">
        {imported && chatSummary ? (
          <ConversationView
            imported={imported}
            source={source}
            title={chatSummary.title}
            query={query}
            selectedDate={selectedDate}
            visibleMessages={visibleMessages}
            hiddenEarlierCount={hiddenEarlierCount}
            attachmentMap={attachmentMap}
            exportState={exportState}
            onSelectedDateChange={setSelectedDate}
            onExportHtml={exportCurrentChat}
            onOpenSource={openSource}
            onShowEarlier={() => setMessageLimit((current) => current + MESSAGE_LIMIT_STEP)}
          />
        ) : (
          <EmptyConversation
            loadState={loadState}
            backupCandidates={backupCandidates}
            backupChats={backupChats}
            backupChatError={backupChatError}
            backupChatState={backupChatState}
            backupScanError={backupScanError}
            backupScanState={backupScanState}
            errorMessage={errorMessage}
            openingBackupChatId={openingBackupChatId}
            selectedBackup={selectedBackup}
            onOpenBackupChat={openBackupChat}
            onOpenSource={openSource}
            onRefreshBackups={refreshBackups}
            onSelectBackup={selectBackup}
          />
        )}
      </section>
    </main>
  );
}

function ChatSidebar({
  backupChatState,
  backupChats,
  chatSummary,
  openingBackupChatId,
  query,
  selectedBackup,
  sourceKind,
  loadState,
  onOpenBackupChat,
  onQueryChange,
  onOpenSource,
}: {
  backupChatState: BackupChatState;
  backupChats: Chat[];
  chatSummary: ReturnType<typeof createChatSummary> | null;
  openingBackupChatId: string | null;
  query: string;
  selectedBackup: IphoneBackupCandidate | null;
  sourceKind: LoadedChatSource["kind"] | null;
  loadState: LoadState;
  onOpenBackupChat: (backup: IphoneBackupCandidate, chat: Chat) => void;
  onQueryChange: (value: string) => void;
  onOpenSource: () => void;
}) {
  const profile = sourceProfile(DEFAULT_SOURCE_KIND);
  const selectedBackupForChats = selectedBackup && backupChats.length > 0 ? selectedBackup : null;
  const hasBackupChats = selectedBackupForChats !== null;
  const hasLoadedContent = Boolean(chatSummary || hasBackupChats);
  const queryHasText = query.trim().length > 0;
  const filteredBackupChats = useMemo(
    () => filterChats(backupChats, query),
    [backupChats, query],
  );
  const activeBackupChat = sourceKind === "iphone_backup"
    ? backupChats.find((chat) => chat.title === chatSummary?.title) ?? null
    : null;
  const visibleBackupChats = useMemo(() => {
    if (!queryHasText || !activeBackupChat) {
      return filteredBackupChats;
    }

    if (filteredBackupChats.some((chat) => chat.id === activeBackupChat.id)) {
      return filteredBackupChats;
    }

    return [activeBackupChat, ...filteredBackupChats];
  }, [activeBackupChat, filteredBackupChats, queryHasText]);

  return (
    <aside className="chat-sidebar">
      <header className="sidebar-header">
        <h1>Chats</h1>
      </header>
      <label className="search-box">
        <input
          value={query}
          onChange={(event) => onQueryChange(event.target.value)}
          placeholder="Search"
          aria-label="Search chats and messages"
          data-testid={TEST_IDS.searchInput}
        />
      </label>
      {hasLoadedContent ? (
        <button
          className="import-strip"
          type="button"
          onClick={onOpenSource}
          data-testid={TEST_IDS.openSourceButton}
        >
          <FileArchive />
          <span>{loadState === "loading" ? profile.loadingLabel : profile.openActionLabel}</span>
        </button>
      ) : null}
      <div className="chat-list" role="list">
        {hasBackupChats && visibleBackupChats.length > 0 ? (
          visibleBackupChats.map((chat) => (
            <button
              key={chat.id}
              className={`chat-row${sourceKind === "iphone_backup" && chatSummary?.title === chat.title ? " selected" : ""}`}
              type="button"
              role="listitem"
              disabled={openingBackupChatId === chat.id}
              onClick={() => onOpenBackupChat(selectedBackupForChats, chat)}
            >
              <Avatar title={chat.title} />
              <span className="chat-row-main">
                <span className="chat-row-title">{chat.title}</span>
                <span className="chat-row-subtitle">
                  {chat.latestMessage ?? `${chat.messageCount.toLocaleString()} messages`}
                </span>
              </span>
              <span className="chat-row-meta">
                <span>{chat.latestMessageTimestamp ? displayTimestamp(chat.latestMessageTimestamp.raw) : ""}</span>
                {chat.attachmentCount > 0 ? (
                  <span className="chat-row-media">{chat.attachmentCount.toLocaleString()} media</span>
                ) : null}
              </span>
            </button>
          ))
        ) : hasBackupChats ? (
          <div className="sidebar-empty">
            <span>No chats match this search.</span>
          </div>
        ) : chatSummary ? (
          <div className="chat-row selected" role="listitem" aria-current="true">
            <Avatar title={chatSummary.title} />
            <span className="chat-row-main">
              <span className="chat-row-title">{chatSummary.title}</span>
              <span className="chat-row-subtitle">{chatSummary.subtitle}</span>
            </span>
            <span className="chat-row-meta">
              <span>{chatSummary.latestTime}</span>
            </span>
          </div>
        ) : (
          <div className="sidebar-empty">
            <span>{backupChatState === "loading" ? "Loading backup chats..." : profile.emptyLabel}</span>
          </div>
        )}
      </div>
    </aside>
  );
}

function Avatar({ title }: { title: string }) {
  const initials = title
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase())
    .join("");

  return <span className="avatar">{initials || "WV"}</span>;
}

function EmptyConversation({
  loadState,
  backupCandidates,
  backupChats,
  backupChatError,
  backupChatState,
  backupScanError,
  backupScanState,
  errorMessage,
  openingBackupChatId,
  selectedBackup,
  onOpenBackupChat,
  onOpenSource,
  onRefreshBackups,
  onSelectBackup,
}: {
  loadState: LoadState;
  backupCandidates: IphoneBackupCandidate[];
  backupChats: Chat[];
  backupChatError: string | null;
  backupChatState: BackupChatState;
  backupScanError: string | null;
  backupScanState: BackupScanState;
  errorMessage: string | null;
  openingBackupChatId: string | null;
  selectedBackup: IphoneBackupCandidate | null;
  onOpenBackupChat: (backup: IphoneBackupCandidate, chat: Chat) => void;
  onOpenSource: () => void;
  onRefreshBackups: () => void;
  onSelectBackup: (backup: IphoneBackupCandidate) => void;
}) {
  const isBrowserPreview = !isDesktopRuntime();

  return (
    <div className="empty-conversation">
      <h2>WhatsVault</h2>
      <p>Local WhatsApp viewer</p>
      <SourceOverview loadState={loadState} onOpenSource={onOpenSource} />
      {errorMessage ? <p className="error-text">{errorMessage}</p> : null}
      {isBrowserPreview ? <p className="muted-note">Desktop runtime required for file access.</p> : null}
      <div className="encryption-note">
        <span>
          Private files stay local to this device.
        </span>
      </div>
      <BackupDiscoveryPanel
        backups={backupCandidates}
        backupChats={backupChats}
        chatErrorMessage={backupChatError}
        chatState={backupChatState}
        errorMessage={backupScanError}
        openingChatId={openingBackupChatId}
        scanState={backupScanState}
        selectedBackup={selectedBackup}
        onOpenChat={onOpenBackupChat}
        onRefresh={onRefreshBackups}
        onSelectBackup={onSelectBackup}
      />
    </div>
  );
}

function SourceOverview({
  loadState,
  onOpenSource,
}: {
  loadState: LoadState;
  onOpenSource: () => void;
}) {
  const exportProfile = sourceProfile("whatsapp_export_zip");
  const backupProfile = sourceProfile("iphone_backup");

  return (
    <section className="source-overview" aria-label="Source status" data-testid={TEST_IDS.sourceOverview}>
      <article
        className={`source-card ${exportProfile.availabilityTone}`}
        data-testid={TEST_IDS.supportedSourceCard}
      >
        <header>
          <span>
            <strong>{exportProfile.displayName}</strong>
            <small>{exportProfile.availabilityLabel}</small>
          </span>
        </header>
        <p>{exportProfile.availabilityDetail}</p>
        <button className="primary-action source-action" type="button" onClick={onOpenSource}>
          <Upload />
          <span>{loadState === "loading" ? exportProfile.loadingLabel : exportProfile.openActionLabel}</span>
        </button>
      </article>
      <article
        className={`source-card ${backupProfile.availabilityTone}`}
        data-testid={TEST_IDS.proofSourceCard}
      >
        <header>
          <span>
            <strong>{backupProfile.displayName}</strong>
            <small>{backupProfile.availabilityLabel}</small>
          </span>
        </header>
        <p>{backupProfile.availabilityDetail}</p>
      </article>
    </section>
  );
}

function BackupDiscoveryPanel({
  backups,
  backupChats,
  chatErrorMessage,
  chatState,
  errorMessage,
  openingChatId,
  scanState,
  selectedBackup,
  onOpenChat,
  onRefresh,
  onSelectBackup,
}: {
  backups: IphoneBackupCandidate[];
  backupChats: Chat[];
  chatErrorMessage: string | null;
  chatState: BackupChatState;
  errorMessage: string | null;
  openingChatId: string | null;
  scanState: BackupScanState;
  selectedBackup: IphoneBackupCandidate | null;
  onOpenChat: (backup: IphoneBackupCandidate, chat: Chat) => void;
  onRefresh: () => void;
  onSelectBackup: (backup: IphoneBackupCandidate) => void;
}) {
  const isScanning = scanState === "loading";

  return (
    <section className="backup-panel" aria-label="Detected iPhone backups">
      <header className="backup-panel-header">
        <span>iPhone backups</span>
        <button className="icon-button compact" type="button" onClick={onRefresh} aria-label="Refresh backups">
          <RefreshCw className={isScanning ? "spin" : ""} />
        </button>
      </header>
      {errorMessage ? <p className="backup-panel-error">{errorMessage}</p> : null}
      {!errorMessage && backups.length === 0 ? (
        <div className="backup-empty">
          <span>{isScanning ? "Scanning default backup folders..." : "No local iPhone backups detected"}</span>
          <small>Finder, Apple Devices, and iTunes backup locations are checked locally.</small>
        </div>
      ) : null}
      {backups.length > 0 ? (
        <div className="backup-list">
          {backups.map((backup) => {
            const isSelected = selectedBackup?.handle === backup.handle;

            return (
              <div className="backup-list-item" key={backup.handle}>
                <BackupCandidateRow
                  backup={backup}
                  expanded={isSelected}
                  onSelectBackup={onSelectBackup}
                />
                {isSelected ? (
                  <BackupChatDrawer
                    backup={backup}
                    chats={backupChats}
                    errorMessage={chatErrorMessage}
                    openingChatId={openingChatId}
                    state={chatState}
                    onOpenChat={onOpenChat}
                  />
                ) : null}
              </div>
            );
          })}
        </div>
      ) : null}
    </section>
  );
}

function BackupCandidateRow({
  backup,
  expanded,
  onSelectBackup,
}: {
  backup: IphoneBackupCandidate;
  expanded: boolean;
  onSelectBackup: (backup: IphoneBackupCandidate) => void;
}) {
  const readiness = backupReadiness(backup);
  const canOpen = readiness.tone === "ready";

  return (
    <button
      className={`backup-row${canOpen ? " openable" : ""}`}
      type="button"
      disabled={!canOpen}
      aria-expanded={canOpen ? expanded : undefined}
      onClick={() => onSelectBackup(backup)}
    >
      <Avatar title={backup.displayName} />
      <span className="backup-row-main">
        <span className="backup-row-title">{backup.displayName}</span>
        <span className="backup-row-subtitle">{backupMetadataLine(backup)}</span>
        <span className="backup-row-detail">{readiness.detail}</span>
      </span>
      <span className={`backup-status ${readiness.tone}`}>{readiness.label}</span>
    </button>
  );
}

function BackupChatDrawer({
  backup,
  chats,
  errorMessage,
  openingChatId,
  state,
  onOpenChat,
}: {
  backup: IphoneBackupCandidate;
  chats: Chat[];
  errorMessage: string | null;
  openingChatId: string | null;
  state: BackupChatState;
  onOpenChat: (backup: IphoneBackupCandidate, chat: Chat) => void;
}) {
  if (state === "loading") {
    return <div className="backup-chat-drawer muted">Loading chats from this backup...</div>;
  }

  if (errorMessage) {
    return <div className="backup-chat-drawer error">{errorMessage}</div>;
  }

  if (state === "ready" && chats.length === 0) {
    return <div className="backup-chat-drawer muted">No readable WhatsApp chats found.</div>;
  }

  return (
    <div className="backup-chat-drawer" role="list" aria-label={`${backup.displayName} chats`}>
      {chats.slice(0, 5).map((chat) => (
        <button
          className="backup-chat-row"
          type="button"
          key={chat.id}
          role="listitem"
          disabled={openingChatId === chat.id}
          onClick={() => onOpenChat(backup, chat)}
        >
          <Avatar title={chat.title} />
          <span className="backup-chat-main">
            <span className="backup-chat-title">{chat.title}</span>
            <span className="backup-chat-subtitle">
              {chat.latestMessage ?? `${chat.messageCount.toLocaleString()} messages`}
            </span>
          </span>
          <span className="backup-chat-meta">
            {chat.latestMessageTimestamp ? displayTimestamp(chat.latestMessageTimestamp.raw) : ""}
          </span>
        </button>
      ))}
      {chats.length > 5 ? (
        <div className="backup-chat-more">{chats.length - 5} more chats in sidebar</div>
      ) : null}
    </div>
  );
}

function ConversationView({
  imported,
  source,
  title,
  query,
  selectedDate,
  visibleMessages,
  hiddenEarlierCount,
  attachmentMap,
  onOpenSource,
  onExportHtml,
  onShowEarlier,
  onSelectedDateChange,
  exportState,
}: {
  imported: ChatImport;
  source: LoadedChatSource | null;
  title: string;
  query: string;
  selectedDate: string;
  visibleMessages: Message[];
  hiddenEarlierCount: number;
  attachmentMap: Map<string, Attachment>;
  onOpenSource: () => void;
  onExportHtml: () => void;
  onShowEarlier: () => void;
  onSelectedDateChange: (value: string) => void;
  exportState: ExportState;
}) {
  const profile = sourceProfile(source?.kind ?? DEFAULT_SOURCE_KIND);
  const canExportHtml = profile.supportsHtmlExport;
  const [imagePreview, setImagePreview] = useState<{
    dataUrl: string;
    alt: string;
    caption: string;
  } | null>(null);
  const messageCanvasRef = useRef<HTMLDivElement>(null);
  const hasActiveFilters = Boolean(query.trim() || selectedDate);
  const handleDateFilterChange = (
    event: ChangeEvent<HTMLInputElement> | FormEvent<HTMLInputElement>,
  ) => {
    onSelectedDateChange(event.currentTarget.value);
  };
  const bannerLabel = exportState.status === "exporting"
    ? "Exporting..."
    : hasActiveFilters
      ? `${visibleMessages.length.toLocaleString()} matches`
      : "Ready";

  useEffect(() => {
    const canvas = messageCanvasRef.current;
    if (!canvas) {
      return;
    }

    canvas.scrollTop = canvas.scrollHeight;
  }, [imported.messages.length, imported.transcript_name, source?.handle]);

  return (
    <div className="conversation-view">
      <header className="conversation-header" data-testid={TEST_IDS.conversationHeader}>
        <div className="header-identity">
          <Avatar title={title} />
          <div>
            <h2 data-testid={TEST_IDS.chatTitle}>{title}</h2>
            <span>
              {imported.messages.length.toLocaleString()} messages ·{" "}
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
          <button className="icon-button" type="button" onClick={onOpenSource} aria-label="Open another source">
            <CircleEllipsis />
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
        <div className="day-pill">Today</div>
        {hiddenEarlierCount > 0 ? (
          <button
            className="show-earlier"
            type="button"
            onClick={onShowEarlier}
            data-testid={TEST_IDS.showEarlierButton}
          >
            Show {Math.min(hiddenEarlierCount, MESSAGE_LIMIT_STEP).toLocaleString()} earlier messages
          </button>
        ) : null}
        {visibleMessages.length > 0 ? (
          visibleMessages.map((message) => (
            <MessageBubble
              key={message.id}
              message={message}
              source={source}
              onOpenImagePreview={setImagePreview}
              attachments={message.attachment_ids
                .map((id) => attachmentMap.get(id))
                .filter((attachment): attachment is Attachment => Boolean(attachment))}
            />
          ))
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

function MessageBubble({
  message,
  source,
  attachments,
  onOpenImagePreview,
}: {
  message: Message;
  source: LoadedChatSource | null;
  attachments: Attachment[];
  onOpenImagePreview: (preview: { dataUrl: string; alt: string; caption: string }) => void;
}) {
  const outgoing = isOutgoingMessage(message);

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
}

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
        const nextPreview = await readLocalAttachmentPreview(source, attachment);
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
