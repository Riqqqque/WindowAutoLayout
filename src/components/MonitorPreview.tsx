import { useMemo, useRef, useState } from "react";
import { clsx } from "clsx";
import type { AppConfig, LayoutRect, MonitorInfo } from "../lib/types";
import { clampRect } from "../lib/helpers";

type DragState = {
  appId: string;
  mode: "move" | "resize";
  offsetX: number;
  offsetY: number;
};

interface MonitorPreviewProps {
  monitor?: MonitorInfo | null;
  apps: AppConfig[];
  selectedAppId?: string | null;
  showGrid?: boolean;
  onSelect?: (appId: string) => void;
  onChange?: (appId: string, rect: LayoutRect) => void;
}

export function MonitorPreview({
  monitor,
  apps,
  selectedAppId,
  showGrid,
  onSelect,
  onChange,
}: MonitorPreviewProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [drag, setDrag] = useState<DragState | null>(null);
  const canvas = useMemo(
    () => ({
      width: Math.max(1, monitor?.width ?? 1920),
      height: Math.max(1, monitor?.height ?? 1080),
    }),
    [monitor],
  );

  function pointerToLayout(event: React.PointerEvent, current: LayoutRect) {
    const bounds = ref.current?.getBoundingClientRect();
    if (!bounds || !drag) return current;
    const scaleX = canvas.width / bounds.width;
    const scaleY = canvas.height / bounds.height;
    const x = (event.clientX - bounds.left) * scaleX;
    const y = (event.clientY - bounds.top) * scaleY;

    if (drag.mode === "resize") {
      return clampRect(
        {
          ...current,
          width: x - current.x,
          height: y - current.y,
        },
        monitor,
      );
    }

    return clampRect(
      {
        ...current,
        x: x - drag.offsetX,
        y: y - drag.offsetY,
      },
      monitor,
    );
  }

  return (
    <div
      ref={ref}
      className={clsx(
        "relative aspect-video min-h-[280px] overflow-hidden rounded-lg border border-zinc-700 bg-zinc-950",
        showGrid && "monitor-grid",
      )}
      onPointerMove={(event) => {
        if (!drag) return;
        const app = apps.find((item) => item.id === drag.appId);
        if (!app) return;
        onChange?.(app.id, pointerToLayout(event, app.layout));
      }}
      onPointerUp={() => setDrag(null)}
      onPointerLeave={() => setDrag(null)}
    >
      <div className="absolute left-3 top-3 rounded-md border border-zinc-700 bg-zinc-950/80 px-2 py-1 text-xs text-zinc-300">
        {monitor ? `${monitor.width} x ${monitor.height}` : "Preview"}
      </div>

      {apps.map((app, index) => {
        const left = (app.layout.x / canvas.width) * 100;
        const top = (app.layout.y / canvas.height) * 100;
        const width = (app.layout.width / canvas.width) * 100;
        const height = (app.layout.height / canvas.height) * 100;
        const selected = selectedAppId === app.id;

        return (
          <div
            key={app.id}
            className={clsx(
              "absolute min-h-8 min-w-12 cursor-move rounded-md border p-2 text-xs shadow-lg transition",
              selected
                ? "border-cyan-300 bg-cyan-300/20 text-cyan-50"
                : "border-zinc-500 bg-zinc-800/80 text-zinc-200",
            )}
            style={{
              left: `${left}%`,
              top: `${top}%`,
              width: `${Math.max(width, 5)}%`,
              height: `${Math.max(height, 5)}%`,
              zIndex: selected ? 20 : 10 + index,
            }}
            onPointerDown={(event) => {
              event.preventDefault();
              onSelect?.(app.id);
              const bounds = ref.current?.getBoundingClientRect();
              if (!bounds) return;
              const scaleX = canvas.width / bounds.width;
              const scaleY = canvas.height / bounds.height;
              setDrag({
                appId: app.id,
                mode: "move",
                offsetX: (event.clientX - bounds.left) * scaleX - app.layout.x,
                offsetY: (event.clientY - bounds.top) * scaleY - app.layout.y,
              });
            }}
          >
            <div className="truncate font-medium">{app.displayName}</div>
            <div className="mt-1 text-[11px] text-zinc-300">
              {app.layout.x}, {app.layout.y} · {app.layout.width} x {app.layout.height}
            </div>
            <button
              aria-label={`Resize ${app.displayName}`}
              title={`Resize ${app.displayName}`}
              className="absolute bottom-1 right-1 h-4 w-4 rounded-sm border border-cyan-200/70 bg-cyan-200/20"
              onPointerDown={(event) => {
                event.preventDefault();
                event.stopPropagation();
                onSelect?.(app.id);
                setDrag({ appId: app.id, mode: "resize", offsetX: 0, offsetY: 0 });
              }}
            />
          </div>
        );
      })}
    </div>
  );
}
