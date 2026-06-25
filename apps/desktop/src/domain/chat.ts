import type { Attachment, Chat, ChatImport, LoadedChatSource, Message, SourceKind } from "../models";

export interface ChatSummary {
  id: string;
  title: string;
  subtitle: string;
  latestTime: string;
  messageCount: number;
  mediaCount: number;
}

export const MESSAGE_WINDOW_TRUNCATED_CODE = "message_window_truncated";
export const SEARCH_RESULTS_TRUNCATED_CODE = "search_results_truncated";

export function createChatSummary(
  imported: ChatImport,
  source: LoadedChatSource | null,
): ChatSummary {
  const latestMessage = imported.messages.at(-1) ?? null;
  return {
    id: `${imported.source_kind}:${imported.transcript_name ?? "imported-chat"}`,
    title: deriveChatTitle(
      imported.transcript_name,
      source?.displayName ?? null,
      imported.source_kind,
    ),
    subtitle: latestMessage ? summarizeMessage(latestMessage, imported.attachments) : "No messages",
    latestTime: latestMessage ? displayTimestamp(latestMessage.timestamp.raw) : "",
    messageCount: imported.messages.length,
    mediaCount: imported.attachments.length,
  };
}

export function messageWindowNotice(imported: ChatImport): string | null {
  return imported.issues.find((issue) => issue.code === MESSAGE_WINDOW_TRUNCATED_CODE)?.message ?? null;
}

export function searchResultsNotice(imported: ChatImport): string | null {
  return imported.issues.find((issue) => issue.code === SEARCH_RESULTS_TRUNCATED_CODE)?.message ?? null;
}

export function messageCountLabel(imported: ChatImport): string {
  const loadedCount = imported.messages.length.toLocaleString();
  return messageWindowNotice(imported)
    ? `${loadedCount} recent messages loaded`
    : `${loadedCount} messages`;
}

export function messageFilterResultLabel(matchCount: number, imported: ChatImport): string {
  const formattedCount = matchCount.toLocaleString();
  return messageWindowNotice(imported)
    ? `${formattedCount} matches in loaded messages`
    : `${formattedCount} matches`;
}

export function deriveChatTitle(
  transcriptName: string | null,
  sourceDisplayName: string | null,
  sourceKind: SourceKind = "whatsapp_export_zip",
): string {
  const candidate =
    sourceKind === "iphone_backup"
      ? transcriptName ?? sourceDisplayName ?? "Imported chat"
      : sourceDisplayName ?? transcriptName ?? "Imported chat";
  return candidate
    .replace(/\.zip$/i, "")
    .replace(/\.txt$/i, "")
    .replace(/^WhatsApp Chat\s*-\s*/i, "")
    .replace(/^_chat$/i, "Imported chat")
    .trim() || "Imported chat";
}

export function createAvatarInitials(title: string): string {
  const initials = title
    .split(/\s+/)
    .map(readableInitial)
    .filter((initial): initial is string => Boolean(initial))
    .slice(0, 2)
    .join("");

  return initials || "WV";
}

function readableInitial(value: string): string | null {
  for (const character of Array.from(value.normalize("NFKC"))) {
    if (/[\p{L}\p{N}]/u.test(character)) {
      return character.toLocaleUpperCase();
    }
  }

  return null;
}

export function createExportFilename(title: string): string {
  const sanitized = title
    .normalize("NFKD")
    .replace(/[^\w\s.-]/g, "")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/^[.-]+/g, "")
    .replace(/[ .]+$/g, "")
    .replaceAll(" ", "-")
    .slice(0, 80);

  return `${sanitized || "whatsvault-chat"}.html`;
}

export function buildAttachmentMap(attachments: Attachment[]): Map<string, Attachment> {
  return new Map(attachments.map((attachment) => [attachment.id, attachment]));
}

function normalizeSearchText(value: string): string {
  return value
    .normalize("NFKD")
    .toLocaleLowerCase()
    .replace(/\p{Diacritic}/gu, "")
    .replace(/\s+/g, " ")
    .trim();
}

function searchTokens(query: string): string[] {
  return normalizeSearchText(query)
    .split(" ")
    .filter(Boolean);
}

function includesEverySearchToken(searchable: string, tokens: string[]): boolean {
  if (tokens.length === 0) {
    return true;
  }

  const normalizedSearchable = normalizeSearchText(searchable);
  return tokens.every((token) => normalizedSearchable.includes(token));
}

