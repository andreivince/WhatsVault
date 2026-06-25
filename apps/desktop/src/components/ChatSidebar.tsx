import { FileArchive } from "lucide-react";
import { useMemo } from "react";

import type { ChatSummary } from "../domain/chat";
import { displayTimestamp, filterChats } from "../domain/chat";
import { DEFAULT_SOURCE_KIND, sourceProfile } from "../domain/source";
import type { Chat, IphoneBackupCandidate, LoadedChatSource } from "../models";
import { TEST_IDS } from "../testing/testIds";
import type {
  BackupChatListSearchStatus,
  BackupChatListWindow,
  BackupChatState,
  LoadState,
} from "../viewState";
import { Avatar } from "./Avatar";

export function ChatSidebar({
  activeBackupChatId,
  backupChatState,
  backupChats,
  backupChatListSearchStatus,
  backupChatListWindow,
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
  activeBackupChatId: string | null;
  backupChatState: BackupChatState;
  backupChats: Chat[];
  backupChatListSearchStatus: BackupChatListSearchStatus;
  backupChatListWindow: BackupChatListWindow;
  chatSummary: ChatSummary | null;
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
  const selectedBackupForChats = selectedBackup && backupChatState === "ready" ? selectedBackup : null;
  const hasBackupChats = selectedBackupForChats !== null;
  const hasLoadedContent = Boolean(chatSummary || hasBackupChats);
  const queryHasText = query.trim().length > 0;
  const usesBackendBackupChatResult = backupChatListSearchStatus.status === "ready";
  const filteredBackupChats = useMemo(
    () => usesBackendBackupChatResult ? backupChats : filterChats(backupChats, query),
    [backupChats, query, usesBackendBackupChatResult],
  );
  const activeBackupChat = sourceKind === "iphone_backup"
    ? backupChats.find((chat) => chat.id === activeBackupChatId)
      ?? backupChats.find((chat) => chat.title === chatSummary?.title)
      ?? null
    : null;
  const backupChatListWindowLabel = backupChatListWindow.isTruncated
    ? usesBackendBackupChatResult
      ? `Showing ${backupChatListWindow.limit.toLocaleString()} matching chats`
      : `Showing newest ${backupChatListWindow.limit.toLocaleString()} chats`
    : null;
  const emptyBackupChatLabel =
    backupChatListSearchStatus.status === "loading"
      ? "Searching backup chat names..."
      : backupChatListSearchStatus.status === "error"
        ? "Could not search all backup chat names. Showing loaded chats."
        : queryHasText
          ? "No chats match this search."
          : "No readable WhatsApp chats found.";
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
      <header className="sidebar-header" data-tauri-drag-region="">
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
      <div className="chat-list">
        {hasBackupChats && visibleBackupChats.length > 0 ? (
          <>
            {visibleBackupChats.map((chat) => (
              <button
                key={chat.id}
                className={`chat-row${sourceKind === "iphone_backup" && chatSummary?.title === chat.title ? " selected" : ""}`}
                type="button"
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
            ))}
            {backupChatListSearchStatus.message ? (
              <div className={`sidebar-list-note ${backupChatListSearchStatus.status}`}>
                {backupChatListSearchStatus.message}
              </div>
            ) : null}
            {backupChatListWindowLabel ? (
              <div className="sidebar-list-note">{backupChatListWindowLabel}</div>
            ) : null}
          </>
        ) : hasBackupChats ? (
          <div className="sidebar-empty">
            <span>{emptyBackupChatLabel}</span>
          </div>
        ) : chatSummary ? (
          <div className="chat-row selected" aria-current="true">
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
