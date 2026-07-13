import { Check, Download, ShieldCheck, Upload } from "lucide-react";
import { useMemo, useState } from "react";
import { Field, NumberInput, SelectInput, TextArea, Toggle } from "../components/Form";
import { IconButton } from "../components/IconButton";
import { api } from "../lib/api";
import type { MonitorInfo, MonitorMissingBehavior, WindowAutoLayoutConfig } from "../lib/types";

interface SettingsProps {
  config: WindowAutoLayoutConfig;
  monitors: MonitorInfo[];
  configPath?: string;
  logPath?: string;
  onConfigChange: (config: WindowAutoLayoutConfig) => void;
  onSave: () => void;
}

const fallbackModes: Array<{ value: MonitorMissingBehavior; label: string }> = [
  { value: "doNothing", label: "Do nothing" },
  { value: "usePrimary", label: "Use primary display" },
  { value: "nearestMatch", label: "Use nearest match" },
];

export function SettingsPage({ config, monitors, configPath, logPath, onConfigChange, onSave }: SettingsProps) {
  const [importText, setImportText] = useState("");
  const [importError, setImportError] = useState<string | null>(null);
  const exportText = useMemo(() => JSON.stringify(config, null, 2), [config]);
  const missingDefaultMonitor =
    config.global.defaultMonitorId && !monitors.some((monitor) => monitor.id === config.global.defaultMonitorId)
      ? config.global.defaultMonitorId
      : null;

  async function importConfig() {
    try {
      onConfigChange(await api.parseConfigJson(importText));
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
    <div className="page-stack">
      <div className="grid gap-4 lg:grid-cols-2">
        <section className="panel p-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <div className="eyebrow">Windows</div>
              <h1 className="mt-1 section-heading">Startup and tray</h1>
            </div>
            <IconButton label="Save settings" onClick={onSave} variant="solid">
              <Check size={16} />
            </IconButton>
          </div>

          <div className="mt-4 max-w-xs">
            <Field label="Startup delay seconds">
              <NumberInput
                min={0}
                max={300}
                value={config.startup.delaySeconds}
                onChange={(event) => onConfigChange({ ...config, startup: { ...config.startup, delaySeconds: Number(event.target.value) } })}
              />
            </Field>
          </div>
          <div className="mt-4 grid gap-2 md:grid-cols-2">
            <Toggle label="Start with Windows" checked={config.startup.enabled} onChange={(checked) => onConfigChange({ ...config, startup: { ...config.startup, enabled: checked } })} />
            <Toggle label="Start minimized to tray" checked={config.startup.startMinimizedToTray} onChange={(checked) => onConfigChange({ ...config, startup: { ...config.startup, startMinimizedToTray: checked } })} />
            <Toggle label="Restore on launch" checked={config.startup.restoreOnLaunch} onChange={(checked) => onConfigChange({ ...config, startup: { ...config.startup, restoreOnLaunch: checked } })} />
            <Toggle label="Close button goes to tray" checked={config.tray.minimizeToTrayOnClose} onChange={(checked) => onConfigChange({ ...config, tray: { ...config.tray, minimizeToTrayOnClose: checked } })} />
          </div>
        </section>

        <section className="panel p-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <div className="eyebrow">Placement</div>
              <h2 className="mt-1 section-heading">Displays and game safety</h2>
            </div>
            <ShieldCheck size={19} className="text-[#42d392]" />
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
            <Field label="Missing monitor">
              <SelectInput
                value={config.global.monitorMissingBehavior}
                onChange={(event) => onConfigChange({
                  ...config,
                  global: { ...config.global, monitorMissingBehavior: event.target.value as MonitorMissingBehavior },
                })}
              >
                {fallbackModes.map((mode) => (
                  <option key={mode.value} value={mode.value}>
                    {mode.label}
                  </option>
                ))}
              </SelectInput>
            </Field>
          </div>
          <div className="mt-4 grid gap-2 border-t border-[#27313a] pt-4 text-sm">
            <div className="flex items-center justify-between"><span className="text-[#91a0ab]">Game protection</span><span className="text-[#aef2d1]">Always on</span></div>
            <div className="flex items-center justify-between"><span className="text-[#91a0ab]">Input hooks</span><span className="text-[#aef2d1]">None</span></div>
            <div className="flex items-center justify-between"><span className="text-[#91a0ab]">Background lock</span><span className="text-[#aef2d1]">Event driven</span></div>
          </div>
        </section>
      </div>

      <section className="panel p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <div className="eyebrow">Configuration</div>
            <h2 className="mt-1 section-heading">Import and export</h2>
          </div>
          <div className="flex gap-2">
            <IconButton label="Download config JSON" onClick={downloadConfig}>
              <Download size={16} />
            </IconButton>
            <IconButton label="Import pasted JSON" onClick={importConfig} disabled={!importText.trim()} variant="solid">
              <Upload size={16} />
            </IconButton>
          </div>
        </div>
        <div className="mt-4 grid gap-4 lg:grid-cols-2">
          <Field label="Current config">
            <TextArea value={exportText} readOnly className="min-h-64 font-mono text-xs" />
          </Field>
          <Field label="Import JSON">
            <TextArea value={importText} onChange={(event) => setImportText(event.target.value)} className="min-h-64 font-mono text-xs" />
          </Field>
        </div>
        {importError && <div className="mt-3 rounded-md border border-rose-400/30 bg-rose-400/10 px-3 py-2 text-sm text-rose-100">{importError}</div>}
        <div className="mt-4 grid gap-1 border-t border-[#27313a] pt-3 text-xs text-[#71818c]">
          {configPath && <div className="truncate" title={configPath}>Config: {configPath}</div>}
          {logPath && <div className="truncate" title={logPath}>Logs: {logPath}</div>}
        </div>
      </section>
    </div>
  );
}
