import { clsx } from "clsx";
import { statusText } from "../lib/helpers";

const toneByStatus: Record<string, string> = {
  success: "border-[#39d98a]/35 bg-[#39d98a]/10 text-[#a8f3cf]",
  launched: "border-[#5db7ff]/35 bg-[#5db7ff]/10 text-[#b8ddff]",
  partialSuccess: "border-amber-400/30 bg-amber-400/10 text-amber-200",
  warn: "border-amber-400/30 bg-amber-400/10 text-amber-200",
  failed: "border-rose-400/30 bg-rose-400/10 text-rose-200",
  error: "border-rose-400/30 bg-rose-400/10 text-rose-200",
  monitorMissing: "border-amber-400/30 bg-amber-400/10 text-amber-200",
  skipped: "border-[#485363] bg-[#485363]/12 text-zinc-300",
  hidden: "border-violet-400/30 bg-violet-400/10 text-violet-200",
  minimized: "border-sky-400/30 bg-sky-400/10 text-sky-200",
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
