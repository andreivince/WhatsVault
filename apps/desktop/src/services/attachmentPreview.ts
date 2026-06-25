import type {
  Attachment,
  AttachmentPreview,
  LoadedChatSource,
} from "../models";

import { readLocalAttachmentPreview } from "./desktop";

const DEFAULT_ATTACHMENT_PREVIEW_CONCURRENCY = 4;
const DEFAULT_ATTACHMENT_PREVIEW_CACHE_ENTRIES = 256;

type PreviewReader = (
  source: LoadedChatSource,
  attachment: Attachment,
) => Promise<AttachmentPreview | null>;

export interface AttachmentPreviewLoaderOptions {
  concurrency?: number;
  maxCacheEntries?: number;
  readPreview?: PreviewReader;
}

export interface AttachmentPreviewLoader {
  load(source: LoadedChatSource, attachment: Attachment): Promise<AttachmentPreview | null>;
  clear(): void;
}

export function createAttachmentPreviewLoader(
  options: AttachmentPreviewLoaderOptions = {},
): AttachmentPreviewLoader {
  const concurrency = Math.max(
    1,
    options.concurrency ?? DEFAULT_ATTACHMENT_PREVIEW_CONCURRENCY,
  );
  const maxCacheEntries = Math.max(
    1,
    options.maxCacheEntries ?? DEFAULT_ATTACHMENT_PREVIEW_CACHE_ENTRIES,
  );
  const readPreview = options.readPreview ?? readLocalAttachmentPreview;
  const cache = new Map<string, Promise<AttachmentPreview | null>>();
  const pendingReads: Array<() => void> = [];
  let activeReads = 0;

  async function withReadSlot<T>(task: () => Promise<T>): Promise<T> {
    if (activeReads >= concurrency) {
      await new Promise<void>((resolve) => pendingReads.push(resolve));
    }

    activeReads += 1;
    try {
      return await task();
    } finally {
      activeReads -= 1;
      pendingReads.shift()?.();
    }
  }

  function remember(
    cacheKey: string,
    request: Promise<AttachmentPreview | null>,
  ): Promise<AttachmentPreview | null> {
    if (!cache.has(cacheKey) && cache.size >= maxCacheEntries) {
      const oldestKey = cache.keys().next().value;
      if (oldestKey) {
        cache.delete(oldestKey);
      }
    }

    cache.set(cacheKey, request);
    return request;
  }

  return {
    load(source, attachment) {
      const cacheKey = attachmentPreviewCacheKey(source, attachment);
      const cached = cache.get(cacheKey);
      if (cached) {
        return cached;
      }

      const request = withReadSlot(() => readPreview(source, attachment)).catch((error) => {
        cache.delete(cacheKey);
        throw error;
      });

      return remember(cacheKey, request);
    },
    clear() {
      cache.clear();
      pendingReads.splice(0, pendingReads.length);
      activeReads = 0;
    },
  };
}

export const attachmentPreviewLoader = createAttachmentPreviewLoader();

function attachmentPreviewCacheKey(source: LoadedChatSource, attachment: Attachment): string {
  return [
    source.kind,
    source.handle,
    source.chatId ?? "",
    attachment.id,
    attachment.archive_path,
    attachment.filename,
    attachment.kind,
  ].join("\u001f");
}
