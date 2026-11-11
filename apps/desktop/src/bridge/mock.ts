// Browser-only mock bridge. It exists so the UI can be developed, unit-tested
// and screenshot-audited in a real browser without the Rust engine. It is not
// shipped in the Tauri build path (the app always uses the real bridge there).
import type {
  ActionSpec,
  ActivityItem,
  ActivityQuery,
  AppInfo,
  EngineStatus,
  IndexCommand,
  OnboardingInput,
  OnboardingState,
  OpenOutcome,
  SearchRequest,
  SearchResponse,
  SearchResult,
  Settings,
  SettingsPatch,
} from "./types";

const params = new URLSearchParams(typeof location !== "undefined" ? location.search : "");
const MOCK_ONBOARDING = params.get("onboard") === "1";
const MOCK_STATE = params.get("state") ?? "ready";
const MOCK_SLOW_MS = params.get("slow") === "1" ? 1200 : 0;
// Agent-catalog fixtures for visual QA: default = Pi detected + Codex missing,
// ?agents=none = no runtimes, ?agents=multi = several CLIs including misses.
const MOCK_AGENTS = params.get("agents") ?? "default";
const MOCK_MONITORS = params.get("monitors") ?? "single";
const USER = "C:\\Users\\Pranshul";

let settings: Settings = {
  schemaVersion: 1,
  userName: MOCK_ONBOARDING ? null : "Pranshul",
  hotkey: "Alt+Space",
  theme: (params.get("theme") as Settings["theme"]) ?? "system",
  density: "comfortable",
  motion: "system",
  launchAtLogin: true,
  ocrEngine: "windows",
  indexing: {
    roots: [
      { path: USER + "\\Documents", mode: "content", enabled: true },
      { path: USER + "\\Desktop", mode: "names_only", enabled: true },
      { path: USER + "\\Downloads", mode: "names_only", enabled: true },
      { path: USER + "\\Pictures", mode: "content_and_ocr", enabled: true },
    ],
    excludedPaths: [],
    contentEnabled: true,
    ocrEnabled: true,
    respectPowerState: true,
    paused: false,
  },
  privacy: {
    activityHistory: true,
    clipboardHistory: true,
    browserHistory: true,
    gitIndexing: true,
    telemetry: false,
  },
  agent: { enabled: false, runtime: "pi", piPath: null, allowWrites: false, workspace: null, thinking: null, provider: null, model: null, sidecarPath: null },
  launcher: {
    primaryMode: "normal",
    island: {
      enabled: true,
      placement: {
        monitor: null,
        anchor: "top_center",
        offsetX: 0,
        offsetY: 22,
        keepInWorkArea: true,
        autoFlipExpanded: true,
      },
      cards: [
        { id: "search", enabled: true },
        { id: "media", enabled: true },
        { id: "clipboard", enabled: true },
        { id: "agent", enabled: true },
        { id: "controls", enabled: true },
        { id: "browser", enabled: true },
        { id: "git", enabled: true },
        { id: "bell", enabled: true },
      ],
      defaultCard: "search",
      showDots: true,
      hideOnFocusLoss: true,
      skipDisabled: true,
      returnToSearch: true,
    },
  },
  clipboard: {
    captureText: true,
    captureImages: true,
    retentionDays: 30,
    maxItems: 500,
    pause: false,
    excludeApps: [],
    clearOnExit: false,
  },
  semantic: { enabled: false, model: null },
  webSearchEnabled: true,
  webSearchEngine: "google",
  onboardingComplete: !MOCK_ONBOARDING,
  legacyMigrated: false,
  resultLimit: 30,
};

