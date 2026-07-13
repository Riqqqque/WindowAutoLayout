import { AlignHorizontalSpaceAround, Columns2, Maximize2, Rows2, Trash2 } from "lucide-react";
import { Field, NumberInput, Toggle } from "../components/Form";
import { IconButton } from "../components/IconButton";
import { MonitorPreview } from "../components/MonitorPreview";
import { WindowPicker } from "../components/WindowPicker";
import { clampRect, patchApp, patchProfile, resolveMonitor, resolveProfileMonitor } from "../lib/helpers";
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
  onRemoveApp: (appId: string) => void;
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
  onRemoveApp,
}: LayoutEditorProps) {
  const app = profile.apps.find((item) => item.id === selectedAppId) ?? profile.apps[0];
  const monitor =
    (app?.targetMonitorId
      ? resolveMonitor(
          monitors,
          app.targetMonitorId,
          config.global.monitorMissingBehavior,
          app.layout,
        ).monitor
      : resolveProfileMonitor(config, profile, monitors).monitor) ??
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
        targetMonitorId: monitor.id,
        apps: profile.apps.map((app, index) => ({
          ...app,
          targetMonitorId: monitor.id,
          layout: {
            x: index * width,
            y: 0,
            width: index === profile.apps.length - 1 ? monitor.width - index * width : width,
            height: monitor.height,
          },
        })),
      })),
    );
  }

  return (
    <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_350px]">
      <section className="panel min-w-0 p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h1 className="section-heading">Layout canvas</h1>
            <div className="mt-1 text-xs text-[#71818c]">{monitor ? `${monitor.name} / ${monitor.width} x ${monitor.height}` : "No display"}</div>
          </div>
          <div className="flex gap-2">
            <IconButton label="Fill left half" disabled={!app || !monitor} onClick={() => app && monitor && setSelectedRect({ x: 0, y: 0, width: monitor.width / 2, height: monitor.height })}>
              <Columns2 size={16} />
            </IconButton>
            <IconButton label="Fill top half" disabled={!app || !monitor} onClick={() => app && monitor && setSelectedRect({ x: 0, y: 0, width: monitor.width, height: monitor.height / 2 })}>
              <Rows2 size={16} />
            </IconButton>
            <IconButton label="Fill monitor bounds" disabled={!app || !monitor} onClick={() => app && monitor && setSelectedRect({ x: 0, y: 0, width: monitor.width, height: monitor.height })}>
              <Maximize2 size={16} />
            </IconButton>
            <IconButton label="Split profile apps evenly on this display" onClick={splitEvenly} disabled={!monitor || profile.apps.length === 0}>
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
        <section className="panel p-4">
          <div className="flex items-center justify-between gap-3">
            <div className="min-w-0">
              <div className="eyebrow">Selected app</div>
              <h2 className="mt-1 truncate text-base font-semibold text-zinc-100">{app?.displayName ?? "None"}</h2>
            </div>
            <div className="flex items-center gap-2">
              {app && (
                <IconButton label={`Remove ${app.displayName} from profile`} onClick={() => onRemoveApp(app.id)} variant="danger">
                  <Trash2 size={15} />
                </IconButton>
              )}
              <Toggle label="Grid" checked={showGrid} onChange={onShowGridChange} />
            </div>
          </div>
          {app ? (
            <div className="mt-4 grid grid-cols-2 gap-3">
              <RectInput label="X" value={app.layout.x} onChange={(value) => setSelectedRect({ ...app.layout, x: value })} />
              <RectInput label="Y" value={app.layout.y} onChange={(value) => setSelectedRect({ ...app.layout, y: value })} />
              <RectInput label="Width" value={app.layout.width} onChange={(value) => setSelectedRect({ ...app.layout, width: value })} />
              <RectInput label="Height" value={app.layout.height} onChange={(value) => setSelectedRect({ ...app.layout, height: value })} />
            </div>
          ) : (
            <div className="mt-4 text-sm text-[#71818c]">No app selected</div>
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
