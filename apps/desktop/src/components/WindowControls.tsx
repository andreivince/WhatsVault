import { Maximize2, Minus, X } from "lucide-react";

import {
  closeAppWindow,
  isDesktopRuntime,
  minimizeAppWindow,
  toggleMaximizeAppWindow,
} from "../services/desktop";

export function WindowControls() {
  if (!isDesktopRuntime()) {
    return null;
  }

  return (
    <nav className="window-controls" aria-label="Window controls">
      <button
        className="icon-button window-control-button"
        type="button"
        onClick={minimizeAppWindow}
        aria-label="Minimize window"
        title="Minimize window"
      >
        <Minus />
      </button>
      <button
        className="icon-button window-control-button"
        type="button"
        onClick={toggleMaximizeAppWindow}
        aria-label="Zoom window"
        title="Zoom window"
      >
        <Maximize2 />
      </button>
      <button
        className="icon-button window-control-button close"
        type="button"
        onClick={closeAppWindow}
        aria-label="Close window"
        title="Close window"
      >
        <X />
      </button>
    </nav>
  );
}
