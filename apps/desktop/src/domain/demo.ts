import type { Chat, ChatImport, IphoneBackupCandidate } from "../models";

const DEMO_PHOTO_PREVIEW_DATA_URL =
  "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCA3MjAgNDgwIiByb2xlPSJpbWciIGFyaWEtbGFiZWw9IlN5bnRoZXRpYyBhcmNoaXZlIHBob3RvIj48ZGVmcz48bGluZWFyR3JhZGllbnQgaWQ9InNreSIgeDE9IjAiIHgyPSIwIiB5MT0iMCIgeTI9IjEiPjxzdG9wIG9mZnNldD0iMCIgc3RvcC1jb2xvcj0iIzlkZDhmZiIvPjxzdG9wIG9mZnNldD0iMC42MiIgc3RvcC1jb2xvcj0iI2U4ZjVmZiIvPjxzdG9wIG9mZnNldD0iMSIgc3RvcC1jb2xvcj0iI2ZmZjdkZiIvPjwvbGluZWFyR3JhZGllbnQ+PGxpbmVhckdyYWRpZW50IGlkPSJoaWxsIiB4MT0iMCIgeDI9IjEiPjxzdG9wIG9mZnNldD0iMCIgc3RvcC1jb2xvcj0iIzZiYzI4ZiIvPjxzdG9wIG9mZnNldD0iMSIgc3RvcC1jb2xvcj0iIzJmOGM2NSIvPjwvbGluZWFyR3JhZGllbnQ+PC9kZWZzPjxyZWN0IHdpZHRoPSI3MjAiIGhlaWdodD0iNDgwIiByeD0iMjgiIGZpbGw9InVybCgjc2t5KSIvPjxjaXJjbGUgY3g9IjU2MCIgY3k9IjEwMiIgcj0iNDgiIGZpbGw9IiNmZmY0YTgiLz48cGF0aCBkPSJNMCAzMzJjOTYtODggMTc4LTEwNyAyNzYtNTEgOTQgNTQgMTYxIDM5IDI1MC0xNCA3MC00MiAxMzYtMzIgMTk0IDMxdjE4MkgweiIgZmlsbD0idXJsKCNoaWxsKSIvPjxwYXRoIGQ9Ik0wIDM4N2MxMTEtNTcgMjE0LTU4IDMxMi00IDg3IDQ4IDE5MyA0OCA0MDgtMjF2MTE4SDB6IiBmaWxsPSIjMTY2ZjUwIiBvcGFjaXR5PSIwLjgyIi8+PHBhdGggZD0iTTEwMiAzMzBsOTAtMTA4IDcwIDgyIDQ0LTUyIDExOCAxNDJINTR6IiBmaWxsPSIjZmZmZmZmIiBvcGFjaXR5PSIwLjg2Ii8+PHBhdGggZD0iTTExMiAzMzBsODAtOTQgNjIgNzQgNTEtMzQgODMgOTZIOTJ6IiBmaWxsPSIjY2NlYmQ5Ii8+PHJlY3QgeD0iNDIiIHk9IjM4IiB3aWR0aD0iMjEyIiBoZWlnaHQ9IjU0IiByeD0iMjciIGZpbGw9IiNmZmZmZmYiIG9wYWNpdHk9IjAuODQiLz48dGV4dCB4PSI3MCIgeT0iNzIiIGZvbnQtZmFtaWx5PSJJbnRlcixTZWdvZSBVSSxBcmlhbCxzYW5zLXNlcmlmIiBmb250LXNpemU9IjIyIiBmb250LXdlaWdodD0iODAwIiBmaWxsPSIjMDA2ZDNmIj5TeW50aGV0aWMgbWVkaWE8L3RleHQ+PC9zdmc+";

