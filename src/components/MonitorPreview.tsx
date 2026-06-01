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
        "relative aspect-video min-h-[280px] overflow-hidden rounded-md border border-[#34404d] bg-[#0d1117]",
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
      <div className="absolute left-3 top-3 rounded-md border border-[#34404d] bg-[#0b0d10]/85 px-2 py-1 text-xs text-zinc-300">
        {monitor ? `${monitor.width} x ${monitor.height}` : "Preview"}
      </div>

      {apps.map((app, index) => {
        const left = (app.layout.x / canvas.width) * 100;
        const top = (app.layout.y / canvas.height) * 100;
        const width = (app.layout.width / canvas.width) * 100;
        const height = (app.layout.height / canvas.height) * 100;
        const selected = selectedAppId === app.id;
        const compact = width < 14 || height < 12;
        const showGeometry = selected || !compact;

        return (
          <div
            key={app.id}
            className={clsx(
              "absolute min-h-8 min-w-12 cursor-move overflow-hidden rounded-md border text-xs shadow-lg transition",
              selected
                ? "border-[#5db7ff] bg-[#182a3a] text-[#d7edff]"
                : "border-[#566273] bg-[#18202a]/90 text-zinc-200",
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
            title={`${app.displayName}: ${app.layout.x}, ${app.layout.y} - ${app.layout.width} x ${app.layout.height}`}
          >
            <div className="pointer-events-none absolute left-1 top-1 max-w-[calc(100%-0.5rem)] rounded border border-[#0b0d10]/50 bg-[#0b0d10]/80 px-1.5 py-1 shadow-sm">
              <div className="truncate text-[11px] font-semibold leading-4">{app.displayName}</div>
              {showGeometry && (
                <div className="mt-0.5 truncate text-[10px] leading-3 text-zinc-300">
                  {app.layout.x}, {app.layout.y} - {app.layout.width} x {app.layout.height}
                </div>
              )}
            </div>
            <button
              aria-label={`Resize ${app.displayName}`}
              title={`Resize ${app.displayName}`}
              className="absolute bottom-1 right-1 h-4 w-4 rounded-sm border border-[#5db7ff]/80 bg-[#5db7ff]/20"
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