const sample: SearchResult[] = [
  {
    id: "file:arch-notes",
    kind: "document",
    title: "Architecture notes.docx",
    subtitle: "Documents / Projects / Nova",
    path: USER + "\\Documents\\Projects\\Nova\\Architecture notes.docx",
    icon: { type: "file_type", ext: "docx" },
    score: 92,
    matchRanges: [{ start: 0, end: 12 }],
    meta: { label: "Yesterday", size: 184320 },
    action: { type: "open_path", path: USER + "\\Documents\\Projects\\Nova\\Architecture notes.docx" },
    actions: [
      { id: "reveal", label: "Reveal in Explorer", action: { type: "reveal_path", path: USER + "\\Documents\\Projects\\Nova\\Architecture notes.docx" } },
      { id: "copy", label: "Copy path", action: { type: "copy_text", text: USER + "\\Documents\\Projects\\Nova\\Architecture notes.docx" } },
    ],
  },
  {
    id: "file:search-md",
    kind: "document",
    title: "search-architecture.md",
    subtitle: "Projects / ProtonSearch / docs",
    path: USER + "\\Documents\\Projects\\protonsearch\\docs\\search-architecture.md",
    icon: { type: "file_type", ext: "md" },
    score: 88,
    matchRanges: [{ start: 0, end: 12 }],
    meta: { label: "Today" },
    action: { type: "open_path", path: USER + "\\Documents\\Projects\\protonsearch\\docs\\search-architecture.md" },
  },
  {
    id: "folder:architecture",
    kind: "folder",
    title: "Architecture",
    subtitle: "Projects / ProtonSearch",
    path: USER + "\\Documents\\Projects\\protonsearch\\docs\\architecture",
    icon: { type: "file_type", ext: "folder" },
    score: 74,
    meta: { itemCount: 24 },
    action: { type: "open_path", path: USER + "\\Documents\\Projects\\protonsearch\\docs\\architecture" },
  },
  {
    id: "file:rank-pdf",
    kind: "document",
    title: "Systems and ranking.pdf",
    subtitle: "Documents / Research",
    path: USER + "\\Documents\\Research\\Systems and ranking.pdf",
    icon: { type: "file_type", ext: "pdf" },
    score: 61,
    meta: { label: "2 days ago" },
    action: { type: "open_path", path: USER + "\\Documents\\Research\\Systems and ranking.pdf" },
  },
  {
    id: "app:vscode",
    kind: "application",
    title: "Visual Studio Code",
    subtitle: "Installed app",
    icon: { type: "monogram" },
    score: 48,
    meta: { label: "App" },
    action: { type: "launch_app", appId: "vscode" },
  },
  {
    id: "action:display",
    kind: "action",
    title: "Open display settings",
    subtitle: "Windows / Settings",
    icon: { type: "kind", kind: "action" },
    score: 40,
    meta: { label: "Action" },
    action: { type: "run_native", command: "open_settings" },
  },
  {
    id: "clip:bench-notes",
    kind: "clipboard",
    title: "Indexing benchmark notes",
    subtitle: "Copied recently",
    icon: { type: "monogram" },
    score: 34,
    meta: { label: "Clipboard" },
    action: { type: "copy_text", text: "Indexing benchmark notes" },
  },
  {
    id: "content:search-arch-42",
    kind: "content_match",
    title: "search-architecture.md",
    subtitle: "docs / ProtonSearch",
    path: USER + "\\Documents\\Projects\\protonsearch\\docs\\search-architecture.md",
    icon: { type: "file_type", ext: "md" },
    score: 30,
    matchRanges: [{ start: 0, end: 12 }],
    meta: {
      label: "line 42",
      line: 42,
      snippet: "The filename architecture keeps a compact arena so warm queries stay under a millisecond.",
    },
    action: {
      type: "open_path",
      path: USER + "\\Documents\\Projects\\protonsearch\\docs\\search-architecture.md",
      line: 42,
    },
  },
];

const activity: ActivityItem[] = [
  { id: 1, timestamp: Date.now() - 240000, kind: "search", title: "architecture", detail: "12 results" },
  { id: 2, timestamp: Date.now() - 900000, kind: "open", title: "architecture notes.docx", detail: "Documents / Projects / Nova" },
  { id: 3, timestamp: Date.now() - 3600000, kind: "launch", title: "Visual Studio Code", detail: "Application" },
  { id: 4, timestamp: Date.now() - 7200000, kind: "index", title: "Content indexing checkpoint", detail: "14,609 documents" },
];

function matches(result: SearchResult, terms: string[]): boolean {
  const haystack = (result.title + " " + result.subtitle + " " + (result.meta?.snippet ?? "")).toLowerCase();
  return terms.length === 0 || terms.every((t) => haystack.includes(t));
}

