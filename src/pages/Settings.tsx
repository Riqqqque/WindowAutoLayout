import { Check, Download, Upload } from "lucide-react";
import { useMemo, useState } from "react";
import { Field, NumberInput, SelectInput, TextArea, TextInput, Toggle } from "../components/Form";
import { IconButton } from "../components/IconButton";
import type { MonitorInfo, MonitorMissingBehavior, WindowAutoLayoutConfig } from "../lib/types";

interface SettingsProps {
  config: WindowAutoLayoutConfig;
  monitors: MonitorInfo[];
  configPath?: string;
  logPath?: string;
  onConfigChange: (config: WindowAutoLayoutConfig) => void;
  onSave: () => void;
}

const fallbackModes: MonitorMissingBehavior[] = ["doNothing", "usePrimary", "nearestMatch", "askNextOpen"];

export function SettingsPage({ config, monitors, configPath, logPath, onConfigChange, onSave }: SettingsProps) {
  const [importText, setImportText] = useState("");
  const [importError, setImportError] = useState<string | null>(null);
  const exportText = useMemo(() => JSON.stringify(config, null, 2), [config]);
  const missingDefaultMonitor =
    config.global.defaultMonitorId && !monitors.some((monitor) => monitor.id === config.global.defaultMonitorId)
      ? config.global.defaultMonitorId
      : null;

  function importConfig() {
    try {
      const parsed = JSON.parse(importText) as WindowAutoLayoutConfig;
      onConfigChange(parsed);
      setImportError(null);
    } catch (error) {
      setImportError(`Import failed: ${String(error)}`);
    }
  }

  function downloadConfig() {
    const blob = new Blob([exportText], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = "windowautolayout-config.json";
    anchor.click();
    URL.revokeObjectURL(url);
  }

  return (
    <div className="grid gap-4 xl:grid-cols-2">
      <section className="surface rounded-md p-4">
        <div className="flex items-center justify-between gap-3">
          <h1 className="text-lg font-semibold text-zinc-50">Settings</h1>
          <IconButton label="Save settings" onClick={onSave} variant="solid">
            <Check size={16} />
          </IconButton>
        </div>

        <div className="mt-4 grid gap-4 md:grid-cols-2">
          <Field label="Default monitor">
            <SelectInput
              value={config.global.defaultMonitorId ?? ""}
              onChange={(event) => onConfigChange({ ...config, global: { ...config.global, defaultMonitorId: event.target.value || null } })}
            >
              <option value="">No default</option>
              {missingDefaultMonitor && <option value={missingDefaultMonitor}>Missing: {missingDefaultMonitor}</option>}
              {monitors.map((monitor) => (
                <option key={monitor.id} value={monitor.id}>
                  {monitor.name} - {monitor.width}x{monitor.height}
                </option>
              ))}
            </SelectInput>
          </Field>
          <Field label="Monitor fallback">
            <SelectInput
              value={config.global.monitorMissingBehavior}
              onChange={(event) =>
                onConfigChange({
                  ...config,
                  global: { ...config.global, monitorMissingBehavior: event.target.value as MonitorMissingBehavior },
                })
              }
            >
              {fallbackModes.map((mode) => (
                <option key={mode} value={mode}>
                  {mode}
                </option>
              ))}
            </SelectInput>
          </Field>
          <Field label="Startup delay seconds">
            <NumberInput
              min={0}
              value={config.startup.delaySeconds}
              onChange={(event) => onConfigChange({ ...config, startup: { ...config.startup, delaySeconds: Number(event.target.value) } })}
            />
          </Field>
          <Field label="Hotkey">
            <TextInput
              value={config.hotkey.accelerator}
              onChange={(event) => onConfigChange({ ...config, hotkey: { ...config.hotkey, accelerator: event.target.value } })}
            />
          </Field>
          <Field label="Lock interval ms">
            <NumberInput
              min={1000}
              max={5000}
              step={500}
              value={config.enforcement.intervalMs}
              onChange={(event) =>
                onConfigChange({ ...config, enforcement: { ...config.enforcement, intervalMs: Number(event.target.value) } })
              }
            />
          </Field>
        </div>

        <div className="mt-4 grid gap-2 md:grid-cols-2">
          <Toggle label="Start with Windows" checked={config.startup.enabled} onChange={(checked) => onConfigChange({ ...config, startup: { ...config.startup, enabled: checked } })} />
          <Toggle label="Start minimized to tray" checked={config.startup.startMinimizedToTray} onChange={(checked) => onConfigChange({ ...config, startup: { ...config.startup, startMinimizedToTray: checked } })} />
          <Toggle label="Restore on launch" checked={config.startup.restoreOnLaunch} onChange={(checked) => onConfigChange({ ...config, startup: { ...config.startup, restoreOnLaunch: checked } })} />
          <Toggle label="Minimize close button to tray" checked={config.tray.minimizeToTrayOnClose} onChange={(checked) => onConfigChange({ ...config, tray: { ...config.tray, minimizeToTrayOnClose: checked } })} />
          <Toggle label="Enable hotkey" checked={config.hotkey.enabled} onChange={(checked) => onConfigChange({ ...config, hotkey: { ...config.hotkey, enabled: checked } })} />
          <Toggle label="Pause lock during fullscreen apps" checked={config.enforcement.pauseForFullscreenGames} onChange={(checked) => onConfigChange({ ...config, enforcement: { ...config.enforcement, pauseForFullscreenGames: checked } })} />
          <Toggle label="Warn when monitor is missing" checked={config.global.warnWhenMonitorMissing} onChange={(checked) => onConfigChange({ ...config, global: { ...config.global, warnWhenMonitorMissing: checked } })} />
          <Toggle label="Advanced mode" checked={config.global.advancedMode} onChange={(checked) => onConfigChange({ ...config, global: { ...config.global, advancedMode: checked } })} />
        </div>

        <div className="mt-4 grid gap-2 text-xs text-zinc-500">
          {configPath && <div className="truncate">Config: {configPath}</div>}
          {logPath && <div className="truncate">Logs: {logPath}</div>}
        </div>
      </section>

      <section className="surface rounded-md p-4">
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-lg font-semibold text-zinc-50">Import / export</h2>
          <div className="flex gap-2">
            <IconButton label="Download config JSON" onClick={downloadConfig}>
              <Download size={16} />
            </IconButton>
            <IconButton label="Import pasted JSON" onClick={importConfig} disabled={!importText.trim()}>
              <Upload size={16} />
            </IconButton>
          </div>
        </div>
        <div className="mt-4 grid gap-4">
          <Field label="Current config">
            <TextArea value={exportText} readOnly className="min-h-64 font-mono text-xs" />
          </Field>
          <Field label="Import JSON">
            <TextArea value={importText} onChange={(event) => setImportText(event.target.value)} className="min-h-40 font-mono text-xs" />
          </Field>
          {importError && <div className="rounded-md border border-rose-400/30 bg-rose-400/10 px-3 py-2 text-sm text-rose-100">{importError}</div>}
        </div>
      </section>
    </div>
  );
}
