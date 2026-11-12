// TypeScript mirror of crates/ps-ipc/src/lib.rs (protocol v1).
// Keep this file in sync with the Rust contract; it is owned by the Lead.

export const PROTOCOL_VERSION = 1;
export const DATA_SCHEMA_VERSION = 1;

export type SearchScope =
  | "all"
  | "photos"
  | "files"
  | "folders"
  | "content"
  | "apps"
  | "clipboard"
  | "browser"
  | "git"
  | "ocr"
  | "actions"
  | "commands"
  | "settings"
  | "agent";

export type SourceKind =
  | "application"
  | "file"
  | "folder"
  | "document"
  | "content_match"
  | "ocr_text"
  | "clipboard"
  | "browser_history"
  | "bookmark"
  | "recent"
  | "action"
  | "command"
  | "calculator"
  | "quicklink"
  | "snippet"
  | "git_commit"
  | "git_repository"
  | "git_todo"
  | "setting"
  | "window"
  | "agent"
  | "system";

export interface TextRange {
  start: number;
  end: number;
}

export type IconRef =
  | { type: "app_icon"; path: string }
  | { type: "app_icon_data"; data: string }
  | { type: "file_type"; ext: string }
  | { type: "monogram" }
  | { type: "kind"; kind: SourceKind };

export type NativeCommand =
  | "lock_workstation"
  | "sleep"
  | "shutdown"
  | "restart"
  | "empty_recycle_bin"
  | "open_settings"
  | "open_task_manager"
  | "open_terminal"
  | "toggle_dark_mode"
  | "pause_indexing"
  | "resume_indexing";

export type ActionSpec =
  | { type: "open_path"; path: string; line?: number | null; column?: number | null }
  | { type: "reveal_path"; path: string }
  | { type: "open_url"; url: string }
  | { type: "launch_app"; appId: string; target?: string | null }
  | { type: "copy_text"; text: string }
  | { type: "copy_image"; path: string }
  | { type: "run_native"; command: NativeCommand }
  | { type: "run_workflow"; id: string }
  | { type: "none" };

export interface ActionItem {
  id: string;
  label: string;
  action: ActionSpec;
  destructive?: boolean;
  shortcut?: string | null;
}

export interface ResultMeta {
  label?: string | null;
  size?: number | null;
  modified?: number | null;
  itemCount?: number | null;
  line?: number | null;
  snippet?: string | null;
  sourceLabel?: string | null;
  thumbnailDataUrl?: string | null;
  imageWidth?: number | null;
  imageHeight?: number | null;
  mime?: string | null;
  /** Clipboard rows: pinned items are listed first. */
  pinned?: boolean;
  branches?: string[];
  /** Browser rows: the site's favicon from the local browser profile. */
  faviconDataUrl?: string | null;
}

export interface SearchResult {
  id: string;
  kind: SourceKind;
  title: string;
  subtitle: string;
  path?: string | null;
  icon: IconRef;
  score: number;
  matchRanges?: TextRange[];
  meta?: ResultMeta;
  action: ActionSpec;
  actions?: ActionItem[];
}

export interface SearchFilters {
  kinds?: SourceKind[];
  extensions?: string[];
  underPath?: string | null;
  modifiedAfter?: number | null;
  modifiedBefore?: number | null;
  minSize?: number | null;
  maxSize?: number | null;
}

export interface SearchRequest {
  protocol?: number;
  requestId: number;
  query: string;
  scope?: SearchScope;
  filters?: SearchFilters;
  limit?: number;
}

export interface TierTiming {
  tier: number;
  label: string;
  tookMs: number;
  hits: number;
}

export type SearchStatus =
  | { type: "ready"; indexedFiles: number; contentDocs: number }
  | { type: "indexing"; indexedFiles: number; phase: string }
  | { type: "degraded"; reason: string };

export interface SearchResponse {
  protocol: number;
  requestId: number;
  query: string;
  results: SearchResult[];
  total: number;
  tookMs: number;
  timings: TierTiming[];
  status: SearchStatus;
}

export type IndexerState = "starting" | "ready" | "indexing" | "paused" | "error";
export type RootMode = "names_only" | "content" | "content_and_ocr";
export type ThemeMode = "system" | "light" | "dark";
export type Density = "comfortable" | "compact";
export type MotionPreference = "system" | "reduced" | "full";

export interface RootConfig {
  path: string;
  mode: RootMode;
  enabled?: boolean;
}

export interface RootStatus {
  path: string;
  mode: RootMode;
  enabled: boolean;
  indexed: number;
  errors: number;
  state: string;
}

