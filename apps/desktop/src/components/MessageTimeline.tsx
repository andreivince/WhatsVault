import { useVirtualizer } from "@tanstack/react-virtual";
import { type RefObject, useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";

import type { Attachment, LoadedChatSource, Message } from "../models";
import { TEST_IDS } from "../testing/testIds";
import { MessageBubble } from "./MessageBubble";

export function MessageTimeline({
  messages,
  source,
  attachmentMap,
  scrollParentRef,
  onOpenImagePreview,
  timelineIdentity,
}: {
  messages: Message[];
  source: LoadedChatSource | null;
  attachmentMap: Map<string, Attachment>;
  scrollParentRef: RefObject<HTMLDivElement | null>;
  onOpenImagePreview: (preview: { dataUrl: string; alt: string; caption: string }) => void;
  timelineIdentity: string;
}) {
  const listRef = useRef<HTMLDivElement>(null);
  const [scrollMargin, setScrollMargin] = useState(0);
  const virtualizer = useVirtualizer({
    count: messages.length,
    getScrollElement: () => scrollParentRef.current,
    getItemKey: useCallback((index: number) => messages[index].id, [messages]),
    estimateSize: () => 100,
    overscan: 12,
    scrollMargin,
    anchorTo: "end",
  });

  const measureLayout = useCallback(() => {
    const parent = scrollParentRef.current;
    const list = listRef.current;
    if (parent && list) {
      setScrollMargin(list.getBoundingClientRect().top - parent.getBoundingClientRect().top + parent.scrollTop);
    }
  }, [scrollParentRef]);

  // Notices and the earlier-messages control can change the list's offset.
  useLayoutEffect(measureLayout);
  useEffect(() => {
    const parent = scrollParentRef.current;
    if (!parent) return;
    let width = parent.clientWidth;
    const observer = new ResizeObserver(() => {
      measureLayout();
      if (parent.clientWidth !== width) {
        width = parent.clientWidth;
        virtualizer.measure();
      }
    });
    observer.observe(parent);
    return () => observer.disconnect();
  }, [measureLayout, scrollParentRef, virtualizer]);

  const positionedTimeline = useRef<string | null>(null);
  useLayoutEffect(() => {
    // The parent ref can attach after this child's first layout effect.
    if (virtualizer.scrollElement && positionedTimeline.current !== timelineIdentity) {
      virtualizer.scrollToEnd();
      positionedTimeline.current = timelineIdentity;
    }
  });

  const rows = virtualizer.getVirtualItems();
  return (
    <div
      className="virtual-message-list"
      ref={listRef}
      style={{ height: virtualizer.getTotalSize() }}
      data-testid={TEST_IDS.virtualMessageList}
      data-total-messages={messages.length}
      data-rendered-messages={rows.length}
    >
      {rows.map(row => (
        <div
          className="virtual-message-item"
          key={row.key}
          data-index={row.index}
          ref={virtualizer.measureElement}
          style={{ transform: `translateY(${row.start - scrollMargin}px)` }}
        >
          <MessageBubble
            message={messages[row.index]}
            source={source}
            onOpenImagePreview={onOpenImagePreview}
            attachmentMap={attachmentMap}
          />
        </div>
      ))}
    </div>
  );
}
