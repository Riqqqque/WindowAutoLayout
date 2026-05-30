import { RefreshCw, Save } from "lucide-react";
import type { WindowInfo } from "../lib/types";
import { IconButton } from "./IconButton";
import { StatusBadge } from "./StatusBadge";

interface WindowPickerProps {
  windows: WindowInfo[];
  selectedHandle?: string | null;
  onRefresh: () => void;
  onSelect: (handle: string) => void;
  onSave: () => void;
}

export function WindowPicker({ windows, selectedHandle, onRefresh, onSelect, onSave }: WindowPickerProps) {
  return (
    <div className="overflow-hidden rounded-lg border border-zinc-800">
      <div className="flex items-center justify-between border-b border-zinc-800 bg-zinc-950 px-3 py-2">
        <span className="text-sm font-medium text-zinc-200">Detected windows</span>
        <div className="flex gap-2">
          <IconButton label="Refresh windows" onClick={onRefresh}>
            <RefreshCw size={16} />
          </IconButton>
          <IconButton label="Save selected window layout" onClick={onSave} disabled={!selectedHandle} variant="solid">
            <Save size={16} />
          </IconButton>
        </div>
      </div>
      <div className="max-h-72 overflow-auto">
        {windows.map((window) => (
          <button
            key={window.handle}
            className={`grid w-full grid-cols-[1fr_120px_120px] gap-3 border-b border-zinc-900 px-3 py-2 text-left text-sm transition last:border-b-0 ${
              selectedHandle === window.handle
                ? "bg-cyan-300/10 text-cyan-100"
                : "bg-zinc-950 text-zinc-300 hover:bg-zinc-900"
            }`}
            onClick={() => onSelect(window.handle)}
          >
            <span className="min-w-0">
              <span className="block truncate font-medium">{window.title || "(Untitled)"}</span>
              <span className="mt-1 flex flex-wrap items-center gap-1 text-xs text-zinc-500">
                <span className="truncate">
                  {window.processName} · {window.className}
                </span>
                {!window.isVisible && <StatusBadge value="hidden" />}
                {window.isMinimized && <StatusBadge value="minimized" />}
              </span>
            </span>
            <span className="text-xs text-zinc-400">
              {window.x}, {window.y}
              <br />
              {window.width} x {window.height}
            </span>
            <span className="truncate text-xs text-zinc-500" title={window.executablePath ?? ""}>
              PID {window.processId}
              <br />
              {window.executablePath ?? "Path unavailable"}
            </span>
          </button>
        ))}
        {windows.length === 0 && <div className="px-3 py-8 text-center text-sm text-zinc-500">No windows detected</div>}
      </div>
    </div>
  );
}
