import { clsx } from "clsx";
import { statusText } from "../lib/helpers";

const toneByStatus: Record<string, string> = {
  success: "border-[#42d392]/35 bg-[#42d392]/10 text-[#aef2d1]",
  launched: "border-[#43c7e7]/35 bg-[#43c7e7]/10 text-[#bcecf5]",
  running: "border-[#42d392]/35 bg-[#42d392]/10 text-[#aef2d1]",
  notRunning: "border-[#485363] bg-[#485363]/12 text-zinc-300",
  partialSuccess: "border-amber-400/30 bg-amber-400/10 text-amber-200",
  paused: "border-amber-400/30 bg-amber-400/10 text-amber-200",
  warn: "border-amber-400/30 bg-amber-400/10 text-amber-200",
  failed: "border-rose-400/30 bg-rose-400/10 text-rose-200",
  error: "border-rose-400/30 bg-rose-400/10 text-rose-200",
  monitorMissing: "border-amber-400/30 bg-amber-400/10 text-amber-200",
  skipped: "border-[#485363] bg-[#485363]/12 text-zinc-300",
  hidden: "border-[#5c91ff]/30 bg-[#5c91ff]/10 text-[#bfd0ff]",
  minimized: "border-sky-400/30 bg-sky-400/10 text-sky-200",
  maximized: "border-[#43c7e7]/30 bg-[#43c7e7]/10 text-[#bcecf5]",
};

export function StatusBadge({ value, className }: { value: string; className?: string }) {
  return (
    <span
      className={clsx(
        "inline-flex h-6 items-center whitespace-nowrap rounded-md border px-2 text-xs font-semibold",
        toneByStatus[value] ?? "border-[#485363] bg-[#171d24] text-zinc-300",
        className,
      )}
    >
      {statusText(value)}
    </span>
  );
}
