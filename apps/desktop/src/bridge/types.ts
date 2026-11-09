// TypeScript mirror of crates/ps-ipc/src/lib.rs (protocol v1).
// Keep this file in sync with the Rust contract; it is owned by the Lead.

export const PROTOCOL_VERSION = 1;
export const DATA_SCHEMA_VERSION = 1;

export type SearchScope =
  | "all"
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

export interface PrivacySettings {
  activityHistory: boolean;
  clipboardHistory: boolean;
  browserHistory: boolean;
  gitIndexing: boolean;
  telemetry: boolean;
}

export type AgentRuntime = "pi" | "pydantic";

export interface AgentSettings {
  enabled: boolean;
  runtime?: AgentRuntime;
  piPath?: string | null;
  allowWrites?: boolean;
  workspace?: string | null;
  thinking?: string | null;
  provider?: string | null;
  model?: string | null;
  sidecarPath?: string | null;
}

export interface SemanticSettings {
  enabled: boolean;
  model?: string | null;
}

export type WebSearchEngine = "google" | "bing" | "duck_duck_go" | "you_tube";

export interface Settings {
  schemaVersion: number;
  userName?: string | null;
  hotkey: string;
  theme: ThemeMode;
  density: Density;
  motion: MotionPreference;
  launchAtLogin: boolean;
  indexing: IndexingSettings;
  privacy: PrivacySettings;
  agent: AgentSettings;
  semantic: SemanticSettings;
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
  privacy?: PrivacySettings;
  agent?: AgentSettings;
  semantic?: SemanticSettings;
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
}

export interface AgentRunRequest {
  runId: string;
  prompt: string;
  contextPaths?: string[];
  model?: string | null;
  maxSteps?: number | null;
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