export type MonitorMissingBehavior =
  | "doNothing"
  | "usePrimary"
  | "nearestMatch"
  | "askNextOpen";

export type WindowStatePreference = "normal" | "maximized" | "minimized";
export type TitleMatchMode = "contains" | "exact" | "startsWith" | "endsWith" | "regex";
export type RestoreStatus = "success" | "partialSuccess" | "failed" | "monitorMissing";
export type AppRestoreStatus =
  | "success"
  | "skipped"
  | "launched"
  | "launchedWindowNotFound"
  | "processRunningWindowNotFound"
  | "monitorMissing"
  | "invalidExecutablePath"
  | "permissionDenied"
  | "moveFailed"
  | "failed";
export type LogSeverity = "info" | "warn" | "error";

export interface LayoutRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface MatchRule {
  mode: TitleMatchMode;
  value: string;
  caseSensitive: boolean;
}

export interface AppConfig {
  id: string;
  displayName: string;
  executablePath?: string | null;
  arguments: string[];
  workingDirectory?: string | null;
  processName?: string | null;
  titleRule?: MatchRule | null;
  className?: string | null;
  targetMonitorId?: string | null;
  layout: LayoutRect;
  windowState: WindowStatePreference;
  launchDelaySeconds: number;
  detectionTimeoutSeconds: number;
  retryIntervalMs: number;
  launchIfMissing: boolean;
  moveIfRunning: boolean;
  forceResize: boolean;
  applyToAllMatchingWindows: boolean;
  restoreIfMinimized: boolean;
  pullHiddenWindows: boolean;
  wakeRunningProcess: boolean;
  allowEmptyTitle: boolean;
  notes?: string | null;
}

export interface Profile {
  id: string;
  name: string;
  description?: string | null;
  targetMonitorId?: string | null;
  apps: AppConfig[];
  startupRestore: boolean;
  enforceAfterRestore: boolean;
}

export interface GlobalSettings {
  defaultMonitorId?: string | null;
  monitorMissingBehavior: MonitorMissingBehavior;
  warnWhenMonitorMissing: boolean;
  advancedMode: boolean;
}

export interface StartupSettings {
  enabled: boolean;
  startMinimizedToTray: boolean;
  defaultProfileId?: string | null;
  delaySeconds: number;
  restoreOnLaunch: boolean;
  launchMissingApps: boolean;
  enforceAfterStartup: boolean;
}

export interface TraySettings {
  minimizeToTrayOnClose: boolean;
  showRestoreStatus: boolean;
}

export interface HotkeySettings {
  enabled: boolean;
  accelerator: string;
  restoreWithoutOpening: boolean;
}

export interface EnforcementSettings {
  enabled: boolean;
  profileId?: string | null;
  durationSeconds: number;
  intervalMs: number;
  pauseForFullscreenGames: boolean;
}

export interface WindowAutoLayoutConfig {
  schemaVersion: number;
  appVersion: string;
  global: GlobalSettings;
  startup: StartupSettings;
  tray: TraySettings;
  hotkey: HotkeySettings;
  enforcement: EnforcementSettings;
  profiles: Profile[];
}

export interface MonitorInfo {
  id: string;
  name: string;
  deviceName: string;
  x: number;
  y: number;
  width: number;
  height: number;
  workX: number;
  workY: number;
  workWidth: number;
  workHeight: number;
  scaleFactor: number;
  isPrimary: boolean;
}

export interface WindowInfo {
  handle: string;
  title: string;
  className: string;
  processId: number;
  processName: string;
  executablePath?: string | null;
  monitorId?: string | null;
  x: number;
  y: number;
  width: number;
  height: number;
  isVisible: boolean;
  isMinimized: boolean;
}

export interface AppRestoreResult {
  appId: string;
  displayName: string;
  status: AppRestoreStatus;
  message: string;
  matchedWindows: WindowInfo[];
}

export interface RestoreResult {
  profileId: string;
  profileName: string;
  status: RestoreStatus;
  startedAt: string;
  finishedAt: string;
  monitor?: MonitorInfo | null;
  results: AppRestoreResult[];
}

export interface CapturedWindowSummary {
  appId: string;
  displayName: string;
  processName: string;
  title: string;
}

export interface CaptureLayoutResult {
  config: WindowAutoLayoutConfig;
  profileId: string;
  monitor: MonitorInfo;
  capturedCount: number;
  skippedCount: number;
  capturedWindows: CapturedWindowSummary[];
}

export interface LogEntry {
  timestamp: string;
  severity: LogSeverity;
  profile?: string | null;
  app?: string | null;
  message: string;
}