function createSearchMatcher(query: string): ((searchable: string) => boolean) | null {
  const tokens = searchTokens(query);
  if (tokens.length === 0) {
    return null;
  }

  return (searchable) => includesEverySearchToken(searchable, tokens);
}

export function filterChats(chats: Chat[], query: string): Chat[] {
  const matches = createSearchMatcher(query);
  if (!matches) {
    return chats;
  }

  return chats.filter((chat) => matches(
    `${chat.title} ${chat.latestMessage ?? ""}`,
  ));
}

export function filterMessages(messages: Message[], query: string): Message[] {
  const matches = createSearchMatcher(query);
  if (!matches) {
    return messages;
  }

  return messages.filter((message) => {
    return matches(`${message.sender ?? ""} ${message.body}`);
  });
}

function paddedDatePart(value: number): string | null {
  if (!Number.isInteger(value) || value <= 0) {
    return null;
  }

  return value.toString().padStart(2, "0");
}

function dateKeyFromParts(year: number, month: number, day: number): string | null {
  if (year < 1000 || year > 9999 || month < 1 || month > 12 || day < 1 || day > 31) {
    return null;
  }

  const monthPart = paddedDatePart(month);
  const dayPart = paddedDatePart(day);
  if (!monthPart || !dayPart) {
    return null;
  }

  return `${year}-${monthPart}-${dayPart}`;
}

export function messageDateKey(message: Message): string | null {
  const raw = message.timestamp.raw.trim();
  const isoLike = raw.match(/^(\d{4})[-/](\d{1,2})[-/](\d{1,2})\b/);
  if (isoLike) {
    return dateKeyFromParts(
      Number.parseInt(isoLike[1], 10),
      Number.parseInt(isoLike[2], 10),
      Number.parseInt(isoLike[3], 10),
    );
  }

  const slashDate = raw.match(/^(\d{1,2})\/(\d{1,2})\/(\d{2,4})\b/);
  if (!slashDate) {
    return null;
  }

  const first = Number.parseInt(slashDate[1], 10);
  const second = Number.parseInt(slashDate[2], 10);
  const year = Number.parseInt(slashDate[3].length === 2 ? `20${slashDate[3]}` : slashDate[3], 10);
  const month = first > 12 && second <= 12 ? second : first;
  const day = first > 12 && second <= 12 ? first : second;

  return dateKeyFromParts(year, month, day);
}

export function filterMessagesByDate(messages: Message[], selectedDate: string): Message[] {
  const normalizedDate = selectedDate.trim();
  if (!normalizedDate) {
    return messages;
  }

  return messages.filter((message) => messageDateKey(message) === normalizedDate);
}

export function createMessageWindow(messages: Message[], limit: number): Message[] {
  if (limit <= 0 || messages.length <= limit) {
    return messages;
  }

  return messages.slice(messages.length - limit);
}

export function summarizeMessage(message: Message, attachments: Attachment[]): string {
  const firstAttachment = message.attachment_ids
    .map((id) => attachments.find((attachment) => attachment.id === id))
    .find((attachment): attachment is Attachment => Boolean(attachment));

  if (firstAttachment) {
    return `${attachmentLabel(firstAttachment.kind)} ${message.body}`.trim();
  }

  return message.body.replace(/\s+/g, " ").trim() || "Empty message";
}

export function attachmentLabel(kind: Attachment["kind"]): string {
  switch (kind) {
    case "audio":
      return "Voice message";
    case "gif":
      return "GIF";
    case "photo":
      return "Photo";
    case "sticker":
      return "Sticker";
    case "video":
      return "Video";
    default:
      return "Attachment";
  }
}

export function isOutgoingMessage(message: Message): boolean {
  const sender = message.sender?.trim().toLocaleLowerCase();
  return sender === "you" || sender === "me" || sender === "você";
}

export function displayTimestamp(raw: string): string {
  const trimmed = raw.trim();
  const timeMatch = trimmed.match(/(?:^|,\s*)(\d{1,2}:\d{2})(?::\d{2})?\s*(AM|PM)?/i);
  if (!timeMatch) {
    return trimmed;
  }

  return `${timeMatch[1]}${timeMatch[2] ? ` ${timeMatch[2].toUpperCase()}` : ""}`;
}