export interface SourceStatus {
  id: string;
  label: string;
  enabled: boolean;
  itemCount: number;
  state: string;
  detail?: string | null;
}

export type AgentState = "not_configured" | "stopped" | "running" | "error";

export interface AgentStatus {
  state: AgentState;
  runtime?: string | null;
  model?: string | null;
  detail?: string | null;
}

export interface EngineStatus {
  protocol: number;
  version: string;
  state: IndexerState;
  reindexing?: boolean;
  filenameEntries: number;
  contentDocuments: number;
  ocrDocuments: number;
  roots: RootStatus[];
  sources: SourceStatus[];
  agent?: AgentStatus | null;
  lastUpdated: number;
  warnings: string[];
}

export interface IndexingSettings {
  roots: RootConfig[];
  excludedPaths: string[];
  contentEnabled: boolean;
  ocrEnabled: boolean;
  respectPowerState: boolean;
  paused: boolean;
}
export type OcrEngine = "windows" | "pp_ocr_v6" | "indra" | "tesseract";
export type OcrEngineAvailability = "available" | "not_installed";
export interface OcrEngineOption {
  engine: OcrEngine;
  availability: OcrEngineAvailability;
}
export interface OcrEngineDetails {
  engine: OcrEngine;
  name: string;
  availability: OcrEngineAvailability;
  sizeLabel: string;
  speedLabel: string;
  badge: string;
  description: string;
  highlights: string[];
  license?: string | null;
  downloadSizeBytes?: number | null;
}
export const OCR_ENGINE_OPTIONS: OcrEngineOption[] = [
  { engine: "windows", availability: "available" },
  { engine: "pp_ocr_v6", availability: "not_installed" },
  { engine: "indra", availability: "not_installed" },
  { engine: "tesseract", availability: "not_installed" },
];

export interface PrivacySettings {
  activityHistory: boolean;
  clipboardHistory: boolean;
  browserHistory: boolean;
  gitIndexing: boolean;
  telemetry: boolean;
}

export type AgentRuntime = "pi" | "pydantic" | "claude" | "codex" | "opencode" | "gemini";

export interface AgentSettings {
  enabled: boolean;
  runtime?: AgentRuntime;
  piPath?: string | null;
  allowWrites?: boolean;
  workspace?: string | null;
  thinking?: string | null;
  provider?: string | null;
  model?: string | null;
  /** Default model per CLI runtime (claude, codex, opencode, gemini). Pi uses `model`. */
  cliModels?: Record<string, string>;
  sidecarPath?: string | null;
}

export interface SemanticSettings {
  enabled: boolean;
  model?: string | null;
}

export type WebSearchEngine = "google" | "bing" | "duck_duck_go" | "you_tube";

export type LauncherMode = "normal" | "dynamic_island";

export type IslandCardId =
  | "search"
  | "media"
  | "clipboard"
  | "agent"
  | "controls"
  | "browser"
  | "git"
  | "bell";

export interface IslandCardConfig {
  id: IslandCardId;
  enabled: boolean;
}

export type IslandAnchor =
  | "top_left"
  | "top_center"
  | "top_right"
  | "center_left"
  | "center"
  | "center_right"
  | "bottom_left"
  | "bottom_center"
  | "bottom_right";

export interface IslandPlacement {
  monitor: string | null;
  anchor: IslandAnchor;
  offsetX: number;
  offsetY: number;
  keepInWorkArea: boolean;
  autoFlipExpanded: boolean;
}

export interface IslandSettings {
  enabled: boolean;
  placement: IslandPlacement;
  cards: IslandCardConfig[];
  defaultCard: IslandCardId;
  showDots: boolean;
  liquidGlass?: boolean;
  liquidGlassLevel?: number;
  hideOnFocusLoss: boolean;
  skipDisabled: boolean;
  returnToSearch: boolean;
}

export interface LauncherSettings {
  primaryMode: LauncherMode;
  island: IslandSettings;
}

export interface ClipboardSettings {
  captureText: boolean;
  captureImages: boolean;
  retentionDays: number;
  maxItems: number;
  pause: boolean;
  excludeApps: string[];
  clearOnExit: boolean;
}

export interface ControlCapability {
  supported: boolean;
  reason: string | null;
  fallbackUri?: string | null;
}

export interface MediaSessionInfo {
  title: string;
  artist: string;
  album: string | null;
  artworkDataUrl: string | null;
  playing: boolean;
  positionMs: number | null;
  durationMs: number | null;
  canSeek: boolean;
  canShuffle: boolean;
  shuffle: boolean;
  canRepeat: boolean;
  repeat: boolean;
  appId: string | null;
}

