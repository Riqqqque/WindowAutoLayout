import {
  AppWindow,
  Boxes,
  FileText,
  LayoutDashboard,
  PanelsTopLeft,
  RefreshCw,
  Save,
  Settings,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { register, unregisterAll } from "@tauri-apps/plugin-global-shortcut";
import { clsx } from "clsx";
import { api, isTauriRuntime } from "./lib/api";
import { activeProfile } from "./lib/helpers";
import type {
  AppConfig,
  LogEntry,
  MonitorInfo,
  RestoreResult,
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
  const [layoutLocked, setLayoutLocked] = useState(false);

  const profile = useMemo(() => (config ? activeProfile(config, selectedProfileId) : null), [config, selectedProfileId]);

  const refresh = useCallback(async () => {
    const [nextConfig, nextMonitors, nextWindows, nextLogs, nextPresets, nextConfigPath, nextLogPath, nextValidation, nextLayoutLocked] =
      await Promise.all([
        api.getConfig(),
        api.monitors(),
        api.windows(),
        api.logs(),
        api.presets(),
        api.configPath(),
        api.logPath(),
        api.validateConfig(),
        api.layoutLockStatus(),
      ]);
    setConfig(nextConfig);
    setMonitors(nextMonitors);
    setWindows(nextWindows);
    setLogs(nextLogs);
    setPresets(nextPresets);
    setConfigPath(nextConfigPath);
    setLogPath(nextLogPath);
    setValidation(nextValidation);
    setLayoutLocked(nextLayoutLocked);
    setSelectedProfileId((current) => current ?? nextConfig.startup.defaultProfileId ?? nextConfig.profiles[0]?.id ?? null);
    setSelectedAppId((current) => current ?? nextConfig.profiles[0]?.apps[0]?.id ?? null);
    setDirty(false);
  }, []);

  useEffect(() => {
    refresh().catch((error) => setMessage(String(error)));
  }, [refresh]);

  const restoreSelected = useCallback(async () => {
    if (!config || !profile) return;
    setBusy(true);
    try {
      const saved = dirty ? await api.saveConfig(config) : config;
      setConfig(saved);
      setDirty(false);
      const result = await api.restoreProfile(profile.id, true);
      setLastRestore(result);
      setLogs(await api.logs());
      setMessage(`Restore finished: ${result.status}`);
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }, [config, dirty, profile]);

  useEffect(() => {
    if (!config?.hotkey.enabled || !config.hotkey.accelerator.trim()) {
      if (isTauriRuntime) {
        unregisterAll().catch(() => undefined);
      }
      return;
    }

    if (!isTauriRuntime) {
      return;
    }

    let disposed = false;
    const accelerator = normalizeAccelerator(config.hotkey.accelerator);
    unregisterAll()
      .then(() => register(accelerator, () => restoreSelected()))
      .catch((error) => setMessage(`Hotkey registration failed: ${String(error)}`));

    return () => {
      disposed = true;
      if (!disposed) return;
      unregisterAll().catch(() => undefined);
    };
  }, [config?.hotkey.enabled, config?.hotkey.accelerator, restoreSelected]);

  function updateConfig(next: WindowAutoLayoutConfig) {
    setConfig(next);
    setDirty(true);
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
      const enabled = await api.setLayoutLock(!layoutLocked, profile.id);
      setConfig({ ...saved, enforcement: { ...saved.enforcement, enabled, profileId: profile.id } });
      setLayoutLocked(enabled);
      setLogs(await api.logs());
      setMessage(enabled ? "Layout lock on" : "Layout lock off");
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function saveAllLayouts() {
    if (!profile) return;
    setBusy(true);
    try {
      const nextConfig = await api.saveAllCurrentLayouts(profile.id);
      setConfig(nextConfig);
      setDirty(false);
      setMessage("Captured matching windows");
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

  if (!config || !profile) {
    return (
      <main className="flex min-h-screen items-center justify-center bg-zinc-950 text-zinc-200">
        <RefreshCw className="mr-3 animate-spin" size={18} />
        Loading WindowAutoLayout
      </main>
    );
  }

  return (
    <main className="min-h-screen bg-zinc-950 text-zinc-100">
      <div className="grid min-h-screen lg:grid-cols-[240px_1fr]">
        <aside className="border-b border-zinc-800 bg-zinc-950/95 p-3 lg:border-b-0 lg:border-r">
          <div className="flex h-12 items-center gap-3 px-2">
            <div className="flex h-9 w-9 items-center justify-center rounded-md border border-cyan-300/40 bg-cyan-300/15 text-cyan-100">
              <PanelsTopLeft size={18} />
            </div>
            <div>
              <div className="font-semibold text-zinc-50">WindowAutoLayout</div>
              <div className="text-xs text-zinc-500">v{config.appVersion}</div>
            </div>
          </div>

          <nav className="mt-4 grid gap-1">
            {navItems.map((item) => {
              const Icon = item.icon;
              return (
                <button
                  key={item.id}
                  className={clsx(
                    "flex h-10 items-center gap-3 rounded-md px-3 text-sm transition",
                    page === item.id
                      ? "bg-zinc-800 text-zinc-50"
                      : "text-zinc-400 hover:bg-zinc-900 hover:text-zinc-100",
                  )}
                  onClick={() => setPage(item.id)}
                >
                  <Icon size={16} />
                  {item.label}
                </button>
              );
            })}
          </nav>
        </aside>

        <section className="min-w-0">
          <header className="sticky top-0 z-30 flex h-14 items-center justify-between gap-3 border-b border-zinc-800 bg-zinc-950/90 px-4 backdrop-blur">
            <div className="truncate text-sm text-zinc-400">
              {message ?? (dirty ? "Unsaved changes" : "Ready")}
            </div>
            <div className="flex gap-2">
              <button
                className="inline-flex h-9 items-center gap-2 rounded-md border border-zinc-700 bg-zinc-900 px-3 text-sm text-zinc-200 hover:bg-zinc-800"
                onClick={() => refresh().catch((error) => setMessage(String(error)))}
              >
                <RefreshCw size={15} />
                Refresh
              </button>
              <button
                className="inline-flex h-9 items-center gap-2 rounded-md border border-cyan-300/40 bg-cyan-300 px-3 text-sm font-medium text-zinc-950 hover:bg-cyan-200 disabled:opacity-40"
                onClick={saveConfig}
                disabled={busy || !dirty}
              >
                <Save size={15} />
                Save
              </button>
            </div>
          </header>

          <div className="p-4">
            {page === "dashboard" && (
              <Dashboard
                config={config}
                profile={profile}
                monitors={monitors}
                windows={windows}
                lastRestore={lastRestore}
                validation={validation}
                busy={busy}
                onProfileChange={(id) => {
                  setSelectedProfileId(id);
                  setSelectedAppId(config.profiles.find((item) => item.id === id)?.apps[0]?.id ?? null);
                }}
                onRestore={restoreSelected}
                onLockToggle={toggleLayoutLock}
                layoutLocked={layoutLocked}
                onRefresh={() => refresh().catch((error) => setMessage(String(error)))}
                onSaveAll={saveAllLayouts}
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

function normalizeAccelerator(accelerator: string) {
  return accelerator
    .replace(/\bCtrl\b/gi, "CommandOrControl")
    .replace(/\bControl\b/gi, "CommandOrControl")
    .replace(/\s+/g, "");
}
