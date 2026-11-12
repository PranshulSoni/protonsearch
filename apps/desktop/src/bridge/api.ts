// The only module allowed to talk to the Tauri shell (or the browser mock).
import { invoke } from "@tauri-apps/api/core";
import type {
  SearchResult,
  ActionSpec,
  AgentApprovalResponse,
  AgentModel,
  AgentRunRequest,
  AgentRuntime,
  AgentStatus,
  PiInstallStatus,
  PiInstallPlan,
  AgentTask,
  AgentTaskNode,
  AgentPermissionState,
  DetectedAgent,
  ControlsStatus,
  MediaSessionInfo,
  MonitorInfo,
  PreviewPayload,
  ActivityItem,
  ActivityQuery,
  AppInfo,
  EngineStatus,
  IndexCommand,
  OcrEngine,
  OcrEngineDetails,
  OnboardingInput,
  OnboardingState,
  NativeCommand,
  OpenOutcome,
  SearchRequest,
  SearchResponse,
  Settings,
  SettingsPatch,
} from "./types";

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in (window as unknown as Record<string, unknown>);
}

type Bridge = {
  search(req: SearchRequest): Promise<SearchResponse>;
  openResult(action: ActionSpec): Promise<OpenOutcome>;
  getStatus(): Promise<EngineStatus>;
  getSettings(): Promise<Settings>;
  updateSettings(patch: SettingsPatch): Promise<Settings>;
  setHotkey(hotkey: string): Promise<Settings>;
  confirmNativeAction(command: NativeCommand): Promise<OpenOutcome>;
  agentStatus(): Promise<AgentStatus>;
  agentModels(agentId?: string | null): Promise<AgentModel[]>;
  agentTest(agentId: string): Promise<string>;
  piInstallStatus(refresh?: boolean): Promise<PiInstallStatus>;
  piInstallPlan(): Promise<PiInstallPlan>;
  piInstall(version: string, confirmed: boolean): Promise<void>;
  piInstallCancel(): Promise<void>;
  piConnect(): Promise<void>;
  detectAgents(refresh?: boolean): Promise<DetectedAgent[]>;
  listMonitors(): Promise<MonitorInfo[]>;
  showIsland(): Promise<void>;
  hideIsland(): Promise<void>;
  setIslandGeometry(width: number, height: number): Promise<void>;
  perfMark(label: string, epochMs: number): Promise<void>;
  agentStart(request: AgentRunRequest, parentId?: string | null, runtime?: AgentRuntime | null): Promise<void>;
  agentDelete(taskId: string): Promise<void>;
  agentRename(taskId: string, title: string): Promise<void>;
  agentCancel(runId?: string | null): Promise<void>;
  agentApprove(response: AgentApprovalResponse): Promise<void>;
  agentStop(): Promise<void>;
  agentTasks(limit?: number): Promise<AgentTask[]>;
  agentTaskTree(limit?: number): Promise<AgentTaskNode[]>;
  agentTask(taskId: string): Promise<AgentTask | null>;
  agentPermissions(taskId: string): Promise<AgentPermissionState | null>;
  agentStopAll(): Promise<AgentTask[]>;
  agentPause(taskId: string): Promise<void>;
  agentResume(taskId: string): Promise<void>;
  getActivity(query?: ActivityQuery): Promise<ActivityItem[]>;
  getOnboarding(): Promise<OnboardingState>;
  completeOnboarding(input: OnboardingInput): Promise<Settings>;
  getOcrEngines(): Promise<OcrEngineDetails[]>;
  downloadOcrEngine(engine: OcrEngine): Promise<Settings>;
  installTesseractOcr(): Promise<Settings>;
  openWindowsLanguageSettings(): Promise<void>;
  indexCommand(command: IndexCommand): Promise<void>;
  setLauncherGeometry(height: number, preview: boolean): Promise<void>;
  getPreview(resultId: string, maxChars?: number): Promise<PreviewPayload>;
  getControlsStatus(): Promise<ControlsStatus>;
  setVolume(percent: number): Promise<void>;
  setMuted(muted: boolean): Promise<void>;
  setBrightness(percent: number): Promise<void>;
  setAudioOutput(deviceId: string): Promise<void>;
  setWifi(enabled: boolean): Promise<void>;
  setBluetooth(enabled: boolean): Promise<void>;
  connectWifi(ssid: string): Promise<void>;
  openSettings(page: string): Promise<void>;
  /** `positionMs` is the target of a "seek". */
  mediaCommand(action: string, positionMs?: number): Promise<void>;
  setClipboardPinned(id: string, pinned: boolean): Promise<void>;
  gitBranchCommits(repo: string, branch: string): Promise<SearchResult[]>;
  getMediaSession(): Promise<MediaSessionInfo | null>;
  showLocalToast(title: string, body: string): Promise<void>;
  notificationsDismiss(notificationId: string): Promise<void>;
  notificationsClear(): Promise<void>;
  hideLauncher(): Promise<void>;
  showLauncher(): Promise<void>;
  showMain(): Promise<void>;
  quitApp(): Promise<void>;
  appInfo(): Promise<AppInfo>;
  osUserName(): Promise<string | null>;
};

