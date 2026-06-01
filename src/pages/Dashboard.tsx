import { AlertTriangle, Camera, CheckCircle2, LockKeyhole, Monitor, Play, RefreshCw, Trash2, UnlockKeyhole } from "lucide-react";
import type { AppConfig, MonitorInfo, Profile, RestoreResult, WindowAutoLayoutConfig, WindowInfo } from "../lib/types";
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
  const monitor = monitors.find((item) => item.id === (profile.targetMonitorId ?? config.global.defaultMonitorId));
  const visibleWindows = windows.filter((window) => window.isVisible && !window.isMinimized).length;
  const hiddenWindows = windows.filter((window) => !window.isVisible || window.isMinimized).length;
  const startupDetail = config.startup.enabled
    ? config.startup.restoreOnLaunch
      ? "Restore on launch"
      : "Starts without restore"
    : "Startup disabled";

  return (
    <div className="grid gap-4">
      <section className="surface rounded-md p-4">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="text-2xl font-semibold text-zinc-50">Restore workspace</h1>
              <span
                className={`mt-0.5 inline-flex h-6 items-center rounded-md border px-2 text-xs font-semibold ${
                  layoutLocked
                    ? "border-[#39d98a]/35 bg-[#39d98a]/10 text-[#a8f3cf]"
                    : "border-[#485363] bg-[#485363]/12 text-zinc-300"
                }`}
              >
                {layoutLocked ? "Lock active" : "Manual"}
              </span>
            </div>
            <p className="mt-1 max-w-2xl text-sm leading-6 text-[#9aa5b3]">
              {profile.description?.trim() || "Launch missing apps, pull tray windows forward, and place everything on the selected monitor."}
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
            <button
              className="inline-flex h-10 items-center gap-2 rounded-md border border-[#5db7ff]/60 bg-[#5db7ff] px-4 text-sm font-semibold text-[#071019] transition hover:bg-[#86caff] disabled:cursor-not-allowed disabled:opacity-40"
              onClick={onRestore}
              disabled={busy}
            >
              <Play size={16} />
              Restore
            </button>
            <button
              className={`inline-flex h-10 items-center gap-2 rounded-md border px-4 text-sm font-semibold transition disabled:cursor-not-allowed disabled:opacity-40 ${
                layoutLocked
                  ? "border-[#39d98a]/50 bg-[#39d98a]/15 text-[#a8f3cf] hover:bg-[#39d98a]/20"
                  : "border-[#2a323d] bg-[#111820] text-zinc-200 hover:border-[#455364] hover:bg-[#17202a]"
              }`}
              onClick={onLockToggle}
              disabled={busy}
            >
              {layoutLocked ? <LockKeyhole size={16} /> : <UnlockKeyhole size={16} />}
              {layoutLocked ? "Locked" : "Lock"}
            </button>
            <IconButton label="Refresh" onClick={onRefresh}>
              <RefreshCw size={16} />
            </IconButton>
          </div>
        </div>
      </section>

      <section className="grid gap-3 md:grid-cols-4">
        <Metric label="Monitors" value={String(monitors.length)} detail={monitor ? monitorLabel(monitor) : "No target set"} tone="blue" />
        <Metric label="Visible windows" value={String(visibleWindows)} detail={`${hiddenWindows} hidden or minimized`} tone="green" />
        <Metric label="Profile apps" value={String(profile.apps.length)} detail={profile.name} tone="amber" />
        <Metric label="Startup" value={config.startup.enabled ? "On" : "Off"} detail={startupDetail} tone="neutral" />
      </section>

      <div className="grid gap-4 xl:grid-cols-[1.2fr_0.8fr]">
        <section className="surface rounded-md p-4">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <h2 className="text-lg font-semibold text-zinc-50">Apps in profile</h2>
              <p className="mt-1 text-sm text-[#8a94a3]">What restore will target right now.</p>
            </div>
            <div className="flex flex-wrap items-center justify-end gap-2">
              <label className="sr-only" htmlFor="capture-monitor">
                Capture monitor
              </label>
              <SelectInput
                id="capture-monitor"
                value={captureMonitorId}
                onChange={(event) => onCaptureMonitorChange(event.target.value)}
                className="min-w-52"
                disabled={busy || monitors.length === 0}
              >
                {monitors.map((item) => (
                  <option key={item.id} value={item.id}>
                    {monitorLabel(item)}
                  </option>
                ))}
              </SelectInput>
              <button
                className="inline-flex h-9 items-center gap-2 rounded-md border border-[#2a323d] bg-[#111820] px-3 text-sm font-semibold text-zinc-200 transition hover:border-[#455364] hover:bg-[#17202a] disabled:cursor-not-allowed disabled:opacity-40"
                onClick={onCaptureCurrentLayout}
                disabled={busy || !captureMonitorId}
              >
                <Camera size={15} />
                Capture current layout
              </button>
            </div>
          </div>
          <div className="mt-4 grid gap-2">
            {profile.apps.map((app) => (
              <AppRow key={app.id} app={app} windows={windows} lastRestore={lastRestore} onRemove={() => onRemoveApp(app.id)} />
            ))}
            {profile.apps.length === 0 && (
              <div className="surface-soft rounded-md px-3 py-8 text-center text-sm text-[#8a94a3]">No apps are configured in this profile.</div>
            )}
          </div>
        </section>

        <section className="grid gap-4">
          <section className="surface rounded-md p-4">
            <h2 className="text-sm font-semibold uppercase tracking-normal text-[#8a94a3]">Health</h2>
            <div className="mt-3 grid gap-2">
              {validation.length === 0 && (
                <div className="flex items-center gap-2 rounded-md border border-[#39d98a]/25 bg-[#39d98a]/10 px-3 py-2 text-sm text-[#a8f3cf]">
                  <CheckCircle2 size={16} />
                  Config checks passed
                </div>
              )}
              {validation.map((issue) => (
                <div key={issue} className="flex gap-2 rounded-md border border-amber-400/25 bg-amber-400/10 px-3 py-2 text-sm text-amber-100">
                  <AlertTriangle size={16} className="mt-0.5 shrink-0" />
                  <span>{issue}</span>
                </div>
              ))}
            </div>
          </section>

          <section className="surface rounded-md p-4">
            <h2 className="text-sm font-semibold uppercase tracking-normal text-[#8a94a3]">Monitors</h2>
            <div className="mt-3 grid gap-2">
              {monitors.map((item) => (
                <div key={item.id} className="surface-soft rounded-md px-3 py-2">
                  <div className="flex items-center justify-between gap-3">
                    <span className="flex min-w-0 items-center gap-2 truncate text-sm font-medium text-zinc-200">
                      <Monitor size={15} className="shrink-0 text-[#5db7ff]" />
                      {item.name}
                    </span>
                    {item.isPrimary && <StatusBadge value="success" className="text-[11px]" />}
                  </div>
                  <div className="mt-1 text-xs text-[#8a94a3]">
                    {item.x}, {item.y} - {item.width} x {item.height} - scale {item.scaleFactor.toFixed(2)}
                  </div>
                </div>
              ))}
            </div>
          </section>
        </section>
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
    <div className="grid gap-3 rounded-md border border-[#252b34] bg-[#0d1117] px-3 py-3 md:grid-cols-[minmax(0,1fr)_120px_132px_44px]">
      <div className="min-w-0">
        <div className="truncate text-sm font-semibold text-zinc-100">{app.displayName}</div>
        <div className="mt-1 truncate text-xs text-[#8a94a3]">{app.processName ?? "Process unset"}</div>
      </div>
      <div className="text-xs text-[#8a94a3]">
        <span className="block font-medium text-zinc-300">{app.layout.width} x {app.layout.height}</span>
        <span>
          {app.layout.x}, {app.layout.y}
        </span>
      </div>
      <div className="flex items-center justify-start gap-2 md:justify-end">
        {restoreStatus ? <StatusBadge value={restoreStatus} /> : <StatusBadge value={matches > 0 ? "success" : "skipped"} />}
        <span className="text-xs text-[#8a94a3]">{matches} live</span>
      </div>
      <IconButton label={`Remove ${app.displayName} from profile`} onClick={onRemove} variant="danger" className="justify-self-start md:justify-self-end">
        <Trash2 size={15} />
      </IconButton>
    </div>
  );
}

function Metric({ label, value, detail, tone }: { label: string; value: string; detail: string; tone: "blue" | "green" | "amber" | "neutral" }) {
  const toneClass = {
    blue: "border-[#5db7ff]/30",
    green: "border-[#39d98a]/30",
    amber: "border-[#f7bf4f]/30",
    neutral: "border-[#34404d]",
  }[tone];

  return (
    <div className={`surface-soft rounded-md p-3 ${toneClass}`}>
      <div className="text-[11px] font-semibold uppercase tracking-normal text-[#8a94a3]">{label}</div>
      <div className="mt-2 text-2xl font-semibold text-zinc-50">{value}</div>
      <div className="mt-1 truncate text-xs text-[#8a94a3]">{detail}</div>
    </div>
  );
}
