export type MonitorMissingBehavior =
  | "doNothing"
  | "usePrimary"
  | "nearestMatch";

export type WindowStatePreference = "normal" | "maximized" | "minimized";
export type TitleMatchMode = "contains" | "exact" | "startsWith" | "endsWith" | "regex";
export type RestoreStatus = "success" | "partialSuccess" | "paused" | "failed" | "monitorMissing";
export type AppRestoreStatus =
  | "success"
  | "skipped"
  | "paused"
  | "launched"
  | "launchedWindowNotFound"
  | "processRunningWindowNotFound"
  | "monitorMissing"
  | "invalidExecutablePath"
  | "permissionDenied"
  | "moveFailed"
  | "failed";
export type LogSeverity = "info" | "warn" | "error";
export type TrayClickAction = "openWindow" | "restoreLayout";

export interface LayoutRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface CapturedDisplay {
  width: number;
  height: number;
  workX: number;
  workY: number;
  workWidth: number;
  workHeight: number;
  scalePercent: number;
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
  capturedDisplay?: CapturedDisplay | null;
  layout: LayoutRect;
  windowState: WindowStatePreference;
  launchDelaySeconds: number;
  detectionTimeoutSeconds: number;
  retryIntervalMs: number;
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
}

export interface GlobalSettings {
  defaultMonitorId?: string | null;
  monitorMissingBehavior: MonitorMissingBehavior;
}

export interface StartupSettings {
  enabled: boolean;
  startMinimizedToTray: boolean;
  defaultProfileId?: string | null;
  delaySeconds: number;
  restoreOnLaunch: boolean;
  launchMissingApps: boolean;
}

export interface TraySettings {
  minimizeToTrayOnClose: boolean;
  leftClickAction: TrayClickAction;
}

export interface EnforcementSettings {
  enabled: boolean;
  profileId?: string | null;
  restoreOnDesktopReveal: boolean;
  restoreAfterGameExit: boolean;
}

export interface WindowAutoLayoutConfig {
  schemaVersion: number;
  appVersion: string;
  global: GlobalSettings;
  startup: StartupSettings;
  tray: TraySettings;
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
  isMaximized: boolean;
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

export interface RuntimeStatus {
  automaticRestoreEnabled: boolean;
  automaticRestoreProfileId?: string | null;
  automaticRestoreProfileName?: string | null;
  restoring: boolean;
  startupRegistered: boolean;
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