function status(): EngineStatus {
  const state = MOCK_STATE === "error" ? "error" : MOCK_STATE === "indexing" ? "indexing" : "ready";
  return {
    protocol: 1,
    version: "2.0.0-alpha.1",
    state,
    filenameEntries: 124318,
    contentDocuments: 14609,
    ocrDocuments: 8440,
    roots: settings.indexing.roots.map((r, i) => ({
      path: r.path,
      mode: r.mode,
      enabled: r.enabled ?? true,
      indexed: [42018, 12480, 18442, 8440][i] ?? 1000,
      errors: 0,
      state: "ready",
    })),
    sources: [
      { id: "apps", label: "Applications", enabled: true, itemCount: 126, state: "ready" },
      { id: "browser", label: "Browser history & bookmarks", enabled: settings.privacy.browserHistory, itemCount: 4523, state: "ready" },
      { id: "clipboard", label: "Clipboard", enabled: settings.privacy.clipboardHistory, itemCount: 12, state: "ready" },
      { id: "recent", label: "Recent files", enabled: true, itemCount: 40, state: "ready" },
      { id: "git", label: "Git repositories", enabled: settings.privacy.gitIndexing, itemCount: 15, state: "ready" },
    ],
    agent: { state: "not_configured", runtime: null, model: null, detail: "Optional Python sidecar is not running" },
    lastUpdated: Date.now() - 60000,
    warnings: MOCK_STATE === "error" ? ["Pictures root is unavailable (device disconnected)"] : [],
  };
}

