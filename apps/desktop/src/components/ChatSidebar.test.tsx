import { renderToStaticMarkup } from "react-dom/server";
import { expect, it } from "vitest";
import { createDemoBackupCandidates, createDemoBackupChats, createDemoBackupImport } from "../domain/demo";
import { createChatSummary } from "../domain/chat";
import { createLoadedBackupSource } from "../domain/source";
import { ChatSidebar } from "./ChatSidebar";

it("selects only the active chat when two chats share a title", () => {
  const backup = createDemoBackupCandidates()[0];
  const first = createDemoBackupChats()[0];
  const second = { ...first, id: "another-chat-with-the-same-title" };
  const source = createLoadedBackupSource(backup, first.id);
  const markup = renderToStaticMarkup(<ChatSidebar
    activeBackupChatId={first.id}
    backupChatState="ready"
    backupChats={[first, second]}
    backupChatListSearchStatus={{ status: "idle", message: null }}
    backupChatListWindow={{ isTruncated: false, limit: 200 }}
    chatSummary={createChatSummary(createDemoBackupImport(first), source)}
    openingBackupChatId={null}
    query=""
    selectedBackup={backup}
    sourceKind="iphone_backup"
    loadState="ready"
    onOpenBackupChat={() => {}}
    onQueryChange={() => {}}
    onOpenSource={() => {}}
    onChangeSource={() => {}}
    isOpeningSource={false}
  />);
  expect(markup.match(/class="chat-row selected"/g)).toHaveLength(1);
  expect(markup.match(/aria-current="true"/g)).toHaveLength(1);
});
