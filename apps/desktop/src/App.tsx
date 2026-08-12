import { useEffect, useMemo, useRef, useState } from "react";

import {
  buildAttachmentMap,
  createChatSummary,
  createExportFilename,
  createMessageWindow,
  filterMessages,
  filterMessagesByDate,
  searchResultsNotice,
} from "./domain/chat";
import {
  createDemoBackupCandidates,
  createDemoBackupChats,
  createDemoBackupImport,
  createDemoImport,
  createDemoLargeBackupImport,
} from "./domain/demo";
import { ChatSidebar } from "./components/ChatSidebar";
import { ConversationView } from "./components/ConversationView";
import { EmptyConversation } from "./components/EmptyConversation";
import { WindowControls } from "./components/WindowControls";
import {
  backupReadiness,
  createDemoChatSource,
  createLoadedBackupSource,
  sourceProfile,
} from "./domain/source";
import type {
  Chat,
  ChatImport,
  IphoneBackupCandidate,
  LoadedChatSource,
} from "./models";
import {
  chooseIphoneBackupFolder,
  exportLocalChatHtml,
  importIphoneBackupChat,
  isDesktopRuntime,
  listIphoneBackupChats,
  listIphoneBackups,
  openLocalChatSource,
  searchIphoneBackupChat,
  searchIphoneBackupChats,
} from "./services/desktop";
import { createLatestRequestGate } from "./services/latestRequest";
import { TEST_IDS } from "./testing/testIds";
import type {
  BackupChatListSearchStatus,
  BackupChatListWindow,
  BackupChatSearchState,
  BackupChatState,
  BackupMessageSearchState,
  BackupScanState,
  ConversationBackupSearchStatus,
  ExportState,
  LoadState,
} from "./viewState";

const INITIAL_MESSAGE_LIMIT = 420;
const MESSAGE_LIMIT_STEP = 420;

const EMPTY_BACKUP_CHAT_LIST_WINDOW: BackupChatListWindow = {
  isTruncated: false,
  limit: 0,
};
const EMPTY_BACKUP_MESSAGE_SEARCH: BackupMessageSearchState = {
  status: "idle",
  query: "",
  result: null,
  message: null,
};
const EMPTY_BACKUP_CHAT_SEARCH: BackupChatSearchState = {
  status: "idle",
  query: "",
  result: null,
  message: null,
};
const EMPTY_CONVERSATION_BACKUP_SEARCH_STATUS: ConversationBackupSearchStatus = {
  status: "idle",
  message: null,
  isTruncated: false,
  limit: 0,
};
const EMPTY_BACKUP_CHAT_LIST_SEARCH_STATUS: BackupChatListSearchStatus = {
  status: "idle",
  message: null,
};
const BACKUP_SEARCH_DEBOUNCE_MS = 180;