export interface AudioDeviceInfo {
  id: string;
  name: string;
  isDefault: boolean;
}

export interface WifiNetworkInfo {
  ssid: string;
  signal: number;
  security: string;
  connected: boolean;
  hasProfile: boolean;
  connectable: boolean;
}

export interface BrightnessMonitorInfo {
  index: number;
  name: string;
  current: number;
  minimum: number;
  maximum: number;
  method: "dxva2" | "ddc_ci";
}

export interface BluetoothDeviceInfo {
  id: string;
  name: string;
  connected: boolean;
  paired: boolean;
}

export interface AppNotification {
  id: string;
  title: string;
  body: string;
  source: string;
  createdAt: number;
  read: boolean;
}

export interface ControlsStatus {
  mediaCapability: ControlCapability;
  media: MediaSessionInfo | null;
  audioOutputs: AudioDeviceInfo[];
  defaultAudioId: string | null;
  volumePercent: number | null;
  muted: boolean | null;
  brightnessCapability: ControlCapability;
  brightnessPercent: number | null;
  brightnessMonitors: BrightnessMonitorInfo[];
  wifi: ControlCapability;
  wifiEnabled: boolean | null;
  wifiNetworks: WifiNetworkInfo[];
  wifiError: string | null;
  bluetooth: ControlCapability;
  bluetoothEnabled: boolean | null;
  bluetoothDevices: BluetoothDeviceInfo[];
  bluetoothError: string | null;
  airplaneMode: ControlCapability;
  nightLight: ControlCapability;
  energySaver: ControlCapability;
  notifications: ControlCapability;
  toastCapability: ControlCapability;
  notificationUnread: number;
  notificationItems: AppNotification[];
}

export interface MonitorInfo {
  id: string;
  name: string;
  primary: boolean;
  x: number;
  y: number;
  width: number;
  height: number;
  workX: number;
  workY: number;
  workWidth: number;
  workHeight: number;
  scaleFactor: number;
}

export interface PreviewField {
  label: string;
  value: string;
}

export interface PreviewRange {
  start: number;
  end: number;
}

export interface PreviewPayload {
  resultId: string;
  title: string;
  subtitle: string;
  format: string;
  text: string | null;
  snippet: string | null;
  highlightRanges: PreviewRange[];
  imageDataUrl: string | null;
  ocrText: string | null;
  fields: PreviewField[];
  status: "ok" | "unsupported" | "error";
  truncated: boolean;
  error: string | null;
  imageMime: string | null;
  unsupportedReason: string | null;
}

export interface Settings {
  schemaVersion: number;
  userName?: string | null;
  hotkey: string;
  theme: ThemeMode;
  density: Density;
  motion: MotionPreference;
  launchAtLogin: boolean;
  indexing: IndexingSettings;
  ocrEngine: OcrEngine;
  privacy: PrivacySettings;
  agent: AgentSettings;
  semantic: SemanticSettings;
  launcher: LauncherSettings;
  clipboard: ClipboardSettings;
  webSearchEnabled: boolean;
  webSearchEngine: WebSearchEngine;
  onboardingComplete: boolean;
  legacyMigrated: boolean;
  resultLimit: number;
}

export interface SettingsPatch {
  userName?: string | null;
  hotkey?: string;
  theme?: ThemeMode;
  density?: Density;
  motion?: MotionPreference;
  launchAtLogin?: boolean;
  indexing?: IndexingSettings;
  ocrEngine?: OcrEngine;
  privacy?: PrivacySettings;
  agent?: AgentSettings;
  semantic?: SemanticSettings;
  launcher?: LauncherSettings;
  clipboard?: ClipboardSettings;
  webSearchEnabled?: boolean;
  webSearchEngine?: WebSearchEngine;
  resultLimit?: number;
}

export interface RootSuggestion {
  path: string;
  label: string;
  exists: boolean;
  defaultMode: RootMode;
}

export interface LegacySummary {
  detected: boolean;
  appDataPath?: string | null;
  indexedFiles: number;
  contentDocuments: number;
  browserItems: number;
  quicklinks: number;
  snippets: number;
  settings: number;
}

export interface OnboardingState {
  complete: boolean;
  legacy: LegacySummary;
  suggestedRoots: RootSuggestion[];
  defaults: Settings;
}

