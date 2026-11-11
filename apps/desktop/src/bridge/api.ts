// The only module allowed to talk to the Tauri shell (or the browser mock).
import { invoke } from "@tauri-apps/api/core";
import type {
  ActionSpec,
  AgentApprovalResponse,
  AgentRunRequest,
  AgentStatus,
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
  detectAgents(): Promise<DetectedAgent[]>;
  listMonitors(): Promise<MonitorInfo[]>;
  showIsland(): Promise<void>;
  hideIsland(): Promise<void>;
  setIslandGeometry(width: number, height: number): Promise<void>;
  perfMark(label: string, epochMs: number): Promise<void>;
  agentStart(request: AgentRunRequest): Promise<void>;
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
  mediaCommand(action: string): Promise<void>;
  getMediaSession(): Promise<MediaSessionInfo | null>;
  showLocalToast(title: string, body: string): Promise<void>;
  notificationsDismiss(notificationId: string): Promise<void>;
  notificationsClear(): Promise<void>;
  hideLauncher(): Promise<void>;
  showLauncher(): Promise<void>;
  showMain(): Promise<void>;
  quitApp(): Promise<void>;
  appInfo(): Promise<AppInfo>;
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
  detectAgents: () => invoke<DetectedAgent[]>("detect_agents"),
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
  mediaCommand: (action) => invoke<void>("control_media", { action }),
  getMediaSession: () => invoke<MediaSessionInfo | null>("get_media_session"),
  showLocalToast: (title, body) => invoke<void>("show_local_toast", { title, body }),
  notificationsDismiss: (notificationId) =>
    invoke<void>("notifications_dismiss", { notificationId }),
  notificationsClear: () => invoke<void>("notifications_clear"),
  perfMark: (label, epochMs) => invoke<void>("perf_mark", { label, epochMs }),
  agentStart: (request) => invoke<void>("agent_start", { request }),
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
  indexCommand: (command) => invoke<void>("index_command", { command }),
  setLauncherGeometry: (height, preview) =>
    invoke<void>("set_launcher_geometry", { height, preview }),
  hideLauncher: () => invoke<void>("hide_launcher"),
  showLauncher: () => invoke<void>("show_launcher"),
  showMain: () => invoke<void>("show_main"),
  quitApp: () => invoke<void>("quit_app"),
  appInfo: () => invoke<AppInfo>("app_info"),
};

export const api: Bridge = isTauriRuntime()
  ? tauriBridge
  : (await import("./mock")).mockBridge;

export { invoke };