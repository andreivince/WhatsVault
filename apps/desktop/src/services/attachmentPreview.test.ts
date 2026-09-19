import { describe, expect, it } from "vitest";

import type { Attachment, AttachmentPreview, LoadedChatSource } from "../models";

import { createAttachmentPreviewLoader } from "./attachmentPreview";

const source: LoadedChatSource = {
  kind: "iphone_backup",
  handle: "backup-source-1",
  displayName: "Example iPhone",
  chatId: "chat-1",
};

function attachment(id: string): Attachment {
  return {
    id,
    archive_path: `Message/Media/${id}.jpg`,
    filename: `${id}.jpg`,
    kind: "photo",
    size_bytes: 100,
  };
}

function preview(id: string): AttachmentPreview {
  return {
    mediaType: "image/jpeg",
    dataUrl: `data:image/jpeg;base64,${id}`,
    sizeBytes: 100,
  };
}

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  const promise = new Promise<T>((nextResolve) => {
    resolve = nextResolve;
  });

  return { promise, resolve };
}

describe("attachmentPreviewLoader", () => {
  it("deduplicates concurrent requests for the same source attachment", async () => {
    let readCount = 0;
    const loader = createAttachmentPreviewLoader({
      readPreview: async (_source, requestedAttachment) => {
        readCount += 1;
        return preview(requestedAttachment.id);
      },
    });
    const requestedAttachment = attachment("photo-1");

    const [first, second] = await Promise.all([
      loader.load(source, requestedAttachment),
      loader.load(source, requestedAttachment),
    ]);

    expect(readCount).toBe(1);
    expect(first).toEqual(preview("photo-1"));
    expect(second).toEqual(preview("photo-1"));
  });

  it("bounds active preview reads", async () => {
    let activeReads = 0;
    let maxActiveReads = 0;
    const loader = createAttachmentPreviewLoader({
      concurrency: 2,
      readPreview: async (_source, requestedAttachment) => {
        activeReads += 1;
        maxActiveReads = Math.max(maxActiveReads, activeReads);
        await new Promise((resolve) => setTimeout(resolve, 0));
        activeReads -= 1;
        return preview(requestedAttachment.id);
      },
    });

    const requests = ["photo-1", "photo-2", "photo-3", "photo-4"].map((id) =>
      loader.load(source, attachment(id)),
    );
    await Promise.all(requests);

    expect(maxActiveReads).toBe(2);
  });

  it("does not abandon queued preview reads when the cache is cleared", async () => {
    const firstRelease = deferred<void>();
    const loader = createAttachmentPreviewLoader({
      concurrency: 1,
      readPreview: async (_source, requestedAttachment) => {
        if (requestedAttachment.id === "photo-1") {
          await firstRelease.promise;
        }
        return preview(requestedAttachment.id);
      },
    });

    const firstRequest = loader.load(source, attachment("photo-1"));
    const queuedRequest = loader.load(source, attachment("photo-2"));
    loader.clear();
    firstRelease.resolve();

    await expect(firstRequest).resolves.toEqual(preview("photo-1"));
    const queuedResult = await Promise.race([
      queuedRequest,
      new Promise<"timed-out">((resolve) => setTimeout(() => resolve("timed-out"), 25)),
    ]);
    expect(queuedResult).toEqual(preview("photo-2"));
  });

  it("preserves the concurrency bound when clearing during an active read", async () => {
    const releases = [deferred<void>(), deferred<void>()];
    let activeReads = 0;
    let maxActiveReads = 0;
    const loader = createAttachmentPreviewLoader({
      concurrency: 1,
      readPreview: async (_source, requestedAttachment) => {
        activeReads += 1;
        maxActiveReads = Math.max(maxActiveReads, activeReads);
        await releases[Number(requestedAttachment.id.at(-1)) - 1].promise;
        activeReads -= 1;
        return preview(requestedAttachment.id);
      },
    });

    const firstRequest = loader.load(source, attachment("photo-1"));
    loader.clear();
    const secondRequest = loader.load(source, attachment("photo-2"));

    expect(maxActiveReads).toBe(1);
    releases[0].resolve();
    await firstRequest;
    releases[1].resolve();
    await secondRequest;
    expect(maxActiveReads).toBe(1);
  });

  it("keeps a replacement cache entry when a pre-clear read fails", async () => {
    const firstRelease = deferred<void>();
    let readCount = 0;
    const loader = createAttachmentPreviewLoader({
      readPreview: async (_source, requestedAttachment) => {
        readCount += 1;
        if (readCount === 1) {
          await firstRelease.promise;
          throw new Error("old read failed");
        }
        return preview(requestedAttachment.id);
      },
    });
    const requestedAttachment = attachment("photo-1");
    const oldRequest = loader.load(source, requestedAttachment);
    const oldFailure = expect(oldRequest).rejects.toThrow("old read failed");
    loader.clear();
    const replacement = loader.load(source, requestedAttachment);
    await replacement;
    firstRelease.resolve();
    await oldFailure;

    expect(loader.load(source, requestedAttachment)).toBe(replacement);
    expect(readCount).toBe(2);
  });


  it("reserves a released read slot for an already queued request", async () => {
    const releases = [deferred<AttachmentPreview>(), deferred<AttachmentPreview>(), deferred<AttachmentPreview>()];
    const started: string[] = [];
    const loader = createAttachmentPreviewLoader({
      concurrency: 1,
      readPreview: (_source, requestedAttachment) => {
        started.push(requestedAttachment.id);
        return releases[Number(requestedAttachment.id.at(-1)) - 1].promise;
      },
    });
    const first = loader.load(source, attachment("photo-1"));
    const second = loader.load(source, attachment("photo-2"));
    loader.clear();
    releases[0].resolve(preview("photo-1"));
    const third = Promise.resolve().then(() => loader.load(source, attachment("photo-3")));
    await first;
    const startedBeforeSecondFinished = [...started];
    releases[1].resolve(preview("photo-2"));
    releases[2].resolve(preview("photo-3"));
    await Promise.all([second, third]);

    expect(startedBeforeSecondFinished).toEqual(["photo-1", "photo-2"]);
    expect(started).toEqual(["photo-1", "photo-2", "photo-3"]);
  });

});