export interface OnboardingInput {
  userName?: string | null;
  roots: RootConfig[];
  contentEnabled: boolean;
  ocrEnabled: boolean;
  ocrEngine: OcrEngine;
  hotkey: string;
  agentOptIn: boolean;
  skipped?: boolean;
}

export type ActivityKind = "search" | "open" | "launch" | "index" | "workflow" | "error" | "system";

export interface ActivityItem {
  id: number;
  timestamp: number;
  kind: ActivityKind;
  title: string;
  detail?: string | null;
  path?: string | null;
}

export interface ActivityQuery {
  limit?: number;
  since?: number | null;
  kinds?: ActivityKind[];
}

export type IndexCommand =
  | "pause"
  | "resume"
  | "rebuild"
  | "rebuild_content"
  | "rebuild_ocr"
  | "shutdown";

export interface OpenOutcome {
  ok: boolean;
  message: string;
}

export type AgentTaskRuntime = "pi" | "pydantic" | "claude" | "codex" | "opencode" | "gemini" | "unknown";
export type AgentTaskState =
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "cancelled"
  | "offline";
export type AgentTaskProgressKind = "determinate" | "indeterminate";
export type AgentTaskLogLevel = "info" | "warn" | "error";

export interface AgentTaskProgress {
  kind: AgentTaskProgressKind;
  fraction: number | null;
}

export interface AgentTaskLogEntry {
  at: number;
  level: AgentTaskLogLevel;
  message: string;
}

export interface AgentTaskErrorInfo {
  code: string;
  message: string;
}

export interface AgentPermissionState {
  requested: string[];
  granted: string[];
  denied: string[];
  pending: string | null;
  readOnly: boolean;
}

export interface AgentTaskControls {
  canCancel: boolean;
  canPause: boolean;
  canResume: boolean;
  canRetry: boolean;
}

export interface AgentTaskRestart {
  at: number;
  reason: string;
}

export interface AgentTask {
  id: string;
  runtime: AgentTaskRuntime;
  runtimeVersion: string | null;
  parentId: string | null;
  title: string;
  promptSummary: string;
  state: AgentTaskState;
  progress: AgentTaskProgress;
  createdAt: number;
  startedAt: number | null;
  endedAt: number | null;
  updatedAt: number;
  log: AgentTaskLogEntry[];
  error: AgentTaskErrorInfo | null;
  permissions: AgentPermissionState;
  writeAccess: boolean;
  controls: AgentTaskControls;
  restarts: AgentTaskRestart[];
  answer?: string | null;
  model: string | null;
}

export interface AgentTaskNode {
  task: AgentTask;
  children: AgentTaskNode[];
}

export interface DetectedAgent {
  id: string;
  name: string;
  kind: string;
  version: string | null;
  path: string | null;
  available: boolean;
  detail: string | null;
}

export interface AppInfo {
  name: string;
  version: string;
  protocol: number;
  dataSchema: number;
  platform: string;
  arch: string;
  /** True only when the shell runs with PROTONSEARCH_PERF_HOOKS=1. */
  perfHooks?: boolean;
}

export interface AgentRunRequest {
  runId: string;
  prompt: string;
  contextPaths?: string[];
  model?: string | null;
  maxSteps?: number | null;
}

/** A model from `pi --list-models`. A run sends it as `provider/id`. */
export interface AgentModel {
  provider: string;
  id: string;
}

export interface AgentApprovalResponse {
  runId: string;
  approvalId: string;
  approved: boolean;
}

export type AgentEvent =
  | { type: "started"; runId: string; runtime: string; model: string }
  | { type: "plan"; steps: string[] }
  | { type: "progress"; message: string; fraction?: number | null }
  | {
      type: "approval_request";
      approvalId: string;
      title: string;
      detail: string;
      action: ActionSpec;
      destructive: boolean;
    }
  | { type: "tool_result"; tool: string; ok: boolean; preview: string }
  | { type: "output"; text: string; finalAnswer: boolean }
  | { type: "error"; code: string; message: string }
  | { type: "finished"; ok: boolean; durationMs: number };

export interface PiInstallStatus {
  state: "not_installed" | "needs_authentication" | "ready" | "running" | "version_mismatch" | "error";
  path: string | null;
  version: string | null;
  nodeOk: boolean;
  authenticationConfigured: boolean;
  installationMethod: string | null;
  gitBashAvailable: boolean;
  detail: string;
}
export interface PiInstallPlan {
  version: string;
  unpackedBytes: number;
  integrity: string;
  license: string;
  destination: string;
  sourceUrl: string;
  dependenciesSizeKnown: boolean;
}
