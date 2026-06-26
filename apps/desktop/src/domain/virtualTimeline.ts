export const TIMELINE_ITEM_ESTIMATED_HEIGHT = 72;
export const TIMELINE_OVERSCAN_ITEMS = 18;
export const TIMELINE_MIN_RENDERED_ITEMS = 36;

export interface VirtualTimelineInput {
  itemCount: number;
  scrollTop: number;
  viewportHeight: number;
  listTop?: number;
  estimatedItemHeight?: number;
  overscanItems?: number;
  minimumRenderedItems?: number;
}

export interface VirtualTimelineWindow {
  startIndex: number;
  endIndex: number;
  beforeHeight: number;
  afterHeight: number;
  totalHeight: number;
  renderedCount: number;
}

export function createTrailingVirtualTimelineWindow(
  itemCount: number,
  options: Pick<
    VirtualTimelineInput,
    "estimatedItemHeight" | "overscanItems" | "minimumRenderedItems"
  > = {},
): VirtualTimelineWindow {
  const itemHeight = positiveOrDefault(
    options.estimatedItemHeight,
    TIMELINE_ITEM_ESTIMATED_HEIGHT,
  );
  const minimumRenderedItems = positiveIntegerOrDefault(
    options.minimumRenderedItems,
    TIMELINE_MIN_RENDERED_ITEMS,
  );

  return createVirtualTimelineWindow({
    itemCount,
    scrollTop: itemCount * itemHeight,
    viewportHeight: minimumRenderedItems * itemHeight,
    estimatedItemHeight: itemHeight,
    overscanItems: options.overscanItems,
    minimumRenderedItems,
  });
}

export function createVirtualTimelineWindow({
  itemCount,
  scrollTop,
  viewportHeight,
  listTop = 0,
  estimatedItemHeight = TIMELINE_ITEM_ESTIMATED_HEIGHT,
  overscanItems = TIMELINE_OVERSCAN_ITEMS,
  minimumRenderedItems = TIMELINE_MIN_RENDERED_ITEMS,
}: VirtualTimelineInput): VirtualTimelineWindow {
  const normalizedItemCount = Math.max(0, Math.floor(itemCount));
  const itemHeight = positiveOrDefault(estimatedItemHeight, TIMELINE_ITEM_ESTIMATED_HEIGHT);
  const overscan = positiveIntegerOrDefault(overscanItems, TIMELINE_OVERSCAN_ITEMS);
  const minimumRendered = Math.min(
    normalizedItemCount,
    positiveIntegerOrDefault(minimumRenderedItems, TIMELINE_MIN_RENDERED_ITEMS),
  );
  const totalHeight = normalizedItemCount * itemHeight;

  if (normalizedItemCount === 0) {
    return {
      startIndex: 0,
      endIndex: 0,
      beforeHeight: 0,
      afterHeight: 0,
      totalHeight: 0,
      renderedCount: 0,
    };
  }

  if (normalizedItemCount <= minimumRendered) {
    return {
      startIndex: 0,
      endIndex: normalizedItemCount,
      beforeHeight: 0,
      afterHeight: 0,
      totalHeight,
      renderedCount: normalizedItemCount,
    };
  }

  const effectiveViewportHeight = Math.max(
    0,
    Number.isFinite(viewportHeight) ? viewportHeight : 0,
  );
  const localViewportStart = Math.max(0, scrollTop - listTop);
  const localViewportEnd = Math.max(
    localViewportStart,
    scrollTop + effectiveViewportHeight - listTop,
  );
  const rawStartIndex = Math.floor(localViewportStart / itemHeight) - overscan;
  const rawEndIndex = Math.ceil(localViewportEnd / itemHeight) + overscan;
  const rawRenderedCount = rawEndIndex - rawStartIndex;
  const extraItemsNeeded = Math.max(0, minimumRendered - rawRenderedCount);
  const startIndex = clampIndex(
    rawStartIndex - Math.floor(extraItemsNeeded / 2),
    normalizedItemCount,
  );
  const endIndex = clampIndex(
    Math.max(rawEndIndex + Math.ceil(extraItemsNeeded / 2), startIndex + minimumRendered),
    normalizedItemCount,
  );
  const adjustedStartIndex = Math.max(0, Math.min(startIndex, endIndex - minimumRendered));

  return {
    startIndex: adjustedStartIndex,
    endIndex,
    beforeHeight: adjustedStartIndex * itemHeight,
    afterHeight: Math.max(0, normalizedItemCount - endIndex) * itemHeight,
    totalHeight,
    renderedCount: endIndex - adjustedStartIndex,
  };
}

function clampIndex(value: number, itemCount: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }

  return Math.max(0, Math.min(itemCount, Math.floor(value)));
}

function positiveOrDefault(value: number | undefined, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) && value > 0
    ? value
    : fallback;
}

function positiveIntegerOrDefault(value: number | undefined, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) && value > 0
    ? Math.floor(value)
    : fallback;
}
