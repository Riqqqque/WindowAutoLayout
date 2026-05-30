import { LockKeyhole, Play, RefreshCw, Save, UnlockKeyhole } from "lucide-react";
import type { MonitorInfo, Profile, RestoreResult, WindowAutoLayoutConfig, WindowInfo } from "../lib/types";
import { monitorLabel } from "../lib/helpers";
import { StatusBadge } from "../components/StatusBadge";
import { IconButton } from "../components/IconButton";
import { SelectInput } from "../components/Form";

interface DashboardProps {
  config: WindowAutoLayoutConfig;
  profile: Profile;
  monitors: MonitorInfo[];
  windows: WindowInfo[];
  lastRestore?: RestoreResult | null;
  validation: string[];
  busy: boolean;
  layoutLocked: boolean;
  onProfileChange: (profileId: string) => void;
  onRestore: () => void;
  onLockToggle: () => void;
  onRefresh: () => void;
  onSaveAll: () => void;
}

export function Dashboard({
  config,
  profile,
  monitors,
  windows,
  lastRestore,
  validation,
  busy,
  layoutLocked,
  onProfileChange,
  onRestore,
  onLockToggle,
  onRefresh,
  onSaveAll,
}: DashboardProps) {
  const configuredApps = profile.apps.length;
  const monitor = monitors.find((item) => item.id === (profile.targetMonitorId ?? config.global.defaultMonitorId));

  return (
    <div className="grid gap-4 xl:grid-cols-[1.25fr_0.75fr]">
      <section className="rounded-lg border border-zinc-800 bg-zinc-900/70 p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h1 className="text-2xl font-semibold text-zinc-50">WindowAutoLayout</h1>
            <p className="mt-1 text-sm text-zinc-400">Profile: {profile.name}</p>
          </div>
          <div className="flex items-center gap-2">
            <SelectInput value={profile.id} onChange={(event) => onProfileChange(event.target.value)}>
              {config.profiles.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </SelectInput>
            <IconButton label="Restore layout" onClick={onRestore} disabled={busy} variant="solid">
              <Play size={16} />
            </IconButton>
            <IconButton
              label={layoutLocked ? "Unlock layout" : "Lock layout"}
              onClick={onLockToggle}
              disabled={busy}
              variant={layoutLocked ? "solid" : "ghost"}
            >
              {layoutLocked ? <LockKeyhole size={16} /> : <UnlockKeyhole size={16} />}
            </IconButton>
            <IconButton label="Refresh" onClick={onRefresh}>
              <RefreshCw size={16} />
            </IconButton>
          </div>
        </div>

        <div className="mt-5 grid gap-3 sm:grid-cols-3">
          <Metric label="Monitors" value={String(monitors.length)} detail={monitor ? monitorLabel(monitor) : "No target set"} />
          <Metric label="Windows" value={String(windows.length)} detail="Visible candidates" />
          <Metric label="Apps" value={String(configuredApps)} detail="In selected profile" />
        </div>

        <div className="mt-5 flex flex-wrap gap-2">
          <button
            className="inline-flex h-10 items-center gap-2 rounded-md border border-zinc-700 bg-zinc-950 px-3 text-sm text-zinc-200 hover:bg-zinc-800 disabled:opacity-40"
            onClick={onSaveAll}
            disabled={busy}
          >
            <Save size={16} />
            Save current layout
          </button>
        </div>

        {lastRestore && (
          <div className="mt-5 rounded-lg border border-zinc-800 bg-zinc-950 p-3">
            <div className="flex items-center justify-between gap-3">
              <span className="text-sm font-medium text-zinc-200">Last restore</span>
              <StatusBadge value={lastRestore.status} />
            </div>
            <div className="mt-3 grid gap-2">
              {lastRestore.results.map((result) => (
                <div key={result.appId} className="flex items-center justify-between gap-3 text-sm">
                  <span className="truncate text-zinc-300">{result.displayName}</span>
                  <StatusBadge value={result.status} />
                </div>
              ))}
            </div>
          </div>
        )}
      </section>

      <section className="rounded-lg border border-zinc-800 bg-zinc-900/70 p-4">
        <h2 className="text-sm font-semibold uppercase tracking-wide text-zinc-500">Health</h2>
        <div className="mt-3 grid gap-2">
          {validation.length === 0 && <div className="rounded-md border border-emerald-400/20 bg-emerald-400/10 px-3 py-2 text-sm text-emerald-100">Config checks passed</div>}
          {validation.map((issue) => (
            <div key={issue} className="rounded-md border border-amber-400/20 bg-amber-400/10 px-3 py-2 text-sm text-amber-100">
              {issue}
            </div>
          ))}
        </div>

        <h2 className="mt-5 text-sm font-semibold uppercase tracking-wide text-zinc-500">Monitors</h2>
        <div className="mt-3 grid gap-2">
          {monitors.map((item) => (
            <div key={item.id} className="rounded-md border border-zinc-800 bg-zinc-950 px-3 py-2">
              <div className="flex items-center justify-between gap-3">
                <span className="truncate text-sm font-medium text-zinc-200">{item.name}</span>
                {item.isPrimary && <StatusBadge value="success" className="text-[11px]" />}
              </div>
              <div className="mt-1 text-xs text-zinc-500">
                {item.x}, {item.y} · {item.width} x {item.height} · scale {item.scaleFactor.toFixed(2)}
              </div>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}

function Metric({ label, value, detail }: { label: string; value: string; detail: string }) {
  return (
    <div className="rounded-lg border border-zinc-800 bg-zinc-950 p-3">
      <div className="text-xs font-medium uppercase tracking-wide text-zinc-500">{label}</div>
      <div className="mt-2 text-2xl font-semibold text-zinc-50">{value}</div>
      <div className="mt-1 truncate text-xs text-zinc-500">{detail}</div>
    </div>
  );
}
