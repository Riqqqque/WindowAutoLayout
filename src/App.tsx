import {
  AppWindow,
  Boxes,
  FileText,
  LayoutDashboard,
  LoaderCircle,
  PanelsTopLeft,
  RefreshCw,
  RotateCcw,
  Save,
  Settings,
  ShieldCheck,
} from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { clsx } from "clsx";
import { api, isTauriRuntime } from "./lib/api";
import { activeProfile, patchProfile, resolveProfileMonitor, statusText } from "./lib/helpers";
import type {
  AppConfig,
  LogEntry,
  MonitorInfo,
  RestoreResult,
  RuntimeStatus,
  WindowAutoLayoutConfig,
  WindowInfo,
} from "./lib/types";
import { Dashboard } from "./pages/Dashboard";
import { ProfilesPage } from "./pages/Profiles";
import { LayoutEditorPage } from "./pages/LayoutEditor";
import { AppsPage } from "./pages/Apps";
import { LogsPage } from "./pages/Logs";
import { SettingsPage } from "./pages/Settings";

type Page = "dashboard" | "profiles" | "layout" | "apps" | "logs" | "settings";

const navItems: Array<{ id: Page; label: string; icon: typeof LayoutDashboard }> = [
  { id: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { id: "profiles", label: "Profiles", icon: PanelsTopLeft },
  { id: "layout", label: "Layout", icon: Boxes },
  { id: "apps", label: "Apps", icon: AppWindow },
  { id: "logs", label: "Logs", icon: FileText },
  { id: "settings", label: "Settings", icon: Settings },
];

const initialRuntimeStatus: RuntimeStatus = {
  automaticRestoreEnabled: false,
  automaticRestoreProfileId: null,
  automaticRestoreProfileName: null,
  restoring: false,
  startupRegistered: false,
};

export default function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const [config, setConfig] = useState<WindowAutoLayoutConfig | null>(null);
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [presets, setPresets] = useState<AppConfig[]>([]);
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
  const [selectedAppId, setSelectedAppId] = useState<string | null>(null);
  const [selectedWindowHandle, setSelectedWindowHandle] = useState<string | null>(null);
  const [lastRestore, setLastRestore] = useState<RestoreResult | null>(null);
  const [validation, setValidation] = useState<string[]>([]);
  const [configPath, setConfigPath] = useState("");
  const [logPath, setLogPath] = useState("");
  const [dirty, setDirty] = useState(false);
  const [busy, setBusy] = useState(false);
  const [showGrid, setShowGrid] = useState(true);
  const [message, setMessage] = useState<string | null>(null);
  const [runtimeStatus, setRuntimeStatus] = useState<RuntimeStatus>(initialRuntimeStatus);
  const [captureMonitorId, setCaptureMonitorId] = useState<string | null>(null);
  const workspaceContentRef = useRef<HTMLDivElement>(null);

  const profile = useMemo(() => (config ? activeProfile(config, selectedProfileId) : null), [config, selectedProfileId]);
  const effectiveCaptureMonitorId = useMemo(() => {
    if (!config || !profile) return "";
    if (captureMonitorId && monitors.some((monitor) => monitor.id === captureMonitorId)) {
      return captureMonitorId;
    }
    return resolveProfileMonitor(config, profile, monitors).monitor?.id ?? monitors[0]?.id ?? "";
  }, [captureMonitorId, config, monitors, profile]);
  const selectedProfileIsAutomatic = runtimeStatus.automaticRestoreEnabled
    && runtimeStatus.automaticRestoreProfileId === profile?.id;
  const operationBusy = busy || runtimeStatus.restoring;

  const refresh = useCallback(async () => {
    const [nextConfig, nextMonitors, nextWindows, nextLogs, nextPresets, nextConfigPath, nextLogPath, nextValidation, nextRuntimeStatus] =
      await Promise.all([
        api.getConfig(),
        api.monitors(),
        api.windows(),
        api.logs(),
        api.presets(),
        api.configPath(),
        api.logPath(),
        api.validateConfig(),
        api.runtimeStatus(),
      ]);
    setConfig(nextConfig);
    setMonitors(nextMonitors);
    setWindows(nextWindows);
    setLogs(nextLogs);
    setPresets(nextPresets);
    setConfigPath(nextConfigPath);
    setLogPath(nextLogPath);
    setValidation(nextValidation);
    setRuntimeStatus(nextRuntimeStatus);
    setSelectedProfileId((current) =>
      current && nextConfig.profiles.some((item) => item.id === current)
        ? current
        : nextConfig.startup.defaultProfileId ?? nextConfig.profiles[0]?.id ?? null,
    );
    setCaptureMonitorId((current) => {
      if (current && nextMonitors.some((monitor) => monitor.id === current)) return current;
      const nextProfile = activeProfile(nextConfig, nextConfig.startup.defaultProfileId ?? nextConfig.profiles[0]?.id ?? null);
      return resolveProfileMonitor(nextConfig, nextProfile, nextMonitors).monitor?.id ?? nextMonitors[0]?.id ?? null;
    });
    setDirty(false);
  }, []);

  useEffect(() => {
    refresh().catch((error) => setMessage(String(error)));
  }, [refresh]);

  useEffect(() => {
    if (!isTauriRuntime) return undefined;
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    listen<RuntimeStatus>("runtime-status-changed", (event) => {
      setRuntimeStatus(event.payload);
      setConfig((current) => current ? {
        ...current,
        enforcement: {
          ...current.enforcement,
          enabled: event.payload.automaticRestoreEnabled,
          profileId: event.payload.automaticRestoreProfileId ?? current.enforcement.profileId,
        },
      } : current);
    }).then((dispose) => {
      if (cancelled) dispose();
      else unlisten = dispose;
    }).catch((error) => setMessage(String(error)));

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (!profile) return;
    setSelectedProfileId((current) => (current === profile.id ? current : profile.id));
    setSelectedAppId((current) =>
      current && profile.apps.some((app) => app.id === current)
        ? current
        : profile.apps[0]?.id ?? null,
    );
  }, [profile]);

  useEffect(() => {
    workspaceContentRef.current?.scrollTo({ top: 0, left: 0 });
    window.scrollTo({ top: 0, left: 0 });
  }, [page]);

  const restoreSelected = useCallback(async () => {
    if (!config || !profile) return;
    setBusy(true);
    try {
      const saved = dirty ? await api.saveConfig(config) : config;
      setConfig(saved);
      setDirty(false);
      const result = await api.restoreProfile(profile.id, saved.startup.launchMissingApps);
      setLastRestore(result);
      setLogs(await api.logs());
      setMessage(`Restore finished: ${statusText(result.status)}`);
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }, [config, dirty, profile]);

  function updateConfig(next: WindowAutoLayoutConfig) {
    setConfig(next);
    setDirty(true);
  }

  function removeAppFromProfile(appId: string) {
    if (!config || !profile) return;
    const removed = profile.apps.find((item) => item.id === appId);
    const remaining = profile.apps.filter((item) => item.id !== appId);
    updateConfig(patchProfile(config, profile.id, (profile) => ({ ...profile, apps: profile.apps.filter((item) => item.id !== appId) })));

    if (!remaining.some((item) => item.id === selectedAppId)) {
      setSelectedAppId(remaining[0]?.id ?? null);
    }
    if (selectedWindowHandle) {
      setSelectedWindowHandle(null);
    }
    setMessage(removed ? `Removed ${removed.displayName}` : "Removed app");
  }

  async function saveConfig() {
    if (!config) return;
    setBusy(true);
    try {
      const saved = await api.saveConfig(config);
      setConfig(saved);
      setDirty(false);
      setValidation(await api.validateConfig());
      setMessage("Saved");
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function toggleLayoutLock() {
    if (!config || !profile) return;
    setBusy(true);
    try {
      const saved = dirty ? await api.saveConfig(config) : config;
      setConfig(saved);
      setDirty(false);
      const enabling = !selectedProfileIsAutomatic;
      const switchingProfile = runtimeStatus.automaticRestoreEnabled && enabling;
      if (enabling) {
        const result = await api.restoreProfile(profile.id, saved.startup.launchMissingApps);
        setLastRestore(result);
        if (["paused", "failed", "monitorMissing"].includes(result.status)) {
          setMessage(`Automatic restore not enabled: ${statusText(result.status)}`);
          return;
        }
      }
      const enabled = await api.setLayoutLock(enabling, profile.id);
      setConfig({ ...saved, enforcement: { ...saved.enforcement, enabled, profileId: profile.id } });
      setRuntimeStatus(await api.runtimeStatus());
      setLogs(await api.logs());
      setMessage(enabled ? (switchingProfile ? `Automatic restore now uses ${profile.name}` : "Automatic restore on") : "Automatic restore off");
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function captureCurrentLayout() {
    if (!config || !profile) return;
    if (!effectiveCaptureMonitorId) {
      setMessage("Pick a monitor before capturing");
      return;
    }
    setBusy(true);
    try {
      const saved = dirty ? await api.saveConfig(config) : config;
      setConfig(saved);
      setDirty(false);
      const result = await api.captureCurrentLayout(profile.id, effectiveCaptureMonitorId);
      const capturedProfile = result.config.profiles.find((item) => item.id === profile.id);
      setConfig(result.config);
      setSelectedProfileId(profile.id);
      setSelectedAppId(capturedProfile?.apps[0]?.id ?? null);
      setCaptureMonitorId(result.monitor.id);
      setMessage(`Captured ${result.capturedCount} window${result.capturedCount === 1 ? "" : "s"} on ${result.monitor.name}`);
      await refreshWindowsAndLogs();
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function saveSelectedWindow() {
    if (!profile || !selectedAppId || !selectedWindowHandle) return;
    setBusy(true);
    try {
      const nextConfig = await api.saveWindowLayout(profile.id, selectedAppId, selectedWindowHandle);
      setConfig(nextConfig);
      setDirty(false);
      setMessage("Captured selected window");
      await refreshWindowsAndLogs();
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function refreshWindowsAndLogs() {
    const [nextWindows, nextLogs, nextValidation] = await Promise.all([api.windows(), api.logs(), api.validateConfig()]);
    setWindows(nextWindows);
    setLogs(nextLogs);
    setValidation(nextValidation);
  }

  function refreshFromUi() {
    if (dirty) {
      setMessage("Save your changes before refreshing");
      return;
    }
    setBusy(true);
    refresh()
      .catch((error) => setMessage(String(error)))
      .finally(() => setBusy(false));
  }

  if (!config || !profile) {
    return (
      <main className="app-frame flex min-h-screen items-center justify-center text-zinc-200">
        <div className="panel flex items-center gap-3 px-4 py-3">
          <RefreshCw className="animate-spin text-[#43c7e7]" size={18} />
          <span className="text-sm font-medium">Loading WindowAutoLayout</span>
        </div>
      </main>
    );
  }

  return (
    <main className="app-frame min-h-screen text-zinc-100">
      <div className="app-shell">
        <aside className="sidebar">
          <div className="brand">
            <div className="brand-mark">
              <PanelsTopLeft size={18} />
            </div>
            <div className="min-w-0">
              <div className="truncate text-sm font-semibold text-zinc-50">WindowAutoLayout</div>
              <div className="text-xs text-[#71818c]">v{config.appVersion}</div>
            </div>
          </div>

          <div className="sidebar-context">
            <div className="eyebrow">Active profile</div>
            <div className="mt-1 truncate text-sm font-medium text-zinc-100">{profile.name}</div>
            <div className="mt-2 flex items-center gap-2 text-xs text-[#8fa0aa]">
              <span className={`h-2 w-2 rounded-full ${selectedProfileIsAutomatic ? "bg-[#42d392]" : runtimeStatus.automaticRestoreEnabled ? "bg-[#f1b84b]" : "bg-[#50606b]"}`} />
              {selectedProfileIsAutomatic
                ? "Automatic restore on"
                : runtimeStatus.automaticRestoreEnabled
                  ? `On for ${runtimeStatus.automaticRestoreProfileName ?? "another profile"}`
                  : "Automatic restore off"}
            </div>
          </div>

          <nav className="sidebar-nav">
            {navItems.map((item) => {
              const Icon = item.icon;
              return (
                <button
                  key={item.id}
                  className={clsx(
                    "nav-item group text-sm",
                    page === item.id && "nav-item-active",
                  )}
                  aria-current={page === item.id ? "page" : undefined}
                  onClick={() => setPage(item.id)}
                >
                  <Icon size={16} className={page === item.id ? "text-[#43c7e7]" : "text-[#73838f] group-hover:text-zinc-300"} />
                  {item.label}
                </button>
              );
            })}
          </nav>
          <div className="sidebar-footer">
            <ShieldCheck size={15} className="text-[#42d392]" />
            <span>No input hooks</span>
          </div>
        </aside>

        <section className="workspace">
          <header className="topbar">
            <div className="min-w-0">
              <div className="truncate text-[15px] font-semibold text-zinc-100">{navItems.find((item) => item.id === page)?.label}</div>
              <div className="mt-0.5 flex flex-wrap items-center gap-2 text-xs text-[#82919c]">
                <span>{message ?? (dirty ? "Unsaved changes" : `${profile.apps.length} apps in ${profile.name}`)}</span>
                {runtimeStatus.restoring && (
                  <>
                    <span aria-hidden="true">/</span>
                    <span className="inline-flex items-center gap-1 text-[#aef2d1]">
                      <LoaderCircle className="animate-spin" size={12} />
                      restoring
                    </span>
                  </>
                )}
              </div>
            </div>
            <div className="topbar-actions">
              <button
                className={`runtime-control ${selectedProfileIsAutomatic ? "runtime-control-on" : ""}`}
                onClick={toggleLayoutLock}
                disabled={operationBusy}
                aria-pressed={selectedProfileIsAutomatic}
                title={selectedProfileIsAutomatic ? "Turn automatic restore off" : `Use ${profile.name} for automatic restore`}
              >
                <span className="runtime-dot" aria-hidden="true" />
                <span className="min-w-0 text-left">
                  <span className="block text-[10px] font-semibold uppercase text-[#7f909b]">Automatic restore</span>
                  <span className="block truncate text-xs font-semibold">
                    {runtimeStatus.automaticRestoreEnabled
                      ? runtimeStatus.automaticRestoreProfileName ?? "On"
                      : "Off"}
                  </span>
                </span>
              </button>
              <button
                className="icon-button"
                onClick={refreshFromUi}
                disabled={operationBusy}
                aria-label="Refresh app data"
                title="Refresh app data"
              >
                <RefreshCw size={15} />
              </button>
              <button
                className="button-secondary"
                onClick={saveConfig}
                disabled={operationBusy || !dirty}
              >
                <Save size={15} />
                Save
              </button>
              <button
                className="button-primary restore-command"
                onClick={restoreSelected}
                disabled={operationBusy}
              >
                {runtimeStatus.restoring ? <LoaderCircle className="animate-spin" size={16} /> : <RotateCcw size={16} />}
                {runtimeStatus.restoring ? "Restoring" : "Restore now"}
              </button>
            </div>
          </header>

          <div ref={workspaceContentRef} className="workspace-content">
            {page === "dashboard" && (
              <Dashboard
                config={config}
                profile={profile}
                monitors={monitors}
                windows={windows}
                lastRestore={lastRestore?.profileId === profile.id ? lastRestore : null}
                validation={validation}
                busy={operationBusy}
                captureMonitorId={effectiveCaptureMonitorId}
                onProfileChange={(id) => {
                  setSelectedProfileId(id);
                  const nextProfile = config.profiles.find((item) => item.id === id);
                  setSelectedAppId(nextProfile?.apps[0]?.id ?? null);
                  setCaptureMonitorId(nextProfile ? resolveProfileMonitor(config, nextProfile, monitors).monitor?.id ?? monitors[0]?.id ?? null : null);
                }}
                onRestore={restoreSelected}
                onLockToggle={toggleLayoutLock}
                runtimeStatus={runtimeStatus}
                onRefresh={refreshFromUi}
                onCaptureMonitorChange={setCaptureMonitorId}
                onCaptureCurrentLayout={captureCurrentLayout}
                onRemoveApp={removeAppFromProfile}
              />
            )}
            {page === "profiles" && (
              <ProfilesPage
                config={config}
                profile={profile}
                monitors={monitors}
                onConfigChange={updateConfig}
                onProfileChange={(id) => {
                  setSelectedProfileId(id);
                  setSelectedAppId(config.profiles.find((item) => item.id === id)?.apps[0]?.id ?? null);
                }}
              />
            )}
            {page === "layout" && (
              <LayoutEditorPage
                config={config}
                profile={profile}
                monitors={monitors}
                windows={windows}
                selectedAppId={selectedAppId}
                selectedWindowHandle={selectedWindowHandle}
                showGrid={showGrid}
                onConfigChange={updateConfig}
                onSelectedAppChange={setSelectedAppId}
                onSelectedWindowChange={setSelectedWindowHandle}
                onRefreshWindows={() => refreshWindowsAndLogs().catch((error) => setMessage(String(error)))}
                onSaveSelectedWindow={saveSelectedWindow}
                onShowGridChange={setShowGrid}
                onRemoveApp={removeAppFromProfile}
              />
            )}
            {page === "apps" && (
              <AppsPage
                config={config}
                profile={profile}
                monitors={monitors}
                presets={presets}
                selectedAppId={selectedAppId}
                onSelectedAppChange={setSelectedAppId}
                onConfigChange={updateConfig}
                onRemoveApp={removeAppFromProfile}
              />
            )}
            {page === "logs" && (
              <LogsPage
                logs={logs}
                onRefresh={() => api.logs().then(setLogs).catch((error) => setMessage(String(error)))}
                onClear={() => api.clearLogs().then(() => setLogs([])).catch((error) => setMessage(String(error)))}
                onOpen={() => api.openLogFile().catch((error) => setMessage(String(error)))}
              />
            )}
            {page === "settings" && (
              <SettingsPage
                config={config}
                monitors={monitors}
                configPath={configPath}
                logPath={logPath}
                runtimeStatus={runtimeStatus}
                onConfigChange={updateConfig}
                onSave={saveConfig}
              />
            )}
          </div>
        </section>
      </div>
    </main>
  );
}
