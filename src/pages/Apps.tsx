import { Plus, Trash2 } from "lucide-react";
import { Field, NumberInput, SelectInput, TextArea, TextInput, Toggle } from "../components/Form";
import { IconButton } from "../components/IconButton";
import { formatArguments, newId, parseArguments, patchApp, patchProfile } from "../lib/helpers";
import type { AppConfig, MonitorInfo, Profile, TitleMatchMode, WindowAutoLayoutConfig, WindowStatePreference } from "../lib/types";

interface AppsProps {
  config: WindowAutoLayoutConfig;
  profile: Profile;
  monitors: MonitorInfo[];
  presets: AppConfig[];
  selectedAppId?: string | null;
  onSelectedAppChange: (appId: string) => void;
  onConfigChange: (config: WindowAutoLayoutConfig) => void;
}

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
}: AppsProps) {
  const app = profile.apps.find((item) => item.id === selectedAppId) ?? profile.apps[0];

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
      layout: { x: 80, y: 80, width: 960, height: 640 },
      windowState: "normal",
      launchDelaySeconds: 0,
      detectionTimeoutSeconds: 25,
      retryIntervalMs: 700,
      launchIfMissing: true,
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
  }

  function addPreset(presetId: string) {
    const preset = presets.find((item) => item.id === presetId);
    if (!preset) return;
    const id = newId("app");
    const next = { ...preset, id };
    onConfigChange(patchProfile(config, profile.id, (profile) => ({ ...profile, apps: [...profile.apps, next] })));
    onSelectedAppChange(id);
  }

  function removeApp(appId: string) {
    const remaining = profile.apps.filter((item) => item.id !== appId);
    onConfigChange(patchProfile(config, profile.id, (profile) => ({ ...profile, apps: remaining })));
    if (remaining[0]) onSelectedAppChange(remaining[0].id);
  }

  return (
    <div className="grid gap-4 xl:grid-cols-[320px_1fr]">
      <section className="rounded-lg border border-zinc-800 bg-zinc-900/70 p-3">
        <div className="flex items-center justify-between gap-2">
          <h1 className="text-lg font-semibold text-zinc-50">Apps</h1>
          <IconButton label="Add custom app" onClick={addCustomApp} variant="solid">
            <Plus size={16} />
          </IconButton>
        </div>
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
        <div className="mt-3 grid gap-2">
          {profile.apps.map((item) => (
            <button
              key={item.id}
              className={`rounded-md border px-3 py-2 text-left transition ${
                item.id === app?.id
                  ? "border-cyan-300/50 bg-cyan-300/10 text-cyan-100"
                  : "border-zinc-800 bg-zinc-950 text-zinc-300 hover:bg-zinc-900"
              }`}
              onClick={() => onSelectedAppChange(item.id)}
            >
              <span className="block truncate text-sm font-medium">{item.displayName}</span>
              <span className="mt-1 block truncate text-xs text-zinc-500">{item.processName ?? "Process unset"}</span>
            </button>
          ))}
        </div>
      </section>

      {app ? (
        <section className="rounded-lg border border-zinc-800 bg-zinc-900/70 p-4">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h2 className="text-lg font-semibold text-zinc-50">{app.displayName}</h2>
            <IconButton label="Remove app" onClick={() => removeApp(app.id)} variant="danger">
              <Trash2 size={16} />
            </IconButton>
          </div>

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
            <Field label="Arguments">
              <TextArea value={formatArguments(app.arguments)} onChange={(event) => updateApp(app.id, (app) => ({ ...app, arguments: parseArguments(event.target.value) }))} />
            </Field>
            <Field label="Target monitor">
              <SelectInput value={app.targetMonitorId ?? ""} onChange={(event) => updateApp(app.id, (app) => ({ ...app, targetMonitorId: event.target.value || null }))}>
                <option value="">Use profile target</option>
                {monitors.map((monitor) => (
                  <option key={monitor.id} value={monitor.id}>
                    {monitor.name} · {monitor.width}x{monitor.height}
                  </option>
                ))}
              </SelectInput>
            </Field>
            <Field label="Window state">
              <SelectInput value={app.windowState} onChange={(event) => updateApp(app.id, (app) => ({ ...app, windowState: event.target.value as WindowStatePreference }))}>
                {windowStates.map((state) => (
                  <option key={state} value={state}>
                    {state}
                  </option>
                ))}
              </SelectInput>
            </Field>
            <Field label="Class name match">
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
                    {mode}
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
            <Field label="Detection timeout">
              <NumberInput value={app.detectionTimeoutSeconds} min={1} onChange={(event) => updateApp(app.id, (app) => ({ ...app, detectionTimeoutSeconds: Number(event.target.value) }))} />
            </Field>
            <Field label="Retry interval ms">
              <NumberInput value={app.retryIntervalMs} min={100} step={50} onChange={(event) => updateApp(app.id, (app) => ({ ...app, retryIntervalMs: Number(event.target.value) }))} />
            </Field>
            <Field label="Launch delay seconds">
              <NumberInput value={app.launchDelaySeconds} min={0} onChange={(event) => updateApp(app.id, (app) => ({ ...app, launchDelaySeconds: Number(event.target.value) }))} />
            </Field>
            <Field label="Notes">
              <TextArea value={app.notes ?? ""} onChange={(event) => updateApp(app.id, (app) => ({ ...app, notes: event.target.value }))} />
            </Field>
          </div>

          <div className="mt-4 grid gap-2 md:grid-cols-3">
            <Toggle label="Launch if missing" checked={app.launchIfMissing} onChange={(checked) => updateApp(app.id, (app) => ({ ...app, launchIfMissing: checked }))} />
            <Toggle label="Move if already running" checked={app.moveIfRunning} onChange={(checked) => updateApp(app.id, (app) => ({ ...app, moveIfRunning: checked }))} />
            <Toggle label="Force resize" checked={app.forceResize} onChange={(checked) => updateApp(app.id, (app) => ({ ...app, forceResize: checked }))} />
            <Toggle label="Move all matching windows" checked={app.applyToAllMatchingWindows} onChange={(checked) => updateApp(app.id, (app) => ({ ...app, applyToAllMatchingWindows: checked }))} />
            <Toggle label="Restore minimized windows" checked={app.restoreIfMinimized} onChange={(checked) => updateApp(app.id, (app) => ({ ...app, restoreIfMinimized: checked }))} />
            <Toggle label="Pull hidden/tray windows" checked={app.pullHiddenWindows} onChange={(checked) => updateApp(app.id, (app) => ({ ...app, pullHiddenWindows: checked }))} />
            <Toggle label="Wake running tray apps" checked={app.wakeRunningProcess} onChange={(checked) => updateApp(app.id, (app) => ({ ...app, wakeRunningProcess: checked }))} />
            <Toggle label="Allow empty titles" checked={app.allowEmptyTitle} onChange={(checked) => updateApp(app.id, (app) => ({ ...app, allowEmptyTitle: checked }))} />
            <Toggle
              label="Case-sensitive title match"
              checked={app.titleRule?.caseSensitive ?? false}
              onChange={(checked) => updateApp(app.id, (app) => ({ ...app, titleRule: app.titleRule ? { ...app.titleRule, caseSensitive: checked } : null }))}
            />
          </div>
        </section>
      ) : (
        <section className="rounded-lg border border-zinc-800 bg-zinc-900/70 p-8 text-center text-sm text-zinc-500">No apps in this profile</section>
      )}
    </div>
  );
}
