// The only module allowed to talk to the Tauri shell (or the browser mock).
import { invoke } from "@tauri-apps/api/core";
import type {
  ActionSpec,
  AgentApprovalResponse,
  AgentRunRequest,
  AgentStatus,
  DetectedAgent,
  ActivityItem,
  ActivityQuery,
  AppInfo,
  EngineStatus,
  IndexCommand,
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
  agentStart(request: AgentRunRequest): Promise<void>;
  agentCancel(runId?: string | null): Promise<void>;
  agentApprove(response: AgentApprovalResponse): Promise<void>;
  agentStop(): Promise<void>;
  getActivity(query?: ActivityQuery): Promise<ActivityItem[]>;
  getOnboarding(): Promise<OnboardingState>;
  completeOnboarding(input: OnboardingInput): Promise<Settings>;
  indexCommand(command: IndexCommand): Promise<void>;
  setLauncherGeometry(height: number, preview: boolean): Promise<void>;
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
  agentStart: (request) => invoke<void>("agent_start", { request }),
  agentCancel: (runId) => invoke<void>("agent_cancel", { runId: runId ?? null }),
  agentApprove: (response) => invoke<void>("agent_approve", { response }),
  agentStop: () => invoke<void>("agent_stop"),
  getActivity: (query) => invoke<ActivityItem[]>("get_activity", { query: query ?? null }),
  getOnboarding: () => invoke<OnboardingState>("get_onboarding"),
  completeOnboarding: (input) => invoke<Settings>("complete_onboarding", { input }),
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