export const mockBridge = {
  async search(req: SearchRequest): Promise<SearchResponse> {
    const started = performance.now();
    const terms = req.query.trim().toLowerCase().split(/\s+/).filter(Boolean);
    const scopeFiltered = sample.filter((r) => {
      if (req.scope && req.scope !== "all") {
        const map: Record<string, string[]> = {
          files: ["file", "folder", "document", "content_match", "recent"],
          folders: ["folder"],
          content: ["document", "content_match", "ocr_text"],
          apps: ["application"],
          clipboard: ["clipboard"],
          browser: ["browser_history", "bookmark"],
          git: ["git_commit", "git_repository", "git_todo"],
          actions: ["action", "calculator", "command"],
        };
        if (map[req.scope] && !map[req.scope].includes(r.kind)) return false;
      }
      return matches(r, terms);
    });
    const results = terms.length === 0 ? sample.slice(0, 6) : scopeFiltered;
    await new Promise((r) => setTimeout(r, MOCK_SLOW_MS || 12));
    return {
      protocol: 1,
      requestId: req.requestId,
      query: req.query,
      results: results.slice(0, req.limit ?? 30),
      total: results.length,
      tookMs: performance.now() - started,
      timings: [
        { tier: 0, label: "exact + prefix", tookMs: 0.7, hits: results.length },
        { tier: 1, label: "fuzzy + content", tookMs: 4.1, hits: 2 },
      ],
      status: status().state === "ready"
        ? { type: "ready", indexedFiles: 124318, contentDocs: 14609 }
        : { type: "indexing", indexedFiles: 42018, phase: "content" },
    };
  },
  async openResult(action: ActionSpec): Promise<OpenOutcome> {
    activity.unshift({
      id: Date.now(),
      timestamp: Date.now(),
      kind: action.type === "launch_app" ? "launch" : "open",
      title: action.type === "launch_app" ? action.appId : action.type === "open_path" ? action.path : action.type,
    });
    return { ok: true, message: "Mock bridge: action recorded (no OS action in browser mode)" };
  },
  async getStatus(): Promise<EngineStatus> {
    return status();
  },
  async getSettings(): Promise<Settings> {
    return settings;
  },
  async updateSettings(patch: SettingsPatch): Promise<Settings> {
    settings = {
      ...settings,
      ...patch,
      privacy: { ...settings.privacy, ...(patch.privacy ?? {}), telemetry: false },
      indexing: { ...settings.indexing, ...(patch.indexing ?? {}) },
      agent: { ...settings.agent, ...(patch.agent ?? {}) },
      semantic: { ...settings.semantic, ...(patch.semantic ?? {}) },
      webSearchEnabled: patch.webSearchEnabled ?? settings.webSearchEnabled,
      webSearchEngine: patch.webSearchEngine ?? settings.webSearchEngine,
    };
    return settings;
  },
  async getActivity(query?: ActivityQuery): Promise<ActivityItem[]> {
    return activity.slice(0, query?.limit ?? 50);
  },
  async getOnboarding(): Promise<OnboardingState> {
    return {
      complete: settings.onboardingComplete,
      legacy: {
        detected: true,
        appDataPath: USER + "\\AppData\\Roaming\\protonsearch",
        indexedFiles: 89813,
        contentDocuments: 14609,
        browserItems: 4523,
        quicklinks: 4,
        snippets: 1,
        settings: 2,
      },
      suggestedRoots: [
        { path: USER + "\\Documents", label: "Documents", exists: true, defaultMode: "content" },
        { path: USER + "\\Desktop", label: "Desktop", exists: true, defaultMode: "names_only" },
        { path: USER + "\\Downloads", label: "Downloads", exists: true, defaultMode: "names_only" },
        { path: USER + "\\Pictures", label: "Pictures", exists: true, defaultMode: "content_and_ocr" },
      ],
      defaults: settings,
    };
  },
  async completeOnboarding(input: OnboardingInput): Promise<Settings> {
    settings = {
      ...settings,
      userName: input.userName || null,
      hotkey: input.hotkey,
      onboardingComplete: true,
      indexing: {
        ...settings.indexing,
        roots: input.roots,
        contentEnabled: input.contentEnabled,
        ocrEnabled: input.ocrEnabled,
      },
      ocrEngine: input.ocrEngine,
      agent: { ...settings.agent, enabled: input.agentOptIn },
    };
    return settings;
  },
  async indexCommand(_command: IndexCommand): Promise<void> {},
  async confirmNativeAction(): Promise<OpenOutcome> {
    return { ok: false, message: "Browser preview: system actions are disabled." };
  },
  async listMonitors(): Promise<import("./types").MonitorInfo[]> {
    const builtIn = {
      id: "\\\\.\\DISPLAY1",
      name: "Built-in display",
      primary: true,
      x: 0,
      y: 0,
      width: 2560,
      height: 1600,
      workX: 0,
      workY: 0,
      workWidth: 2560,
      workHeight: 1560,
      scaleFactor: 1.5,
    };
    if (MOCK_MONITORS === "multi") {
      return [
        builtIn,
        {
          id: "\\\\.\\DISPLAY2",
          name: "External display",
          primary: false,
          x: 2560,
          y: 0,
          width: 1920,
          height: 1200,
          workX: 2560,
          workY: 0,
          workWidth: 1920,
          workHeight: 1160,
          scaleFactor: 1,
        },
      ];
    }
    if (MOCK_MONITORS === "none") return [];
    return [builtIn];
  },

  async showIsland(): Promise<void> {},
  async hideIsland(): Promise<void> {},
  async setIslandGeometry(): Promise<void> {},

  async getControlsStatus(): Promise<import("./types").ControlsStatus> {
    return {
      mediaCapability: { supported: false, reason: "Media session adapter isn't wired in this build." },
      media: null,
      audioOutputs: [{ id: "default", name: "Laptop speakers", isDefault: true }],
      defaultAudioId: "default",
      volumePercent: 62,
      muted: false,
      brightnessCapability: { supported: true, reason: null },
      brightnessPercent: 80,
      brightnessMonitors: [{ index: 0, name: "Built-in display", current: 80, minimum: 0, maximum: 100, method: "dxva2" }],
      wifi: { supported: true, reason: null },
      wifiEnabled: true,
      wifiNetworks: [
        { ssid: "Home-5G", signal: 87, security: "WPA3", connected: true, hasProfile: true, connectable: true },
        { ssid: "Neighbour", signal: 54, security: "WPA2", connected: false, hasProfile: false, connectable: false },
      ],
      wifiError: null,
      bluetooth: { supported: true, reason: null },
      bluetoothEnabled: false,
      bluetoothDevices: [
        { id: "bt-1", name: "Mivi SuperPods Immersio", connected: true, paired: true },
        { id: "bt-2", name: "Pranshul's iPhone", connected: false, paired: true },
      ],
      bluetoothError: null,
      airplaneMode: { supported: false, reason: "Not exposed by this Windows build." },
      nightLight: { supported: false, reason: "Not exposed by this Windows build." },
      energySaver: { supported: true, reason: null },
      notifications: { supported: true, reason: null },
      toastCapability: { supported: false, reason: "Windows toast notifications need an app identity." },
      notificationUnread: 1,
      notificationItems: [
        {
          id: "1",
          title: "ProtonSearch is ready",
          body: "Your index is warm. Press Alt+Space to search.",
          source: "ProtonSearch",
          createdAt: Date.now(),
          read: false,
        },
      ],
    };
  },

  async setVolume(): Promise<void> {},
  async setMuted(): Promise<void> {},
  async setBrightness(): Promise<void> {},
  async setAudioOutput(): Promise<void> {},
  async setWifi(): Promise<void> {},
  async setBluetooth(): Promise<void> {},
  async connectWifi(): Promise<void> {},
  async openSettings(): Promise<void> {},
  async mediaCommand(): Promise<void> {},
  async getMediaSession(): Promise<import("./types").MediaSessionInfo | null> {
    return null;
  },
  async showLocalToast(): Promise<void> {},
  async notificationsDismiss(): Promise<void> {},
  async notificationsClear(): Promise<void> {},

  async getPreview(resultId: string): Promise<import("./types").PreviewPayload> {
    return {
      resultId,
      title: "architecture notes.md",
      subtitle: "Projects / ProtonSearch / docs",
      format: "markdown",
      text: "# Architecture notes\n\nThe search pipeline uses a compact filename arena plus FTS5 content search.",
      snippet: "The search pipeline uses a compact filename arena plus FTS5 content search.",
      highlightRanges: [{ start: 4, end: 18 }],
      imageDataUrl: null,
      ocrText: null,
      fields: [
        { label: "Size", value: "12.4 KB" },
        { label: "Modified", value: "2026-11-09" },
      ],
      status: "ok",
      truncated: false,
      error: null,
      imageMime: null,
      unsupportedReason: null,
    };
  },
  async perfMark(): Promise<void> {},

  async detectAgents(): Promise<import("./types").DetectedAgent[]> {
    const pi = {
      id: "pi",
      name: "Pi",
      kind: "cli",
      version: "1.5.0",
      path: "C:\\Users\\you\\.pi\\agent\\bin\\pi.cmd",
      available: true,
      detail: "Detected on PATH. Read-only in this alpha.",
    };
    const codexMissing = {
      id: "codex",
      name: "Codex",
      kind: "cli",
      version: null,
      path: null,
      available: false,
      detail: "Not found on PATH.",
    };
    const codex = {
      id: "codex",
      name: "Codex",
      kind: "cli",
      version: "0.9.2",
      path: "C:\\Users\\you\\AppData\\Roaming\\npm\\codex.cmd",
      available: true,
      detail: "Detected on PATH.",
    };
    const opencode = {
      id: "opencode",
      name: "OpenCode",
      kind: "cli",
      version: "0.12.4",
      path: "C:\\Users\\you\\AppData\\Roaming\\npm\\opencode.cmd",
      available: true,
      detail: "Detected on PATH.",
    };
    const clineMissing = {
      id: "cline",
      name: "Cline",
      kind: "cli",
      version: null,
      path: null,
      available: false,
      detail: "Not found on PATH.",
    };
    const antigravityMissing = {
      id: "antigravity",
      name: "Antigravity",
      kind: "cli",
      version: null,
      path: null,
      available: false,
      detail: "Not found on PATH.",
    };
    if (MOCK_AGENTS === "none") return [];
    if (MOCK_AGENTS === "multi") {
      return [pi, codex, opencode, clineMissing, antigravityMissing];
    }
    return [pi, codexMissing];
  },

  async agentTasks(): Promise<import("./types").AgentTask[]> {
    return [];
  },

  async agentTaskTree(): Promise<import("./types").AgentTaskNode[]> {
    return [];
  },

  async agentTask(): Promise<import("./types").AgentTask | null> {
    return null;
  },

  async agentPermissions(): Promise<import("./types").AgentPermissionState | null> {
    return null;
  },

  async agentStopAll(): Promise<import("./types").AgentTask[]> {
    return [];
  },

  async agentPause(): Promise<void> {},
  async agentResume(): Promise<void> {},

  async agentStatus(): Promise<import("./types").AgentStatus> {
    return {
      state: "not_configured",
      runtime: null,
      model: null,
      detail: "Browser preview: the optional Python sidecar is only available in the desktop app.",
    };
  },
  async agentStart(): Promise<void> {
    throw new Error("Agent workflows require the desktop app (browser preview has no sidecar).");
  },
  async agentCancel(): Promise<void> {},
  async agentApprove(): Promise<void> {},
  async agentStop(): Promise<void> {},
  async setHotkey(hotkey: string): Promise<Settings> {
    settings = { ...settings, hotkey };
    return settings;
  },
  async setLauncherGeometry(_height: number, _preview: boolean): Promise<void> {},
  async hideLauncher(): Promise<void> {},
  async showLauncher(): Promise<void> {},
  async showMain(): Promise<void> {},
  async quitApp(): Promise<void> {},
  async appInfo(): Promise<AppInfo> {
    return { name: "ProtonSearch", version: "2.0.0-alpha.1", protocol: 1, dataSchema: 1, platform: "windows", arch: "x86_64", perfHooks: false };
  },
};