export function createDemoImport(): ChatImport {
  return {
    source_kind: "whatsapp_export_zip",
    transcript_name: "_chat.txt",
    attachments: [
      {
        id: "demo-photo",
        archive_path: "Media/demo-photo.jpg",
        filename: "demo-photo.jpg",
        kind: "photo",
        size_bytes: 184320,
        preview: {
          mediaType: "image/svg+xml",
          dataUrl: DEMO_PHOTO_PREVIEW_DATA_URL,
          sizeBytes: 1122,
        },
      },
      {
        id: "demo-audio",
        archive_path: "Media/demo-audio.opus",
        filename: "demo-audio.opus",
        kind: "audio",
        size_bytes: 32000,
      },
    ],
    issues: [],
    messages: [
      {
        id: "demo-001",
        timestamp: { raw: "06/23/2026, 7:35 AM" },
        sender: "Demo Contact",
        body: "I found the old travel photos in the WhatsApp export.",
        attachment_ids: [],
      },
      {
        id: "demo-002",
        timestamp: { raw: "06/23/2026, 7:35 AM" },
        sender: "Demo Contact",
        body: "It is wild how much context is locked inside a backup.",
        attachment_ids: [],
      },
      {
        id: "demo-003",
        timestamp: { raw: "06/23/2026, 7:36 AM" },
        sender: "You",
        body: "That is exactly why a local viewer should exist.",
        attachment_ids: [],
      },
      {
        id: "demo-004",
        timestamp: { raw: "06/23/2026, 7:36 AM" },
        sender: "Demo Contact",
        body: "Can it show the media without uploading anything?",
        attachment_ids: [],
      },
      {
        id: "demo-005",
        timestamp: { raw: "06/23/2026, 7:36 AM" },
        sender: "You",
        body: "Yes. The files stay local on this computer.",
        attachment_ids: [],
      },
      {
        id: "demo-006",
        timestamp: { raw: "06/23/2026, 7:36 AM" },
        sender: "You",
        body: "Search and export are the next useful pieces.",
        attachment_ids: [],
      },
      {
        id: "demo-007",
        timestamp: { raw: "06/23/2026, 7:36 AM" },
        sender: "Demo Contact",
        body: "Here is one photo from the archive.",
        attachment_ids: ["demo-photo"],
      },
      {
        id: "demo-008",
        timestamp: { raw: "06/23/2026, 7:36 AM" },
        sender: "You",
        body: "This is the moment the app starts feeling useful.",
        attachment_ids: [],
      },
      {
        id: "demo-009",
        timestamp: { raw: "06/23/2026, 7:37 AM" },
        sender: "Demo Contact",
        body: "And the voice note stays local too.",
        attachment_ids: ["demo-audio"],
      },
    ],
  };
}

export function createDemoBackupCandidates(): IphoneBackupCandidate[] {
  return [
    {
      handle: "demo-backup-ready",
      displayName: "Demo iPhone",
      productLabel: "iPhone 15 Pro",
      productVersion: "18.5",
      lastBackupDate: "2026-06-23T10:00:00Z",
      isEncrypted: false,
      hasInfoPlist: true,
      hasStatusPlist: true,
      hasManifestPlist: true,
      whatsapp: {
        manifestReadable: true,
        hasChatStorage: true,
        hasContacts: true,
        mediaFileCount: 1286,
      },
    },
    {
      handle: "demo-backup-no-whatsapp",
      displayName: "Travel Phone",
      productLabel: "iPhone 13",
      productVersion: "17.6",
      lastBackupDate: "2026-06-18T21:40:00Z",
      isEncrypted: false,
      hasInfoPlist: true,
      hasStatusPlist: true,
      hasManifestPlist: true,
      whatsapp: {
        manifestReadable: true,
        hasChatStorage: false,
        hasContacts: false,
        mediaFileCount: 0,
      },
    },
    {
      handle: "demo-backup-encrypted",
      displayName: "Encrypted iPhone",
      productLabel: "iPhone 12",
      productVersion: "16.7",
      lastBackupDate: "2026-05-30T08:15:00Z",
      isEncrypted: true,
      hasInfoPlist: true,
      hasStatusPlist: true,
      hasManifestPlist: true,
      whatsapp: {
        manifestReadable: true,
        hasChatStorage: true,
        hasContacts: false,
        mediaFileCount: 418,
      },
    },
  ];
}

export function createDemoBackupChats(): Chat[] {
  return [
    {
      id: "demo-backup-chat-1",
      title: "Design Preview",
      latestMessage: "And the voice note stays local too.",
      latestMessageTimestamp: { raw: "06/23/2026, 7:37 AM" },
      messageCount: 9,
      attachmentCount: 2,
    },
    {
      id: "demo-backup-chat-2",
      title: "Project Archive",
      latestMessage: "The backup import path is ready for review.",
      latestMessageTimestamp: { raw: "06/22/2026, 4:10 PM" },
      messageCount: 128,
      attachmentCount: 16,
    },
    {
      id: "demo-backup-chat-3",
      title: "Media Archive",
      latestMessage: "Photo from the old trip",
      latestMessageTimestamp: { raw: "06/20/2026, 9:02 AM" },
      messageCount: 84,
      attachmentCount: 39,
    },
  ];
}

export function createDemoBackupImport(chat: Chat): ChatImport {
  return {
    ...createDemoImport(),
    source_kind: "iphone_backup",
    transcript_name: chat.title,
  };
}

export function createDemoLargeBackupImport(messageCount = 900): ChatImport {
  return {
    source_kind: "iphone_backup",
    transcript_name: "Large Archive",
    attachments: [],
    issues: [
      {
        code: "message_window_truncated",
        message: `Only the latest ${messageCount.toLocaleString("en-US")} messages were loaded for performance`,
      },
    ],
    messages: Array.from({ length: messageCount }, (_, index) => {
      const messageNumber = index + 1;
      const hour = 7 + Math.floor(index / 60);
      const minute = index % 60;

      return {
        id: `large-${messageNumber.toString().padStart(4, "0")}`,
        timestamp: {
          raw: `06/23/2026, ${hour.toString().padStart(2, "0")}:${minute
            .toString()
            .padStart(2, "0")}`,
        },
        sender: index % 3 === 0 ? "You" : "Demo Contact",
        body: `Large archive synthetic message ${messageNumber.toLocaleString("en-US")}.`,
        attachment_ids: [],
      };
    }),
  };
}
