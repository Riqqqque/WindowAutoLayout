import {
  AlertTriangle,
  Camera,
  CheckCircle2,
  LockKeyhole,
  Monitor,
  Play,
  RefreshCw,
  Trash2,
  UnlockKeyhole,
} from "lucide-react";
import type { AppConfig, MonitorInfo, Profile, RestoreResult, WindowAutoLayoutConfig, WindowInfo } from "../lib/types";
import { monitorLabel, resolveProfileMonitor } from "../lib/helpers";
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
  captureMonitorId: string;
  onProfileChange: (profileId: string) => void;
  onRestore: () => void;
  onLockToggle: () => void;
  onRefresh: () => void;
  onCaptureMonitorChange: (monitorId: string) => void;
  onCaptureCurrentLayout: () => void;
  onRemoveApp: (appId: string) => void;
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
  captureMonitorId,
  onProfileChange,
  onRestore,
  onLockToggle,
  onRefresh,
  onCaptureMonitorChange,
  onCaptureCurrentLayout,
  onRemoveApp,
}: DashboardProps) {
  const resolvedMonitor = resolveProfileMonitor(config, profile, monitors);
  const monitor = resolvedMonitor.monitor;
  const visibleWindows = windows.filter((window) => window.isVisible && !window.isMinimized).length;
  const hiddenWindows = windows.filter((window) => !window.isVisible || window.isMinimized).length;

  return (
    <div className="page-stack">
      <section className="panel-raised p-4">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="min-w-0">
            <div className="eyebrow">Workspace command</div>
            <h1 className="mt-1 truncate text-xl font-semibold text-zinc-50">{profile.name}</h1>
            <p className="mt-1 max-w-2xl text-sm leading-5 text-[#91a0ab]">
              {profile.description?.trim() || `${profile.apps.length} saved apps`}
            </p>
          </div>
          <div className="flex flex-wrap items-center justify-end gap-2">
            <SelectInput value={profile.id} onChange={(event) => onProfileChange(event.target.value)} className="min-w-44">
              {config.profiles.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </SelectInput>
            <button className="button-primary" onClick={onRestore} disabled={busy}>
              <Play size={16} />
              Restore
            </button>
            <button
              className={layoutLocked ? "button-secondary border-[#42d392]/55 text-[#aef2d1]" : "button-secondary"}
              onClick={onLockToggle}
              disabled={busy}
            >
              {layoutLocked ? <LockKeyhole size={16} /> : <UnlockKeyhole size={16} />}
              {layoutLocked ? "Locked" : "Lock"}
            </button>
            <IconButton label="Refresh" onClick={onRefresh} disabled={busy}>
              <RefreshCw size={16} />
            </IconButton>
          </div>
        </div>

        <div className="mt-4 flex flex-wrap items-end gap-2 border-t border-[#2b3740] pt-4">
          <label className="grid min-w-[240px] flex-1 gap-1.5">
            <span className="field-label">Capture monitor</span>
            <SelectInput
              id="capture-monitor"
              aria-label="Capture monitor"
              value={captureMonitorId}
              onChange={(event) => onCaptureMonitorChange(event.target.value)}
              disabled={busy || monitors.length === 0}
            >
              {monitors.map((item) => (
                <option key={item.id} value={item.id}>
                  {monitorLabel(item)}
                </option>
              ))}
            </SelectInput>
          </label>
          <button className="button-secondary" onClick={onCaptureCurrentLayout} disabled={busy || !captureMonitorId}>
            <Camera size={15} />
            Capture current layout
          </button>
        </div>
      </section>

      <section className="metric-strip">
        <Metric label="Displays" value={String(monitors.length)} detail={monitor ? monitorLabel(monitor) : "Target missing"} />
        <Metric label="Windows ready" value={String(visibleWindows)} detail={`${hiddenWindows} hidden or minimized`} />
        <Metric label="Profile apps" value={String(profile.apps.length)} detail={profile.name} />
        <Metric label="Startup" value={config.startup.enabled ? "On" : "Off"} detail={config.startup.enabled ? "Tray resident" : "Not registered"} />
      </section>

      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_330px]">
        <section className="panel min-w-0 p-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <h2 className="section-heading">Profile apps</h2>
              <div className="mt-1 text-xs text-[#7e8d98]">{profile.apps.length} restore targets</div>
            </div>
            {lastRestore && <StatusBadge value={lastRestore.status} />}
          </div>

          <div className="data-list mt-4">
            {profile.apps.map((app) => (
              <AppRow key={app.id} app={app} windows={windows} lastRestore={lastRestore} onRemove={() => onRemoveApp(app.id)} />
            ))}
            {profile.apps.length === 0 && <div className="px-4 py-10 text-center text-sm text-[#7e8d98]">No apps in this profile</div>}
          </div>
        </section>

        <aside className="grid content-start gap-4">
          <section className="panel p-4">
            <h2 className="section-heading">System status</h2>
            <div className="mt-3 grid gap-2">
              {validation.length === 0 ? (
                <div className="flex items-center gap-2 text-sm text-[#aef2d1]">
                  <CheckCircle2 size={16} />
                  Config checks passed
                </div>
              ) : (
                validation.map((issue) => (
                  <div key={issue} className="flex gap-2 text-sm text-amber-200">
                    <AlertTriangle size={16} className="mt-0.5 shrink-0" />
                    <span>{issue}</span>
                  </div>
                ))
              )}
              <div className="mt-2 flex items-center justify-between border-t border-[#27313a] pt-3 text-sm">
                <span className="text-[#91a0ab]">Event lock</span>
                <span className={layoutLocked ? "text-[#aef2d1]" : "text-[#91a0ab]"}>{layoutLocked ? "Active" : "Off"}</span>
              </div>
              <div className="flex items-center justify-between text-sm">
                <span className="text-[#91a0ab]">Input hooks</span>
                <span className="text-[#aef2d1]">None</span>
              </div>
            </div>
          </section>

          <section className="panel p-4">
            <h2 className="section-heading">Displays</h2>
            <div className="mt-3 divide-y divide-[#27313a]">
              {monitors.map((item) => (
                <div key={item.id} className="py-3 first:pt-0 last:pb-0">
                  <div className="flex items-center justify-between gap-3">
                    <span className="flex min-w-0 items-center gap-2 truncate text-sm font-medium text-zinc-200">
                      <Monitor size={15} className="shrink-0 text-[#43c7e7]" />
                      {item.name}
                    </span>
                    {item.isPrimary && <span className="text-[11px] font-semibold text-[#bcecf5]">Primary</span>}
                  </div>
                  <div className="mt-1 pl-[23px] text-xs text-[#71818c]">
                    {item.width} x {item.height} at {item.x}, {item.y}
                  </div>
                </div>
              ))}
            </div>
          </section>
        </aside>
      </div>
    </div>
  );
}