const tauriBridge: Bridge = {
  search: (req) => invoke<SearchResponse>("search", { req }),
  openResult: (action) => invoke<OpenOutcome>("open_result", { action }),
  getStatus: () => invoke<EngineStatus>("get_status"),
  getSettings: () => invoke<Settings>("get_settings"),
  updateSettings: (patch) => invoke<Settings>("update_settings", { patch }),
  setHotkey: (hotkey) => invoke<Settings>("set_hotkey", { hotkey }),
  confirmNativeAction: (command) => invoke<OpenOutcome>("confirm_native_action", { command }),
  agentStatus: () => invoke<AgentStatus>("agent_status"),
  agentModels: (agentId) => invoke<AgentModel[]>("agent_models", { agentId: agentId ?? null }),
  agentTest: (agentId) => invoke<string>("agent_test", { agentId }),
  detectAgents: (refresh) => invoke<DetectedAgent[]>("detect_agents", { refresh: refresh ?? null }),
  listMonitors: () => invoke<MonitorInfo[]>("list_monitors"),
  showIsland: () => invoke<void>("show_island"),
  hideIsland: () => invoke<void>("hide_island"),
  setIslandGeometry: (width, height) => invoke<void>("set_island_geometry", { width, height }),
  getPreview: (resultId, maxChars) =>
    invoke<PreviewPayload>("get_preview", {
      req: { resultId, maxChars: maxChars ?? null },
    }),
  getControlsStatus: () => invoke<ControlsStatus>("get_controls_status"),
  setVolume: (percent) => invoke<void>("control_set_volume", { percent }),
  setMuted: (muted) => invoke<void>("control_set_muted", { muted }),
  setBrightness: (percent) => invoke<void>("control_set_brightness", { percent }),
  setAudioOutput: (deviceId) => invoke<void>("control_set_audio_output", { deviceId }),
  setWifi: (enabled) => invoke<void>("control_set_wifi", { enabled }),
  setBluetooth: (enabled) => invoke<void>("control_set_bluetooth", { enabled }),
  connectWifi: (ssid) => invoke<void>("control_connect_wifi", { ssid }),
  openSettings: (page) => invoke<void>("control_open_settings", { page }),
  mediaCommand: (action, positionMs) =>
    invoke<void>("control_media", { action, positionMs: positionMs ?? null }),
  setClipboardPinned: (id, pinned) => invoke<void>("clipboard_set_pinned", { id, pinned }),
  gitBranchCommits: (repo, branch) => invoke<SearchResult[]>("git_branch_commits", { repo, branch }),
  getMediaSession: () => invoke<MediaSessionInfo | null>("get_media_session"),
  showLocalToast: (title, body) => invoke<void>("show_local_toast", { title, body }),
  notificationsDismiss: (notificationId) =>
    invoke<void>("notifications_dismiss", { notificationId }),
  notificationsClear: () => invoke<void>("notifications_clear"),
  perfMark: (label, epochMs) => invoke<void>("perf_mark", { label, epochMs }),
  piInstallStatus: (refresh = false) => invoke<PiInstallStatus>("pi_install_status", { refresh }),
  piInstallPlan: () => invoke<PiInstallPlan>("pi_install_plan"),
  piInstall: (version, confirmed) => invoke<void>("pi_install", { version, confirmed }),
  piInstallCancel: () => invoke<void>("pi_install_cancel"),
  piConnect: () => invoke<void>("pi_connect"),
  agentStart: (request, parentId, runtime) =>
    invoke<void>("agent_start", { request, parentId: parentId ?? null, runtime: runtime ?? null }),
  agentDelete: (taskId) => invoke<void>("agent_delete", { taskId }),
  agentRename: (taskId, title) => invoke<void>("agent_rename", { taskId, title }),
  agentCancel: (runId) => invoke<void>("agent_cancel", { runId: runId ?? null }),
  agentApprove: (response) => invoke<void>("agent_approve", { response }),
  agentStop: () => invoke<void>("agent_stop"),
  agentTasks: (limit) => invoke<AgentTask[]>("agent_tasks", { limit: limit ?? null }),
  agentTaskTree: (limit) => invoke<AgentTaskNode[]>("agent_task_tree", { limit: limit ?? null }),
  agentTask: (taskId) => invoke<AgentTask | null>("agent_task", { taskId }),
  agentPermissions: (taskId) => invoke<AgentPermissionState | null>("agent_permissions", { taskId }),
  agentStopAll: () => invoke<AgentTask[]>("agent_stop_all"),
  agentPause: (taskId) => invoke<void>("agent_pause", { taskId }),
  agentResume: (taskId) => invoke<void>("agent_resume", { taskId }),
  getActivity: (query) => invoke<ActivityItem[]>("get_activity", { query: query ?? null }),
  getOnboarding: () => invoke<OnboardingState>("get_onboarding"),
  completeOnboarding: (input) => invoke<Settings>("complete_onboarding", { input }),
  getOcrEngines: () => invoke<OcrEngineDetails[]>("get_ocr_engines"),
  downloadOcrEngine: (engine) => invoke<Settings>("download_ocr_engine", { engine }),
  installTesseractOcr: () => invoke<Settings>("install_tesseract_ocr"),
  openWindowsLanguageSettings: () => invoke<void>("open_windows_language_settings"),
  indexCommand: (command) => invoke<void>("index_command", { command }),
  setLauncherGeometry: (height, preview) =>
    invoke<void>("set_launcher_geometry", { height, preview }),
  hideLauncher: () => invoke<void>("hide_launcher"),
  showLauncher: () => invoke<void>("show_launcher"),
  showMain: () => invoke<void>("show_main"),
  quitApp: () => invoke<void>("quit_app"),
  appInfo: () => invoke<AppInfo>("app_info"),
  osUserName: () => invoke<string | null>("os_user_name"),
};

export const api: Bridge = isTauriRuntime()
  ? tauriBridge
  : (await import("./mock")).mockBridge;

export { invoke };