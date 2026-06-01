import { Copy, ExternalLink, RefreshCw, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import { IconButton } from "../components/IconButton";
import { StatusBadge } from "../components/StatusBadge";
import { TextInput } from "../components/Form";
import type { LogEntry } from "../lib/types";

interface LogsProps {
  logs: LogEntry[];
  onRefresh: () => void;
  onClear: () => void;
  onOpen: () => void;
}

export function LogsPage({ logs, onRefresh, onClear, onOpen }: LogsProps) {
  const [query, setQuery] = useState("");
  const filtered = useMemo(() => {
    const value = query.trim().toLowerCase();
    if (!value) return logs;
    return logs.filter((entry) =>
      [entry.severity, entry.profile, entry.app, entry.message, entry.timestamp]
        .filter(Boolean)
        .some((part) => String(part).toLowerCase().includes(value)),
    );
  }, [logs, query]);

  function copyLogs() {
    const text = filtered
      .map((entry) => `${entry.timestamp} ${entry.severity.toUpperCase()} ${entry.profile ?? ""} ${entry.app ?? ""} ${entry.message}`)
      .join("\n");
    navigator.clipboard?.writeText(text);
  }

  return (
    <section className="surface rounded-md p-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-lg font-semibold text-zinc-50">Logs</h1>
        <div className="flex gap-2">
          <IconButton label="Refresh logs" onClick={onRefresh}>
            <RefreshCw size={16} />
          </IconButton>
          <IconButton label="Copy logs" onClick={copyLogs}>
            <Copy size={16} />
          </IconButton>
          <IconButton label="Open log file" onClick={onOpen}>
            <ExternalLink size={16} />
          </IconButton>
          <IconButton label="Clear logs" onClick={onClear} variant="danger">
            <Trash2 size={16} />
          </IconButton>
        </div>
      </div>

      <div className="mt-4">
        <TextInput placeholder="Search logs" value={query} onChange={(event) => setQuery(event.target.value)} />
      </div>

      <div className="mt-4 overflow-hidden rounded-md border border-[#252b34]">
        <div className="max-h-[520px] overflow-auto">
          {filtered.map((entry, index) => (
            <div key={`${entry.timestamp}-${index}`} className="grid gap-2 border-b border-[#151a21] bg-[#0d1117] px-3 py-2 text-sm last:border-b-0 md:grid-cols-[170px_80px_1fr]">
              <span className="text-xs text-[#8a94a3]">{new Date(entry.timestamp).toLocaleString()}</span>
              <StatusBadge value={entry.severity} />
              <span className="text-zinc-300">
                {entry.profile && <span className="text-[#8a94a3]">{entry.profile} / </span>}
                {entry.app && <span className="text-[#8a94a3]">{entry.app} / </span>}
                {entry.message}
              </span>
            </div>
          ))}
          {filtered.length === 0 && <div className="px-3 py-8 text-center text-sm text-[#8a94a3]">No log entries</div>}
        </div>
      </div>
    </section>
  );
}
