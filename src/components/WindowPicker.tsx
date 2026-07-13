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
    <section className="panel overflow-hidden">
      <div className="flex items-center justify-between border-b border-[#27313a] px-3 py-2.5">
        <div>
          <span className="text-sm font-semibold text-zinc-200">Detected windows</span>
          <span className="ml-2 text-xs text-[#71818c]">{windows.length}</span>
        </div>
        <div className="flex gap-2">
          <IconButton label="Refresh windows" onClick={onRefresh}>
            <RefreshCw size={16} />
          </IconButton>
          <IconButton label="Save selected window layout" onClick={onSave} disabled={!selectedHandle} variant="solid">
            <Save size={16} />
          </IconButton>
        </div>
      </div>
      <div className="max-h-[360px] overflow-auto bg-[#0c1217]">
        {windows.map((window) => (
          <button
            key={window.handle}
            className={`grid w-full grid-cols-[minmax(0,1fr)_auto] gap-3 border-b border-[#202a31] px-3 py-2.5 text-left text-sm transition last:border-b-0 ${
              selectedHandle === window.handle
                ? "bg-[#17303a] text-[#e1f8fb]"
                : "text-zinc-300 hover:bg-[#121a20]"
            }`}
            onClick={() => onSelect(window.handle)}
          >
            <span className="min-w-0">
              <span className="block truncate font-medium">{window.title || "(Untitled)"}</span>
              <span className="mt-1 flex min-w-0 flex-wrap items-center gap-1 text-xs text-[#758590]">
                <span className="truncate">{window.processName}</span>
                {!window.isVisible && <StatusBadge value="hidden" />}
                {window.isMinimized && <StatusBadge value="minimized" />}
                {window.isMaximized && <StatusBadge value="maximized" />}
              </span>
            </span>
            <span className="text-right text-xs text-zinc-400">
              {window.x}, {window.y}
              <br />
              {window.width} x {window.height}
            </span>
          </button>
        ))}
        {windows.length === 0 && <div className="px-3 py-8 text-center text-sm text-[#71818c]">No windows detected</div>}
      </div>
    </section>
  );
}
