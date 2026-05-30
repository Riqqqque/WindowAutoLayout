import { clsx } from "clsx";
import { statusText } from "../lib/helpers";

const toneByStatus: Record<string, string> = {
  success: "border-emerald-400/30 bg-emerald-400/10 text-emerald-200",
  launched: "border-cyan-400/30 bg-cyan-400/10 text-cyan-200",
  partialSuccess: "border-amber-400/30 bg-amber-400/10 text-amber-200",
  warn: "border-amber-400/30 bg-amber-400/10 text-amber-200",
  failed: "border-rose-400/30 bg-rose-400/10 text-rose-200",
  error: "border-rose-400/30 bg-rose-400/10 text-rose-200",
  monitorMissing: "border-amber-400/30 bg-amber-400/10 text-amber-200",
  skipped: "border-zinc-500/40 bg-zinc-500/10 text-zinc-300",
  hidden: "border-violet-400/30 bg-violet-400/10 text-violet-200",
  minimized: "border-sky-400/30 bg-sky-400/10 text-sky-200",
};

export function StatusBadge({ value, className }: { value: string; className?: string }) {
  return (
    <span
      className={clsx(
        "inline-flex h-6 items-center whitespace-nowrap rounded-md border px-2 text-xs font-medium",
        toneByStatus[value] ?? "border-zinc-600 bg-zinc-800 text-zinc-300",
        className,
      )}
    >
      {statusText(value)}
    </span>
  );
}
