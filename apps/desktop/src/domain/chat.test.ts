import { describe, expect, it } from "vitest";

import {
  createChatSummary,
  createMessageWindow,
  createExportFilename,
  deriveChatTitle,
  displayTimestamp,
  filterMessagesByDate,
  filterChats,
  filterMessages,
  messageDateKey,
  isOutgoingMessage,
} from "./chat";
import { createLoadedChatSource } from "./source";
import type { Chat, ChatImport, LoadedChatSource, Message } from "../models";

function message(id: string, body: string, sender: string | null = "Ana"): Message {
  return {
    id,
    body,
    sender,
    attachment_ids: [],
    timestamp: { raw: "01/02/2026, 09:15:00" },
  };
}

function chatImport(overrides: Partial<ChatImport> = {}): ChatImport {
  return {
    source_kind: "whatsapp_export_zip",
    transcript_name: "_chat.txt",
    messages: [message("1", "hello")],
    attachments: [],
    issues: [],
    ...overrides,
  };
}

function backupSource(): LoadedChatSource {
  return {
    kind: "iphone_backup",
    handle: "backup-source-1",
    displayName: "Example iPhone",
  };
}

function chat(overrides: Partial<Chat>): Chat {
  return {
    id: "chat-1",
    title: "Design Preview",
    latestMessage: "And the voice note stays local too.",
    latestMessageTimestamp: { raw: "01/02/2026, 09:15:00" },
    messageCount: 9,
    attachmentCount: 2,
    ...overrides,
  };
}

describe("chat domain helpers", () => {
  it("derives local chat titles without exposing full paths", () => {
    const source = createLoadedChatSource(
      "whatsapp_export_zip",
      "export-source-1",
      "WhatsApp Chat - Family.zip",
    );

    expect(deriveChatTitle("_chat.txt", source.displayName)).toBe("Family");
    expect(deriveChatTitle("_chat.txt", null)).toBe("Imported chat");
  });

  it("prefers selected chat titles for iPhone backup imports", () => {
    expect(deriveChatTitle("Backup Chat", "Example iPhone", "iphone_backup")).toBe(
      "Backup Chat",
    );

    expect(createChatSummary(
      chatImport({
        source_kind: "iphone_backup",
        transcript_name: "Backup Chat",
      }),
      backupSource(),
    )).toMatchObject({
      id: "iphone_backup:Backup Chat",
      title: "Backup Chat",
    });
  });

  it("creates bounded filesystem-safe HTML export names", () => {
    expect(createExportFilename("Family chat")).toBe("Family-chat.html");
    expect(createExportFilename(" ../bad <name> ")).toBe("bad-name.html");
    expect(createExportFilename("🔥")).toBe("whatsvault-chat.html");
  });

  it("filters messages by sender or body", () => {
    const messages = [message("1", "project question", "Ana"), message("2", "dinner", "Bruno")];

    expect(filterMessages(messages, "project").map((item) => item.id)).toEqual(["1"]);
    expect(filterMessages(messages, "bru").map((item) => item.id)).toEqual(["2"]);
    expect(filterMessages(messages, "  ")).toEqual(messages);
  });

  it("filters chat rows by title or latest message", () => {
    const chats = [
      chat({ id: "design", title: "Design Preview" }),
      chat({
        id: "project",
        title: "Project Archive",
        latestMessage: "The backup import path is ready for review.",
      }),
      chat({ id: "media", title: "Media Archive", latestMessage: null }),
    ];

    expect(filterChats(chats, "preview").map((item) => item.id)).toEqual(["design"]);
    expect(filterChats(chats, "ready review").map((item) => item.id)).toEqual(["project"]);
    expect(filterChats(chats, "  ")).toEqual(chats);
  });

  it("extracts stable date keys from supported timestamp shapes", () => {
    expect(messageDateKey(message("1", "a", "Ana"))).toBe("2026-01-02");
    expect(messageDateKey({
      ...message("2", "b"),
      timestamp: { raw: "2026-06-23, 07:35" },
    })).toBe("2026-06-23");
    expect(messageDateKey({
      ...message("3", "c"),
      timestamp: { raw: "06/23/2026, 7:35 AM" },
    })).toBe("2026-06-23");
    expect(messageDateKey({
      ...message("4", "d"),
      timestamp: { raw: "Unknown time" },
    })).toBeNull();
  });

  it("filters messages by selected calendar date", () => {
    const messages = [
      {
        ...message("1", "day one"),
        timestamp: { raw: "06/23/2026, 7:35 AM" },
      },
      {
        ...message("2", "day two"),
        timestamp: { raw: "06/24/2026, 8:00 AM" },
      },
      {
        ...message("3", "unknown"),
        timestamp: { raw: "Unknown time" },
      },
    ];

    expect(filterMessagesByDate(messages, "2026-06-23").map((item) => item.id)).toEqual(["1"]);
    expect(filterMessagesByDate(messages, "")).toEqual(messages);
  });

  it("keeps the latest message window stable", () => {
    const messages = [message("1", "a"), message("2", "b"), message("3", "c")];

    expect(createMessageWindow(messages, 2).map((item) => item.id)).toEqual(["2", "3"]);
    expect(createMessageWindow(messages, 0)).toEqual(messages);
  });

  it("detects common exported self sender labels", () => {
    expect(isOutgoingMessage(message("1", "reply", "You"))).toBe(true);
    expect(isOutgoingMessage(message("2", "reply", "Você"))).toBe(true);
    expect(isOutgoingMessage(message("3", "reply", "Ana"))).toBe(false);
  });

  it("formats common WhatsApp export timestamps compactly", () => {
    expect(displayTimestamp("01/02/2026, 09:15:00")).toBe("09:15");
    expect(displayTimestamp("02/01/2026, 9:15 PM")).toBe("9:15 PM");
  });
});
