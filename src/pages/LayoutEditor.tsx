import { AlignHorizontalSpaceAround, Columns2, Maximize2, Rows2 } from "lucide-react";
import { Field, NumberInput, Toggle } from "../components/Form";
import { IconButton } from "../components/IconButton";
import { MonitorPreview } from "../components/MonitorPreview";
import { WindowPicker } from "../components/WindowPicker";
import { clampRect, patchApp, patchProfile } from "../lib/helpers";
import type { LayoutRect, MonitorInfo, Profile, WindowAutoLayoutConfig, WindowInfo } from "../lib/types";

interface LayoutEditorProps {
  config: WindowAutoLayoutConfig;
  profile: Profile;
  monitors: MonitorInfo[];
  windows: WindowInfo[];
  selectedAppId?: string | null;
  selectedWindowHandle?: string | null;
  showGrid: boolean;
  onConfigChange: (config: WindowAutoLayoutConfig) => void;
  onSelectedAppChange: (appId: string) => void;
  onSelectedWindowChange: (handle: string) => void;
  onRefreshWindows: () => void;
  onSaveSelectedWindow: () => void;
  onShowGridChange: (value: boolean) => void;
}

export function LayoutEditorPage({
  config,
  profile,
  monitors,
  windows,
  selectedAppId,
  selectedWindowHandle,
  showGrid,
  onConfigChange,
  onSelectedAppChange,
  onSelectedWindowChange,
  onRefreshWindows,
  onSaveSelectedWindow,
  onShowGridChange,
}: LayoutEditorProps) {
  const app = profile.apps.find((item) => item.id === selectedAppId) ?? profile.apps[0];
  const monitor =
    monitors.find((item) => item.id === (app?.targetMonitorId ?? profile.targetMonitorId ?? config.global.defaultMonitorId)) ??
    monitors.find((item) => !item.isPrimary) ??
    monitors[0];

  function updateRect(appId: string, rect: LayoutRect) {
    onConfigChange(patchApp(config, profile.id, appId, (app) => ({ ...app, layout: clampRect(rect, monitor) })));
  }

  function setSelectedRect(rect: LayoutRect) {
    if (!app) return;
    updateRect(app.id, rect);
  }

  function splitEvenly() {
    if (!monitor || profile.apps.length === 0) return;
    const width = Math.floor(monitor.width / profile.apps.length);
    onConfigChange(
      patchProfile(config, profile.id, (profile) => ({
        ...profile,
        apps: profile.apps.map((app, index) => ({
          ...app,
          layout: { x: index * width, y: 0, width, height: monitor.height },
        })),
      })),
    );
  }

  return (
    <div className="grid gap-4 xl:grid-cols-[1fr_360px]">
      <section className="surface rounded-md p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h1 className="text-lg font-semibold text-zinc-50">Layout</h1>
          <div className="flex gap-2">
            <IconButton label="Fill left half" onClick={() => app && setSelectedRect({ x: 0, y: 0, width: (monitor?.width ?? 1920) / 2, height: monitor?.height ?? 1080 })}>
              <Columns2 size={16} />
            </IconButton>
            <IconButton label="Fill top half" onClick={() => app && setSelectedRect({ x: 0, y: 0, width: monitor?.width ?? 1920, height: (monitor?.height ?? 1080) / 2 })}>
              <Rows2 size={16} />
            </IconButton>
            <IconButton label="Maximize in monitor bounds" onClick={() => app && setSelectedRect({ x: 0, y: 0, width: monitor?.width ?? 1920, height: monitor?.height ?? 1080 })}>
              <Maximize2 size={16} />
            </IconButton>
            <IconButton label="Split apps evenly" onClick={splitEvenly}>
              <AlignHorizontalSpaceAround size={16} />
            </IconButton>
          </div>
        </div>

        <div className="mt-4">
          <MonitorPreview
            monitor={monitor}
            apps={profile.apps}
            selectedAppId={app?.id}
            showGrid={showGrid}
            onSelect={onSelectedAppChange}
            onChange={updateRect}
          />
        </div>
      </section>

      <aside className="grid gap-4">
        <section className="surface rounded-md p-4">
          <div className="flex items-center justify-between gap-3">
            <h2 className="text-sm font-semibold uppercase tracking-normal text-[#8a94a3]">Selected app</h2>
            <Toggle label="Grid" checked={showGrid} onChange={onShowGridChange} />
          </div>
          {app ? (
            <div className="mt-4 grid grid-cols-2 gap-3">
              <RectInput label="X" value={app.layout.x} onChange={(value) => setSelectedRect({ ...app.layout, x: value })} />
              <RectInput label="Y" value={app.layout.y} onChange={(value) => setSelectedRect({ ...app.layout, y: value })} />
              <RectInput label="Width" value={app.layout.width} onChange={(value) => setSelectedRect({ ...app.layout, width: value })} />
              <RectInput label="Height" value={app.layout.height} onChange={(value) => setSelectedRect({ ...app.layout, height: value })} />
            </div>
          ) : (
            <div className="mt-4 text-sm text-zinc-500">No app selected</div>
          )}
        </section>

        <WindowPicker
          windows={windows}
          selectedHandle={selectedWindowHandle}
          onRefresh={onRefreshWindows}
          onSelect={onSelectedWindowChange}
          onSave={onSaveSelectedWindow}
        />
      </aside>
    </div>
  );
}

function RectInput({ label, value, onChange }: { label: string; value: number; onChange: (value: number) => void }) {
  return (
    <Field label={label}>
      <NumberInput value={Math.round(value)} onChange={(event) => onChange(Number(event.target.value))} />
    </Field>
  );
}