function AppRow({
  app,
  windows,
  lastRestore,
  onRemove,
}: {
  app: AppConfig;
  windows: WindowInfo[];
  lastRestore?: RestoreResult | null;
  onRemove: () => void;
}) {
  const processName = app.processName?.toLowerCase();
  const matches = processName ? windows.filter((window) => window.processName.toLowerCase() === processName).length : 0;
  const restoreStatus = lastRestore?.results.find((result) => result.appId === app.id)?.status;

  return (
    <div className="data-row grid min-h-[66px] items-center gap-3 px-3 py-2.5 md:grid-cols-[minmax(0,1fr)_120px_132px_38px]">
      <div className="min-w-0">
        <div className="truncate text-sm font-semibold text-zinc-100">{app.displayName}</div>
        <div className="mt-1 truncate text-xs text-[#758590]">{app.processName ?? "Process unset"}</div>
      </div>
      <div className="text-xs text-[#7f8f99]">
        <span className="block font-medium text-zinc-300">{app.layout.width} x {app.layout.height}</span>
        <span>{app.layout.x}, {app.layout.y}</span>
      </div>
      <div className="flex items-center gap-2 md:justify-end">
        <StatusBadge value={restoreStatus ?? (matches > 0 ? "running" : "notRunning")} />
        <span className="text-xs text-[#758590]">{matches} live</span>
      </div>
      <IconButton label={`Remove ${app.displayName} from profile`} onClick={onRemove} variant="danger">
        <Trash2 size={15} />
      </IconButton>
    </div>
  );
}

function Metric({ label, value, detail }: { label: string; value: string; detail: string }) {
  return (
    <div className="metric-cell">
      <div className="eyebrow">{label}</div>
      <div className="mt-1.5 text-xl font-semibold text-zinc-50">{value}</div>
      <div className="mt-1 truncate text-xs text-[#758590]">{detail}</div>
    </div>
  );
}
