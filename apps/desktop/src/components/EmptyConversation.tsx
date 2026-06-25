import { FileArchive, RefreshCw, Upload } from "lucide-react";

import { displayTimestamp } from "../domain/chat";
import {
  backupMetadataLine,
  backupReadiness,
  type BackupReadinessTone,
  sourceProfile,
} from "../domain/source";
import type { Chat, IphoneBackupCandidate } from "../models";
import { isDesktopRuntime } from "../services/desktop";
import { TEST_IDS } from "../testing/testIds";
import type {
  BackupChatListWindow,
  BackupChatState,
  BackupScanState,
  LoadState,
} from "../viewState";
import { Avatar } from "./Avatar";

export function EmptyConversation({
  loadState,
  backupCandidates,
  backupChats,
  backupChatListWindow,
  backupChatError,
  backupChatState,
  backupScanError,
  backupScanState,
  errorMessage,
  openingBackupChatId,
  selectedBackup,
  onChooseBackupFolder,
  onOpenBackupChat,
  onOpenSource,
  onRefreshBackups,
  onSelectBackup,
}: {
  loadState: LoadState;
  backupCandidates: IphoneBackupCandidate[];
  backupChats: Chat[];
  backupChatListWindow: BackupChatListWindow;
  backupChatError: string | null;
  backupChatState: BackupChatState;
  backupScanError: string | null;
  backupScanState: BackupScanState;
  errorMessage: string | null;
  openingBackupChatId: string | null;
  selectedBackup: IphoneBackupCandidate | null;
  onOpenBackupChat: (backup: IphoneBackupCandidate, chat: Chat) => void;
  onOpenSource: () => void;
  onChooseBackupFolder: () => void;
  onRefreshBackups: () => void;
  onSelectBackup: (backup: IphoneBackupCandidate) => void;
}) {
  const isBrowserPreview = !isDesktopRuntime();

  return (
    <div className="empty-conversation" data-tauri-drag-region="">
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
        backupChatListWindow={backupChatListWindow}
        chatErrorMessage={backupChatError}
        chatState={backupChatState}
        errorMessage={backupScanError}
        openingChatId={openingBackupChatId}
        scanState={backupScanState}
        selectedBackup={selectedBackup}
        onChooseFolder={onChooseBackupFolder}
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
  backupChatListWindow,
  chatErrorMessage,
  chatState,
  errorMessage,
  openingChatId,
  scanState,
  selectedBackup,
  onOpenChat,
  onChooseFolder,
  onRefresh,
  onSelectBackup,
}: {
  backups: IphoneBackupCandidate[];
  backupChats: Chat[];
  backupChatListWindow: BackupChatListWindow;
  chatErrorMessage: string | null;
  chatState: BackupChatState;
  errorMessage: string | null;
  openingChatId: string | null;
  scanState: BackupScanState;
  selectedBackup: IphoneBackupCandidate | null;
  onOpenChat: (backup: IphoneBackupCandidate, chat: Chat) => void;
  onChooseFolder: () => void;
  onRefresh: () => void;
  onSelectBackup: (backup: IphoneBackupCandidate) => void;
}) {
  const isScanning = scanState === "loading";

  return (
    <section className="backup-panel" aria-label="Detected iPhone backups">
      <header className="backup-panel-header" data-tauri-drag-region="">
        <span>iPhone backups</span>
        <div className="backup-panel-actions">
          <button className="secondary-action compact" type="button" onClick={onChooseFolder}>
            <FileArchive />
            <span>Choose folder</span>
          </button>
          <button className="icon-button compact" type="button" onClick={onRefresh} aria-label="Refresh backups">
            <RefreshCw className={isScanning ? "spin" : ""} />
          </button>
        </div>
      </header>
      {errorMessage ? <p className="backup-panel-error">{errorMessage}</p> : null}
      {!errorMessage && backups.length === 0 ? (
        <div className="backup-empty">
          <span>{isScanning ? "Scanning default backup folders..." : "No local iPhone backups detected"}</span>
          <small>Choose the Backup folder or a device backup folder to grant access.</small>
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
                    chatListWindow={backupChatListWindow}
                    errorMessage={chatErrorMessage}
                    openingChatId={openingChatId}
                    state={chatState}
                    onChooseFolder={onChooseFolder}
                    onOpenChat={onOpenChat}
                    onRefresh={onRefresh}
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

  return (
    <button
      className="backup-row openable"
      type="button"
      aria-expanded={expanded}
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
  chatListWindow,
  errorMessage,
  openingChatId,
  state,
  onChooseFolder,
  onOpenChat,
  onRefresh,
}: {
  backup: IphoneBackupCandidate;
  chats: Chat[];
  chatListWindow: BackupChatListWindow;
  errorMessage: string | null;
  openingChatId: string | null;
  state: BackupChatState;
  onChooseFolder: () => void;
  onOpenChat: (backup: IphoneBackupCandidate, chat: Chat) => void;
  onRefresh: () => void;
}) {
  const readiness = backupReadiness(backup);
  if (readiness.tone !== "ready") {
    return (
      <div className={`backup-chat-drawer unavailable ${readiness.tone}`}>
        <strong>{readiness.label}</strong>
        <span>{backupUnavailableGuidance(readiness.tone)}</span>
        <div className="backup-chat-actions">
          <button className="secondary-action compact" type="button" onClick={onRefresh}>
            <RefreshCw />
            <span>Refresh</span>
          </button>
          <button className="secondary-action compact" type="button" onClick={onChooseFolder}>
            <FileArchive />
            <span>Choose folder</span>
          </button>
        </div>
      </div>
    );
  }

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
    <div className="backup-chat-drawer" aria-label={`${backup.displayName} chats`}>
      {chats.slice(0, 5).map((chat) => (
        <button
          className="backup-chat-row"
          type="button"
          key={chat.id}
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
      {chatListWindow.isTruncated ? (
        <div className="backup-chat-more bounded">
          Showing newest {chatListWindow.limit.toLocaleString()} chats for performance
        </div>
      ) : null}
    </div>
  );
}

function backupUnavailableGuidance(tone: BackupReadinessTone): string {
  switch (tone) {
    case "blocked":
      return "WhatsVault cannot open this backup yet. Choose an unencrypted backup or make a new unencrypted local backup.";
    case "warning":
      return "WhatsApp data was not found in this backup. If the phone backup is still running, refresh when it finishes.";
    case "pending":
      return "This backup is still being inspected. Refresh in a moment or choose the device backup folder directly.";
    case "ready":
      return "";
  }
}
