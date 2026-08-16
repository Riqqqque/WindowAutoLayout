import { Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { Field, NumberInput, SelectInput, TextArea, TextInput, Toggle } from "../components/Form";
import { IconButton } from "../components/IconButton";
import { capturedDisplayForMonitor, formatArguments, newId, parseArguments, patchApp, patchProfile, statusText } from "../lib/helpers";
import type { AppConfig, MonitorInfo, Profile, TitleMatchMode, WindowAutoLayoutConfig, WindowStatePreference } from "../lib/types";

interface AppsProps {
  config: WindowAutoLayoutConfig;
  profile: Profile;
  monitors: MonitorInfo[];
  presets: AppConfig[];
  selectedAppId?: string | null;
  onSelectedAppChange: (appId: string) => void;
  onConfigChange: (config: WindowAutoLayoutConfig) => void;
  onRemoveApp: (appId: string) => void;
}

type EditorTab = "general" | "matching" | "behavior";

const titleModes: TitleMatchMode[] = ["contains", "exact", "startsWith", "endsWith", "regex"];
const windowStates: WindowStatePreference[] = ["normal", "maximized", "minimized"];

export function AppsPage({
  config,
  profile,
  monitors,
  presets,
  selectedAppId,
  onSelectedAppChange,
  onConfigChange,
  onRemoveApp,
}: AppsProps) {
  const [tab, setTab] = useState<EditorTab>("general");
  const app = profile.apps.find((item) => item.id === selectedAppId) ?? profile.apps[0];
  const missingTargetMonitor =
    app?.targetMonitorId && !monitors.some((monitor) => monitor.id === app.targetMonitorId)
      ? app.targetMonitorId
      : null;

  function updateApp(appId: string, update: (app: AppConfig) => AppConfig) {
    onConfigChange(patchApp(config, profile.id, appId, update));
  }

  function addCustomApp() {
    const id = newId("app");
    const next: AppConfig = {
      id,
      displayName: "New App",
      executablePath: null,
      arguments: [],
      workingDirectory: null,
      processName: null,
      titleRule: null,
      className: null,
      targetMonitorId: null,
      capturedDisplay: null,
      layout: { x: 80, y: 80, width: 960, height: 640 },
      windowState: "normal",
      launchDelaySeconds: 0,
      detectionTimeoutSeconds: 25,
      retryIntervalMs: 700,
      moveIfRunning: true,
      forceResize: true,
      applyToAllMatchingWindows: false,
      restoreIfMinimized: true,
      pullHiddenWindows: true,
      wakeRunningProcess: true,
      allowEmptyTitle: false,
      notes: "",
    };
    onConfigChange(patchProfile(config, profile.id, (profile) => ({ ...profile, apps: [...profile.apps, next] })));
    onSelectedAppChange(id);
    setTab("general");
  }

  function addPreset(presetId: string) {
    const preset = presets.find((item) => item.id === presetId);
    if (!preset) return;
    const id = newId("app");
    const nextPreset: AppConfig = {
      ...preset,
      id,
      arguments: [...preset.arguments],
      layout: { ...preset.layout },
      titleRule: preset.titleRule ? { ...preset.titleRule } : null,
    };
    onConfigChange(patchProfile(config, profile.id, (profile) => ({ ...profile, apps: [...profile.apps, nextPreset] })));
    onSelectedAppChange(id);
    setTab("general");
  }

  return (
    <div className="grid gap-4 lg:grid-cols-[280px_minmax(0,1fr)]">
      <section className="panel p-3">
        <div className="flex items-center justify-between gap-2">
          <div>
            <h1 className="section-heading">Profile apps</h1>
            <div className="mt-1 text-xs text-[#71818c]">{profile.apps.length} configured</div>
          </div>
          <IconButton label="Add custom app" onClick={addCustomApp} variant="solid">
            <Plus size={16} />
          </IconButton>
        </div>
        <div className="mt-3">
          <Field label="Add preset">
            <SelectInput value="" onChange={(event) => addPreset(event.target.value)}>
              <option value="">Choose preset</option>
              {presets.map((preset) => (
                <option key={preset.id} value={preset.id}>
                  {preset.displayName}
                </option>
              ))}
            </SelectInput>
          </Field>
        </div>
        <div className="data-list mt-3">
          {profile.apps.map((item) => (
            <div
              key={item.id}
              className={`data-row flex min-h-[62px] items-center gap-2 py-2 pl-3 pr-2 ${
                item.id === app?.id ? "bg-[#16252e] text-[#e4f6fa]" : "text-zinc-300 hover:bg-[#121a20]"
              }`}
            >
              <button className="min-w-0 flex-1 text-left" onClick={() => onSelectedAppChange(item.id)}>
                <span className="block truncate text-sm font-medium">{item.displayName}</span>
                <span className="mt-1 block truncate text-xs text-[#71818c]">{item.processName ?? "Process unset"}</span>
              </button>
              <IconButton label={`Remove ${item.displayName} from profile`} onClick={() => onRemoveApp(item.id)} variant="danger">
                <Trash2 size={15} />
              </IconButton>
            </div>
          ))}
          {profile.apps.length === 0 && <div className="px-3 py-8 text-center text-sm text-[#71818c]">No apps in this profile</div>}
        </div>
      </section>

      {app ? (
        <section className="panel min-w-0 p-4">
          <div className="flex flex-wrap items-start justify-between gap-3 border-b border-[#27313a] pb-4">
            <div className="min-w-0">
              <div className="eyebrow">App settings</div>
              <h2 className="mt-1 truncate text-lg font-semibold text-zinc-50">{app.displayName}</h2>
              <div className="mt-1 truncate text-xs text-[#71818c]">{app.processName ?? "Process not set"}</div>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <div className="segmented" role="tablist" aria-label="App settings sections">
                {(["general", "matching", "behavior"] as EditorTab[]).map((item) => (
                  <button
                    key={item}
                    role="tab"
                    aria-selected={tab === item}
                    className={`segment ${tab === item ? "segment-active" : ""}`}
                    onClick={() => setTab(item)}
                  >
                    {statusText(item)}
                  </button>
                ))}
              </div>
              <IconButton label={`Remove ${app.displayName} from profile`} onClick={() => onRemoveApp(app.id)} variant="danger">
                <Trash2 size={16} />
              </IconButton>
            </div>
          </div>

          {tab === "general" && (
            <div className="mt-4 grid gap-4 lg:grid-cols-2">
              <Field label="Display name">
                <TextInput value={app.displayName} onChange={(event) => updateApp(app.id, (app) => ({ ...app, displayName: event.target.value }))} />
              </Field>
              <Field label="Process name">
                <TextInput value={app.processName ?? ""} onChange={(event) => updateApp(app.id, (app) => ({ ...app, processName: event.target.value || null }))} />
              </Field>
              <Field label="Executable path">
                <TextInput value={app.executablePath ?? ""} onChange={(event) => updateApp(app.id, (app) => ({ ...app, executablePath: event.target.value || null }))} />
              </Field>
              <Field label="Working directory">
                <TextInput value={app.workingDirectory ?? ""} onChange={(event) => updateApp(app.id, (app) => ({ ...app, workingDirectory: event.target.value || null }))} />
              </Field>
              <Field label="Target monitor">
                <SelectInput
                  value={app.targetMonitorId ?? ""}
                  onChange={(event) => {
                    const monitorId = event.target.value || null;
                    const targetMonitor = monitors.find((monitor) => monitor.id === monitorId);
                    updateApp(app.id, (app) => ({
                      ...app,
                      targetMonitorId: monitorId,
                      capturedDisplay: targetMonitor ? capturedDisplayForMonitor(targetMonitor) : null,
                    }));
                  }}
                >
                  <option value="">Use profile target</option>
                  {missingTargetMonitor && <option value={missingTargetMonitor}>Missing: {missingTargetMonitor}</option>}
                  {monitors.map((monitor) => (
                    <option key={monitor.id} value={monitor.id}>
                      {monitor.name} - {monitor.width}x{monitor.height}
                    </option>
                  ))}
                </SelectInput>
              </Field>
              <Field label="Window state">
                <SelectInput value={app.windowState} onChange={(event) => updateApp(app.id, (app) => ({ ...app, windowState: event.target.value as WindowStatePreference }))}>
                  {windowStates.map((state) => (
                    <option key={state} value={state}>
                      {statusText(state)}
                    </option>
                  ))}
                </SelectInput>
              </Field>
              <Field label="Arguments">
                <TextArea value={formatArguments(app.arguments)} onChange={(event) => updateApp(app.id, (app) => ({ ...app, arguments: parseArguments(event.target.value) }))} />
              </Field>
              <Field label="Notes">
                <TextArea value={app.notes ?? ""} onChange={(event) => updateApp(app.id, (app) => ({ ...app, notes: event.target.value }))} />
              </Field>
            </div>
          )}

          {tab === "matching" && (
            <>
              <div className="mt-4 grid gap-4 lg:grid-cols-2">
                <Field label="Process name">
                  <TextInput value={app.processName ?? ""} onChange={(event) => updateApp(app.id, (app) => ({ ...app, processName: event.target.value || null }))} />
                </Field>
                <Field label="Class name">
                  <TextInput value={app.className ?? ""} onChange={(event) => updateApp(app.id, (app) => ({ ...app, className: event.target.value || null }))} />
                </Field>
                <Field label="Title match mode">
                  <SelectInput
                    value={app.titleRule?.mode ?? ""}
                    onChange={(event) =>
                      updateApp(app.id, (app) => ({
                        ...app,
                        titleRule: event.target.value
                          ? { mode: event.target.value as TitleMatchMode, value: app.titleRule?.value ?? "", caseSensitive: app.titleRule?.caseSensitive ?? false }
                          : null,
                      }))
                    }
                  >
                    <option value="">No title rule</option>
                    {titleModes.map((mode) => (
                      <option key={mode} value={mode}>
                        {statusText(mode)}
                      </option>
                    ))}
                  </SelectInput>
                </Field>
                <Field label="Title match value">
                  <TextInput
                    value={app.titleRule?.value ?? ""}
                    disabled={!app.titleRule}
                    onChange={(event) => updateApp(app.id, (app) => ({ ...app, titleRule: app.titleRule ? { ...app.titleRule, value: event.target.value } : null }))}
                  />
                </Field>
              </div>
              <div className="mt-4 grid gap-2 md:grid-cols-2">
                <Toggle label="Case-sensitive title match" checked={app.titleRule?.caseSensitive ?? false} onChange={(checked) => updateApp(app.id, (app) => ({ ...app, titleRule: app.titleRule ? { ...app.titleRule, caseSensitive: checked } : null }))} />
                <Toggle label="Allow empty window titles" checked={app.allowEmptyTitle} onChange={(checked) => updateApp(app.id, (app) => ({ ...app, allowEmptyTitle: checked }))} />
              </div>
            </>
          )}

          {tab === "behavior" && (
            <>
              <div className="mt-4 grid gap-4 md:grid-cols-3">
                <Field label="Detection timeout">
                  <NumberInput value={app.detectionTimeoutSeconds} min={1} max={120} onChange={(event) => updateApp(app.id, (app) => ({ ...app, detectionTimeoutSeconds: Number(event.target.value) }))} />
                </Field>
                <Field label="Retry interval ms">
                  <NumberInput value={app.retryIntervalMs} min={250} max={5000} step={50} onChange={(event) => updateApp(app.id, (app) => ({ ...app, retryIntervalMs: Number(event.target.value) }))} />
                </Field>
                <Field label="Launch delay seconds">
                  <NumberInput value={app.launchDelaySeconds} min={0} max={120} onChange={(event) => updateApp(app.id, (app) => ({ ...app, launchDelaySeconds: Number(event.target.value) }))} />
                </Field>
              </div>
              <div className="mt-4 grid gap-2 md:grid-cols-2 xl:grid-cols-3">
                <Toggle label="Move if already running" checked={app.moveIfRunning} onChange={(checked) => updateApp(app.id, (app) => ({ ...app, moveIfRunning: checked }))} />
                <Toggle label="Force saved size" checked={app.forceResize} onChange={(checked) => updateApp(app.id, (app) => ({ ...app, forceResize: checked }))} />
                <Toggle label="Move all matching windows" checked={app.applyToAllMatchingWindows} onChange={(checked) => updateApp(app.id, (app) => ({ ...app, applyToAllMatchingWindows: checked }))} />
                <Toggle label="Restore minimized windows" checked={app.restoreIfMinimized} onChange={(checked) => updateApp(app.id, (app) => ({ ...app, restoreIfMinimized: checked }))} />
                <Toggle label="Pull hidden or tray windows" checked={app.pullHiddenWindows} onChange={(checked) => updateApp(app.id, (app) => ({ ...app, pullHiddenWindows: checked }))} />
                <Toggle label="Wake running tray apps" checked={app.wakeRunningProcess} onChange={(checked) => updateApp(app.id, (app) => ({ ...app, wakeRunningProcess: checked }))} />
              </div>
            </>
          )}
        </section>
      ) : (
        <section className="panel p-8 text-center text-sm text-[#71818c]">No apps in this profile</section>
      )}
    </div>
  );
}
