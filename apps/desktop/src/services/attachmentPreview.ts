import type {
  Attachment,
  AttachmentPreview,
  LoadedChatSource,
} from "../models";

import { readLocalAttachmentPreview } from "./desktop";
import { loadedChatSourceIdentity } from "../domain/source";

const DEFAULT_ATTACHMENT_PREVIEW_CONCURRENCY = 4;
const DEFAULT_ATTACHMENT_PREVIEW_CACHE_ENTRIES = 256;
const DEFAULT_ATTACHMENT_PREVIEW_CACHE_BYTES = 64 * 1024 * 1024;

interface CacheEntry {
  request: Promise<AttachmentPreview | null>;
  bytes: number;
}

type PreviewReader = (
  source: LoadedChatSource,
  attachment: Attachment,
) => Promise<AttachmentPreview | null>;

export interface AttachmentPreviewLoaderOptions {
  concurrency?: number;
  maxCacheEntries?: number;
  maxCacheBytes?: number;
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
  const maxCacheBytes = Math.max(0, options.maxCacheBytes ?? DEFAULT_ATTACHMENT_PREVIEW_CACHE_BYTES);
  const cache = new Map<string, CacheEntry>();
  let cachedBytes = 0;
  const pendingReads: Array<() => void> = [];
  let activeReads = 0;

  async function withReadSlot<T>(task: () => Promise<T>): Promise<T> {
    if (activeReads >= concurrency) {
      await new Promise<void>((resolve) => pendingReads.push(resolve));
    } else {
      activeReads += 1;
    }

    try {
      return await task();
    } finally {
      const nextRead = pendingReads.shift();
      if (nextRead) {
        // Transfer this slot directly so a new request cannot overtake the queue.
        nextRead();
      } else {
        activeReads -= 1;
      }
    }
  }

  function forget(cacheKey: string) {
    cachedBytes -= cache.get(cacheKey)?.bytes ?? 0;
    cache.delete(cacheKey);
  }

  function trim() {
    while (cache.size > maxCacheEntries || cachedBytes > maxCacheBytes) {
      const oldestKey = cache.keys().next().value;
      if (oldestKey === undefined) break;
      forget(oldestKey);
    }
  }

  return {
    load(source, attachment) {
      const cacheKey = attachmentPreviewCacheKey(source, attachment);
      const cached = cache.get(cacheKey);
      if (cached) {
        cache.delete(cacheKey);
        cache.set(cacheKey, cached);
        return cached.request;
      }

      const request = withReadSlot(() => readPreview(source, attachment))
        .then((preview) => {
          const entry = cache.get(cacheKey);
          if (entry?.request === request) {
            // Budget the retained URL, not the smaller decoded file size. Two bytes per
            // UTF-16 code unit is conservative even when the engine stores ASCII compactly.
            const bytes = (preview?.dataUrl.length ?? 0) * 2;
            if (bytes > maxCacheBytes) {
              forget(cacheKey);
            } else {
              entry.bytes = bytes;
              cachedBytes += bytes;
              trim();
            }
          }
          return preview;
        })
        .catch((error) => {
          if (cache.get(cacheKey)?.request === request) forget(cacheKey);
          throw error;
        });

      cache.set(cacheKey, { request, bytes: 0 });
      trim();
      return request;
    },
    clear() {
      cache.clear();
      cachedBytes = 0;
    },
  };
}

export const attachmentPreviewLoader = createAttachmentPreviewLoader();

function attachmentPreviewCacheKey(source: LoadedChatSource, attachment: Attachment): string {
  return JSON.stringify([
    loadedChatSourceIdentity(source),
    attachment.id,
    attachment.archive_path,
    attachment.filename,
    attachment.kind,
  ]);
}