export function App() {
  const backupSelectionRequests = useRef(createLatestRequestGate());
  const [source, setSource] = useState<LoadedChatSource | null>(null);
  const [imported, setImported] = useState<ChatImport | null>(null);
  const [query, setQuery] = useState("");
  const [loadState, setLoadState] = useState<LoadState>("idle");
  const [backupScanState, setBackupScanState] = useState<BackupScanState>("idle");
  const [backupScanError, setBackupScanError] = useState<string | null>(null);
  const [backupCandidates, setBackupCandidates] = useState<IphoneBackupCandidate[]>([]);
  const [selectedBackup, setSelectedBackup] = useState<IphoneBackupCandidate | null>(null);
  const [backupChats, setBackupChats] = useState<Chat[]>([]);
  const [backupChatListWindow, setBackupChatListWindow] = useState<BackupChatListWindow>(
    EMPTY_BACKUP_CHAT_LIST_WINDOW,
  );
  const [backupChatState, setBackupChatState] = useState<BackupChatState>("idle");
  const [backupChatError, setBackupChatError] = useState<string | null>(null);
  const [backupMessageSearch, setBackupMessageSearch] = useState<BackupMessageSearchState>(
    EMPTY_BACKUP_MESSAGE_SEARCH,
  );
  const [backupChatSearch, setBackupChatSearch] = useState<BackupChatSearchState>(
    EMPTY_BACKUP_CHAT_SEARCH,
  );
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
      setBackupChatListWindow(EMPTY_BACKUP_CHAT_LIST_WINDOW);
      return;
    }

    if (demoMode === "backups") {
      const backups = createDemoBackupCandidates();
      setLoadState("idle");
      setBackupScanState("ready");
      setBackupCandidates(backups);
      setSelectedBackup(backups[0] ?? null);
      setBackupChats(createDemoBackupChats());
      setBackupChatListWindow(EMPTY_BACKUP_CHAT_LIST_WINDOW);
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
      setBackupChatListWindow(EMPTY_BACKUP_CHAT_LIST_WINDOW);
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
      setBackupChatListWindow(EMPTY_BACKUP_CHAT_LIST_WINDOW);
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

  const normalizedQuery = query.trim();
  const desktopRuntime = isDesktopRuntime();

  useEffect(() => {
    if (
      !source ||
      source.kind !== "iphone_backup" ||
      !source.chatId ||
      !normalizedQuery ||
      selectedDate ||
      !desktopRuntime
    ) {
      setBackupMessageSearch(EMPTY_BACKUP_MESSAGE_SEARCH);
      return;
    }

    let cancelled = false;
    const searchQuery = normalizedQuery;
    setBackupMessageSearch({
      status: "loading",
      query: searchQuery,
      result: null,
      message: null,
    });

    const timer = window.setTimeout(async () => {
      try {
        const result = await searchIphoneBackupChat(source, searchQuery);
        if (!cancelled) {
          setBackupMessageSearch({
            status: "ready",
            query: searchQuery,
            result,
            message: null,
          });
        }
      } catch (error) {
        if (!cancelled) {
          setBackupMessageSearch({
            status: "error",
            query: searchQuery,
            result: null,
            message: error instanceof Error ? error.message : String(error),
          });
        }
      }
    }, BACKUP_SEARCH_DEBOUNCE_MS);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [desktopRuntime, source, normalizedQuery, selectedDate]);

  useEffect(() => {
    if (!desktopRuntime || !selectedBackup || backupChatState !== "ready" || !normalizedQuery) {
      setBackupChatSearch(EMPTY_BACKUP_CHAT_SEARCH);
      return;
    }

    let cancelled = false;
    const searchQuery = normalizedQuery;
    setBackupChatSearch({
      status: "loading",
      query: searchQuery,
      result: null,
      message: null,
    });

    const timer = window.setTimeout(async () => {
      try {
        const result = await searchIphoneBackupChats(selectedBackup, searchQuery);
        if (!cancelled) {
          setBackupChatSearch({
            status: "ready",
            query: searchQuery,
            result,
            message: null,
          });
        }
      } catch (error) {
        if (!cancelled) {
          setBackupChatSearch({
            status: "error",
            query: searchQuery,
            result: null,
            message: error instanceof Error ? error.message : String(error),
          });
        }
      }
    }, BACKUP_SEARCH_DEBOUNCE_MS);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [backupChatState, desktopRuntime, normalizedQuery, selectedBackup]);

  const isBackupMessageSearchEligible =
    desktopRuntime &&
    source?.kind === "iphone_backup" &&
    Boolean(source.chatId) &&
    Boolean(normalizedQuery) &&
    !selectedDate;
  const backupMessageSearchMatchesQuery =
    isBackupMessageSearchEligible && backupMessageSearch.query === normalizedQuery;
  const activeBackupSearchResult =
    backupMessageSearchMatchesQuery && backupMessageSearch.status === "ready"
      ? backupMessageSearch.result
      : null;
  const timelineImport = activeBackupSearchResult?.imported ?? imported;
  const backupSearchStatus = useMemo<ConversationBackupSearchStatus>(() => {
    if (!isBackupMessageSearchEligible) {
      return EMPTY_CONVERSATION_BACKUP_SEARCH_STATUS;
    }

    if (!backupMessageSearchMatchesQuery || backupMessageSearch.status === "loading") {
      return {
        status: "loading",
        message: "Searching the full selected backup chat...",
        isTruncated: false,
        limit: 0,
      };
    }

    if (backupMessageSearch.status === "error") {
      return {
        status: "error",
        message: "Could not search the full backup chat. Showing loaded recent messages instead.",
        isTruncated: false,
        limit: 0,
      };
    }

    if (!backupMessageSearch.result) {
      return EMPTY_CONVERSATION_BACKUP_SEARCH_STATUS;
    }

    return {
      status: "ready",
      message: searchResultsNotice(backupMessageSearch.result.imported),
      isTruncated: backupMessageSearch.result.isTruncated,
      limit: backupMessageSearch.result.limit,
    };
  }, [
    backupMessageSearch,
    backupMessageSearchMatchesQuery,
    isBackupMessageSearchEligible,
  ]);

  const isBackupChatSearchEligible =
    desktopRuntime &&
    selectedBackup !== null &&
    backupChatState === "ready" &&
    Boolean(normalizedQuery);
  const backupChatSearchMatchesQuery =
    isBackupChatSearchEligible && backupChatSearch.query === normalizedQuery;
  const activeBackupChatSearchResult =
    backupChatSearchMatchesQuery && backupChatSearch.status === "ready"
      ? backupChatSearch.result
      : null;
  const backupChatListSearchStatus = useMemo<BackupChatListSearchStatus>(() => {
    if (!isBackupChatSearchEligible) {
      return EMPTY_BACKUP_CHAT_LIST_SEARCH_STATUS;
    }

    if (!backupChatSearchMatchesQuery || backupChatSearch.status === "loading") {
      return {
        status: "loading",
        message: "Searching backup chat names...",
      };
    }

    if (backupChatSearch.status === "error") {
      return {
        status: "error",
        message: "Could not search all backup chat names. Showing loaded chats.",
      };
    }

    return {
      status: "ready",
      message: null,
    };
  }, [
    backupChatSearch.status,
    backupChatSearchMatchesQuery,
    isBackupChatSearchEligible,
  ]);

  const attachmentMap = useMemo(
    () => buildAttachmentMap(timelineImport?.attachments ?? []),
    [timelineImport?.attachments],
  );
  const chatSummary = useMemo(
    () => (imported ? createChatSummary(imported, source) : null),
    [imported, source],
  );
  const activeLoadedBackupChat = useMemo(() => {
    if (source?.kind !== "iphone_backup") {
      return null;
    }

    return (
      backupChats.find((chat) => chat.id === source.chatId) ??
      backupChats.find((chat) => chat.title === chatSummary?.title) ??
      null
    );
  }, [backupChats, chatSummary?.title, source]);
  const sidebarBackupChats = useMemo(() => {
    const searchedChats = activeBackupChatSearchResult?.chats;
    if (!searchedChats) {
      return backupChats;
    }

    if (
      !activeLoadedBackupChat ||
      searchedChats.some((chat) => chat.id === activeLoadedBackupChat.id)
    ) {
      return searchedChats;
    }

    return [activeLoadedBackupChat, ...searchedChats];
  }, [activeBackupChatSearchResult?.chats, activeLoadedBackupChat, backupChats]);
  const sidebarBackupChatListWindow = activeBackupChatSearchResult
    ? {
        isTruncated: activeBackupChatSearchResult.isTruncated,
        limit: activeBackupChatSearchResult.limit,
      }
    : backupChatListWindow;
  const dateFilteredMessages = useMemo(() => {
    if (activeBackupSearchResult) {
      return activeBackupSearchResult.imported.messages;
    }

    return filterMessagesByDate(imported?.messages ?? [], selectedDate);
  }, [activeBackupSearchResult, imported?.messages, selectedDate]);
  const filteredMessages = useMemo(() => {
    if (activeBackupSearchResult) {
      return activeBackupSearchResult.imported.messages;
    }

    return filterMessages(dateFilteredMessages, query);
  }, [activeBackupSearchResult, dateFilteredMessages, query]);
  const visibleMessages = useMemo(() => {
    if (activeBackupSearchResult || normalizedQuery || selectedDate) {
      return filteredMessages;
    }

    return createMessageWindow(filteredMessages, messageLimit);
  }, [activeBackupSearchResult, filteredMessages, messageLimit, normalizedQuery, selectedDate]);
  const hiddenEarlierCount = activeBackupSearchResult
    ? 0
    : Math.max(0, filteredMessages.length - visibleMessages.length);
  const timelineIdentity = activeBackupSearchResult
    ? `backup-search:${source?.handle ?? ""}:${source?.chatId ?? ""}:${normalizedQuery}:${visibleMessages.length}`
    : `import:${source?.handle ?? ""}:${source?.chatId ?? ""}:${imported?.messages.length ?? 0}`;

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
      setBackupChatListWindow(EMPTY_BACKUP_CHAT_LIST_WINDOW);
      setBackupChatState("idle");
      setBackupChatError(null);
      setQuery("");
      setSelectedDate("");
      setMessageLimit(INITIAL_MESSAGE_LIMIT);
      setBackupMessageSearch(EMPTY_BACKUP_MESSAGE_SEARCH);
      setBackupChatSearch(EMPTY_BACKUP_CHAT_SEARCH);
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
    resetBackupSelection();

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

  async function chooseBackupFolder() {
    setBackupScanError(null);
    setBackupScanState("loading");
    resetBackupSelection();

    try {
      const candidates = await chooseIphoneBackupFolder();
      if (!candidates) {
        setBackupScanState(backupCandidates.length > 0 ? "ready" : "idle");
        return;
      }

      setBackupCandidates(candidates);
      setBackupScanState("ready");
      if (candidates.length === 0) {
        setBackupScanError("No iPhone backups were found in the selected folder.");
        return;
      }

      const firstReadyBackup = candidates.find(
        (candidate) => backupReadiness(candidate).tone === "ready",
      );
      if (firstReadyBackup) {
        await selectBackup(firstReadyBackup);
      }
    } catch (error) {
      setBackupScanError(error instanceof Error ? error.message : String(error));
      setBackupCandidates([]);
      setBackupScanState("error");
    }
  }

  function resetBackupSelection() {
    backupSelectionRequests.current.invalidate();
    setSelectedBackup(null);
    setBackupChats([]);
    setBackupChatListWindow(EMPTY_BACKUP_CHAT_LIST_WINDOW);
    setBackupChatState("idle");
    setBackupChatError(null);
    setBackupChatSearch(EMPTY_BACKUP_CHAT_SEARCH);
  }

  async function selectBackup(backup: IphoneBackupCandidate) {
    const isCurrentRequest = backupSelectionRequests.current.begin();
    const readiness = backupReadiness(backup);
    setSelectedBackup(backup);
    setBackupChats([]);
    setBackupChatListWindow(EMPTY_BACKUP_CHAT_LIST_WINDOW);
    setBackupChatError(null);
    setBackupChatSearch(EMPTY_BACKUP_CHAT_SEARCH);

    if (readiness.tone !== "ready") {
      setBackupChatState("idle");
      return;
    }

    setBackupChatState("loading");

    try {
      const result = demoMode === "backups" || demoMode === "backup-chat"
        ? {
            chats: createDemoBackupChats(),
            isTruncated: false,
            limit: 0,
          }
        : await listIphoneBackupChats(backup);
      if (!isCurrentRequest()) {
        return;
      }

      setBackupChats(result.chats);
      setBackupChatListWindow({
        isTruncated: result.isTruncated,
        limit: result.limit,
      });
      setBackupChatState("ready");
    } catch (error) {
      if (!isCurrentRequest()) {
        return;
      }

      setBackupChats([]);
      setBackupChatListWindow(EMPTY_BACKUP_CHAT_LIST_WINDOW);
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
      setBackupMessageSearch(EMPTY_BACKUP_MESSAGE_SEARCH);
      setBackupChatSearch(EMPTY_BACKUP_CHAT_SEARCH);
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
      const messageWindowNote =
        result.skippedMessageCount > 0
          ? ` · latest ${result.exportedMessageCount.toLocaleString()} messages exported`
          : "";

      setExportState({
        status: "success",
        message: `${result.embeddedAttachmentCount.toLocaleString()} media files embedded${skippedNote}${messageWindowNote}`,
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
      <WindowControls />
      <ChatSidebar
        activeBackupChatId={source?.kind === "iphone_backup" ? source.chatId ?? null : null}
        backupChatState={backupChatState}
        backupChats={sidebarBackupChats}
        backupChatListSearchStatus={backupChatListSearchStatus}
        backupChatListWindow={sidebarBackupChatListWindow}
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
            backupSearchStatus={backupSearchStatus}
            messageLimitStep={MESSAGE_LIMIT_STEP}
            exportState={exportState}
            timelineIdentity={timelineIdentity}
            onSelectedDateChange={setSelectedDate}
            onExportHtml={exportCurrentChat}
            onShowEarlier={() => setMessageLimit((current) => current + MESSAGE_LIMIT_STEP)}
          />
        ) : (
          <EmptyConversation
            loadState={loadState}
            backupCandidates={backupCandidates}
            backupChats={backupChats}
            backupChatListWindow={backupChatListWindow}
            backupChatError={backupChatError}
            backupChatState={backupChatState}
            backupScanError={backupScanError}
            backupScanState={backupScanState}
            errorMessage={errorMessage}
            openingBackupChatId={openingBackupChatId}
            selectedBackup={selectedBackup}
            onOpenBackupChat={openBackupChat}
            onOpenSource={openSource}
            onChooseBackupFolder={chooseBackupFolder}
            onRefreshBackups={refreshBackups}
            onSelectBackup={selectBackup}
          />
        )}
      </section>
    </main>
  );
}
