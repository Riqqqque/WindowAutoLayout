import type { AppConfig, LayoutRect, MonitorInfo, Profile, WindowAutoLayoutConfig } from "./types";

export function newId(prefix: string) {
  return `${prefix}-${crypto.randomUUID()}`;
}

export function activeProfile(config: WindowAutoLayoutConfig, profileId?: string | null) {
  return (
    config.profiles.find((profile) => profile.id === profileId) ??
    config.profiles.find((profile) => profile.id === config.startup.defaultProfileId) ??
    config.profiles[0]
  );
}

export function patchProfile(
  config: WindowAutoLayoutConfig,
  profileId: string,
  update: (profile: Profile) => Profile,
) {
  return {
    ...config,
    profiles: config.profiles.map((profile) => (profile.id === profileId ? update(profile) : profile)),
  };
}

export function patchApp(
  config: WindowAutoLayoutConfig,
  profileId: string,
  appId: string,
  update: (app: AppConfig) => AppConfig,
) {
  return patchProfile(config, profileId, (profile) => ({
    ...profile,
    apps: profile.apps.map((app) => (app.id === appId ? update(app) : app)),
  }));
}

export function monitorLabel(monitor?: MonitorInfo | null) {
  if (!monitor) return "No monitor";
  return `${monitor.name} ${monitor.width}x${monitor.height}${monitor.isPrimary ? " primary" : ""}`;
}

export function resolveMonitor(
  monitors: MonitorInfo[],
  preferredId?: string | null,
  fallbackMode: WindowAutoLayoutConfig["global"]["monitorMissingBehavior"] = "nearestMatch",
  targetSize?: { width: number; height: number },
) {
  if (preferredId) {
    const exact = monitors.find((monitor) => monitor.id === preferredId);
    if (exact) return { monitor: exact, isFallback: false };
    if (fallbackMode === "doNothing") return { monitor: null, isFallback: false };
  }

  const fallback =
    fallbackMode === "usePrimary"
      ? monitors.find((monitor) => monitor.isPrimary)
      : targetSize
        ? [...monitors].sort((left, right) => {
            const leftScore = Math.abs(left.width - targetSize.width) + Math.abs(left.height - targetSize.height);
            const rightScore = Math.abs(right.width - targetSize.width) + Math.abs(right.height - targetSize.height);
            return leftScore - rightScore || Number(left.isPrimary) - Number(right.isPrimary);
          })[0]
        : monitors.find((monitor) => monitor.isPrimary) ?? monitors[0];
  return { monitor: fallback ?? null, isFallback: Boolean(preferredId && fallback) };
}

export function resolveProfileMonitor(config: WindowAutoLayoutConfig, profile: Profile, monitors: MonitorInfo[]) {
  const bounds = profile.apps.reduce(
    (current, app) => ({
      left: Math.min(current.left, app.layout.x),
      top: Math.min(current.top, app.layout.y),
      right: Math.max(current.right, app.layout.x + app.layout.width),
      bottom: Math.max(current.bottom, app.layout.y + app.layout.height),
    }),
    { left: Infinity, top: Infinity, right: -Infinity, bottom: -Infinity },
  );
  const targetSize = profile.apps.length
    ? { width: bounds.right - bounds.left, height: bounds.bottom - bounds.top }
    : undefined;
  return resolveMonitor(
    monitors,
    profile.targetMonitorId ?? config.global.defaultMonitorId,
    config.global.monitorMissingBehavior,
    targetSize,
  );
}

export function clampRect(rect: LayoutRect, monitor?: MonitorInfo | null): LayoutRect {
  const maxWidth = Math.max(160, monitor?.width ?? 3840);
  const maxHeight = Math.max(120, monitor?.height ?? 2160);
  const width = Math.min(Math.max(120, Math.round(rect.width)), maxWidth);
  const height = Math.min(Math.max(90, Math.round(rect.height)), maxHeight);
  return {
    x: Math.min(Math.max(0, Math.round(rect.x)), Math.max(0, maxWidth - width)),
    y: Math.min(Math.max(0, Math.round(rect.y)), Math.max(0, maxHeight - height)),
    width,
    height,
  };
}

export function statusText(value: string) {
  return value
    .replace(/([A-Z])/g, " $1")
    .replace(/^./, (char) => char.toUpperCase())
    .trim();
}

export function parseArguments(text: string) {
  return text
    .split(/\r?\n/)
    .map((value) => value.trim())
    .filter(Boolean);
}

export function formatArguments(args: string[]) {
  return args.join("\n");
}
