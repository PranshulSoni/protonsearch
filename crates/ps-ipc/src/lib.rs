//! ProtonSearch v2 IPC contract.
//!
//! This crate is the single source of truth for messages exchanged between the
//! Rust engine, the Tauri shell and the WebView UI (and, later, the optional
//! Python agent sidecar). It intentionally depends on serde only, so every
//! layer can share it.
//!
//! Rules:
//! - Bump PROTOCOL_VERSION on any breaking change.
//! - Field names are camelCase on the wire; enum variants are snake_case.
//! - The TypeScript mirror lives in apps/desktop/src/bridge/types.ts.
//!   Keep both in sync; contract tests below document the shape.

use serde::{Deserialize, Serialize};

/// Version of the desktop <-> UI protocol.
pub const PROTOCOL_VERSION: u32 = 1;

/// Bumped whenever the on-disk v2 data layout changes incompatibly.
pub const DATA_SCHEMA_VERSION: u32 = 1;

fn default_limit() -> usize {
    30
}

fn default_protocol() -> u32 {
    PROTOCOL_VERSION
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

/// Which slice of the index a query should search. All is the launcher default;
/// scopes are used by explicit prefixes such as file: app: content: history:.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SearchScope {
    #[default]
    All,
    Files,
    Folders,
    Content,
    Apps,
    Clipboard,
    Browser,
    Git,
    Ocr,
    Actions,
    Commands,
    Settings,
    Agent,
}

/// Optional structured filters layered on top of free text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchFilters {
    /// Restrict to specific result families (empty = all).
    #[serde(default)]
    pub kinds: Vec<SourceKind>,
    /// Lower-case extensions without the dot, e.g. ["pdf", "docx"].
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Only results whose path starts with this prefix (search-path scope).
    #[serde(default)]
    pub under_path: Option<String>,
    /// Unix milliseconds; only results modified after this instant.
    #[serde(default)]
    pub modified_after: Option<i64>,
    #[serde(default)]
    pub modified_before: Option<i64>,
    #[serde(default)]
    pub min_size: Option<u64>,
    #[serde(default)]
    pub max_size: Option<u64>,
}

/// A versioned search request. request_id is monotonic per UI session; the
/// engine must drop responses for stale ids rather than let late tiers win.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    #[serde(default = "default_protocol")]
    pub protocol: u32,
    pub request_id: u64,
    pub query: String,
    #[serde(default)]
    pub scope: SearchScope,
    #[serde(default)]
    pub filters: SearchFilters,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

impl SearchRequest {
    pub fn new(request_id: u64, query: impl Into<String>) -> Self {
        Self {
            protocol: PROTOCOL_VERSION,
            request_id,
            query: query.into(),
            scope: SearchScope::All,
            filters: SearchFilters::default(),
            limit: default_limit(),
        }
    }
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

/// Result family. The UI groups results by this value and renders a badge or
/// icon accordingly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Application,
    File,
    Folder,
    Document,
    ContentMatch,
    OcrText,
    Clipboard,
    BrowserHistory,
    Bookmark,
    Recent,
    Action,
    Command,
    Calculator,
    Quicklink,
    Snippet,
    GitCommit,
    GitRepository,
    GitTodo,
    Setting,
    Window,
    Agent,
    System,
}

impl SourceKind {
    /// Stable group label used by the launcher UI.
    pub fn group_label(self) -> &'static str {
        match self {
            SourceKind::Application => "APPLICATIONS",
            SourceKind::File | SourceKind::Folder => "FILES",
            SourceKind::Document | SourceKind::ContentMatch | SourceKind::OcrText => "DOCUMENTS",
            SourceKind::Clipboard => "CLIPBOARD",
            SourceKind::BrowserHistory | SourceKind::Bookmark => "BROWSER",
            SourceKind::Recent => "RECENT",
            SourceKind::Action | SourceKind::Command | SourceKind::Calculator => "ACTIONS",
            SourceKind::Quicklink | SourceKind::Snippet => "WORKFLOWS",
            SourceKind::GitCommit | SourceKind::GitRepository | SourceKind::GitTodo => "GIT",
            SourceKind::Setting => "SETTINGS",
            SourceKind::Window => "WINDOWS",
            SourceKind::Agent => "AGENT",
            SourceKind::System => "SYSTEM",
        }
    }
}

/// Highlight range within a result title, as UTF-16 code-unit offsets to match
/// the JavaScript string model. start is inclusive, end exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

/// How the UI should render the leading icon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IconRef {
    /// Cached PNG of a real Windows icon; path is absolute (asset protocol).
    AppIcon { path: String },
    /// Real Windows icon as a self-contained PNG data URL. Used so the UI never
    /// needs a filesystem/protocol scope to show application icons.
    AppIconData { data: String },
    /// File-extension badge (UI draws a letter/monogram tile).
    FileType { ext: String },
    /// A monogram chosen by the UI from the title.
    Monogram,
    /// Built-in semantic kind; UI maps it to its icon set.
    Kind { kind: SourceKind },
}

/// Actions are always structured; no shell strings are ever interpolated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionSpec {
    /// Open a file/folder with the OS default handler.
    OpenPath {
        path: String,
        #[serde(default)]
        line: Option<u32>,
        #[serde(default)]
        column: Option<u32>,
    },
    /// Show a path in File Explorer.
    RevealPath { path: String },
    /// Open an http(s) URL in the default browser.
    OpenUrl { url: String },
    /// Launch a Start-menu / UWP application.
    LaunchApp {
        app_id: String,
        #[serde(default)]
        target: Option<String>,
    },
    /// Copy a string to the clipboard.
    CopyText { text: String },
    /// Run one of the built-in, reviewed native commands.
    RunNative { command: NativeCommand },
    /// Execute the action described by a quicklink/snippet entry.
    RunWorkflow { id: String },
    /// No-op (used for disabled rows / informational results).
    None,
}

/// Reviewed, enumerated native commands. Destructive ones require a
/// confirmation in the UI and a permission check in the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeCommand {
    LockWorkstation,
    Sleep,
    Shutdown,
    Restart,
    EmptyRecycleBin,
    OpenSettings,
    OpenTaskManager,
    OpenTerminal,
    ToggleDarkMode,
    PauseIndexing,
    ResumeIndexing,
}

/// An extra context-menu action attached to a result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionItem {
    pub id: String,
    pub label: String,
    pub action: ActionSpec,
    #[serde(default)]
    pub destructive: bool,
    #[serde(default)]
    pub shortcut: Option<String>,
}

/// Secondary details rendered as quiet metadata next to a result.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultMeta {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub modified: Option<i64>,
    #[serde(default)]
    pub item_count: Option<u64>,
    #[serde(default)]
    pub line: Option<u32>,
    #[serde(default)]
    pub snippet: Option<String>,
    #[serde(default)]
    pub source_label: Option<String>,
}

/// One rendered result row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    /// Stable identity across incremental refreshes (kind + canonical path/id).
    pub id: String,
    pub kind: SourceKind,
    pub title: String,
    pub subtitle: String,
    #[serde(default)]
    pub path: Option<String>,
    pub icon: IconRef,
    /// Higher is better. Deterministic for a given corpus + query.
    pub score: f32,
    #[serde(default)]
    pub match_ranges: Vec<TextRange>,
    #[serde(default)]
    pub meta: ResultMeta,
    pub action: ActionSpec,
    #[serde(default)]
    pub actions: Vec<ActionItem>,
}

/// Per-tier timing for the developer/perf overlay and benchmark harness.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TierTiming {
    pub tier: u8,
    pub label: String,
    pub took_ms: f32,
    pub hits: usize,
}

/// Coarse index state for the UI status dot and status bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SearchStatus {
    Ready {
        indexed_files: u64,
        content_docs: u64,
    },
    Indexing {
        indexed_files: u64,
        phase: String,
    },
    Degraded {
        reason: String,
    },
}

fn default_protocol_version() -> u32 {
    PROTOCOL_VERSION
}

/// Response to a SearchRequest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    #[serde(default = "default_protocol_version")]
    pub protocol: u32,
    pub request_id: u64,
    pub query: String,
    pub results: Vec<SearchResult>,
    pub total: usize,
    pub took_ms: f32,
    #[serde(default)]
    pub timings: Vec<TierTiming>,
    pub status: SearchStatus,
}

// ---------------------------------------------------------------------------
// Status / settings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexerState {
    Starting,
    Ready,
    Indexing,
    Paused,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootMode {
    /// Only names/paths; fastest, no file content is read.
    NamesOnly,
    /// Names + extracted text content.
    Content,
    /// Names + content + OCR of images/screenshots.
    ContentAndOcr,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootConfig {
    pub path: String,
    pub mode: RootMode,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootStatus {
    pub path: String,
    pub mode: RootMode,
    pub enabled: bool,
    pub indexed: u64,
    pub errors: u64,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatus {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub item_count: u64,
    pub state: String,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    NotConfigured,
    Stopped,
    Running,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    pub state: AgentState,
    #[serde(default)]
    pub runtime: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
}

/// Aggregate engine status for the desktop app.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    #[serde(default = "default_protocol_version")]
    pub protocol: u32,
    pub version: String,
    pub state: IndexerState,
    pub filename_entries: u64,
    pub content_documents: u64,
    pub ocr_documents: u64,
    #[serde(default)]
    pub roots: Vec<RootStatus>,
    #[serde(default)]
    pub sources: Vec<SourceStatus>,
    #[serde(default)]
    pub agent: Option<AgentStatus>,
    /// Unix ms of the last index write.
    #[serde(default)]
    pub last_updated: i64,
    /// Non-fatal degradation messages (e.g. browser DB locked).
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Density {
    #[default]
    Comfortable,
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MotionPreference {
    #[default]
    System,
    Reduced,
    Full,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexingSettings {
    pub roots: Vec<RootConfig>,
    #[serde(default)]
    pub excluded_paths: Vec<String>,
    pub content_enabled: bool,
    pub ocr_enabled: bool,
    /// Respect Windows battery-saver / high-load states.
    #[serde(default = "default_true")]
    pub respect_power_state: bool,
    #[serde(default)]
    pub paused: bool,
}

impl Default for IndexingSettings {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            excluded_paths: Vec::new(),
            content_enabled: false,
            ocr_enabled: false,
            respect_power_state: true,
            paused: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacySettings {
    pub activity_history: bool,
    pub clipboard_history: bool,
    pub browser_history: bool,
    pub git_indexing: bool,
    /// Always false in v2; present so the UI can show "no telemetry" truthfully.
    pub telemetry: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            activity_history: true,
            // Clipboard contents can include passwords and secrets: opt-in only.
            clipboard_history: false,
            browser_history: false,
            git_indexing: false,
            telemetry: false,
        }
    }
}

/// Which optional agent runtime executes workflows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentRuntime {
    /// Pi coding-agent CLI (https://pi.dev), spawned on demand as a subprocess.
    #[default]
    Pi,
    /// Legacy embedded Pydantic AI sidecar (kept as an optional fallback).
    Pydantic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSettings {
    pub enabled: bool,
    #[serde(default)]
    pub runtime: AgentRuntime,
    /// Optional explicit path to the pi launcher (pi.cmd / pi executable).
    #[serde(default)]
    pub pi_path: Option<String>,
    /// When false (default) Pi runs with read-only tools only.
    #[serde(default)]
    pub allow_writes: bool,
    /// Working directory for Pi runs. None -> a dedicated workspace inside the
    /// ProtonSearch data directory (never the user's whole home by default).
    #[serde(default)]
    pub workspace: Option<String>,
    /// Optional Pi thinking level; None -> Pi default (env override supported).
    #[serde(default)]
    pub thinking: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub sidecar_path: Option<String>,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            runtime: AgentRuntime::Pi,
            pi_path: None,
            allow_writes: false,
            workspace: None,
            thinking: None,
            provider: None,
            model: None,
            sidecar_path: None,
        }
    }
}

/// Search engine used for the "Search the web for ..." result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchEngine {
    #[default]
    Google,
    Bing,
    DuckDuckGo,
    YouTube,
}

impl WebSearchEngine {
    pub fn label(self) -> &'static str {
        match self {
            WebSearchEngine::Google => "Google",
            WebSearchEngine::Bing => "Bing",
            WebSearchEngine::DuckDuckGo => "DuckDuckGo",
            WebSearchEngine::YouTube => "YouTube",
        }
    }
    pub fn search_url(self, query: &str) -> String {
        let encoded = urlencode(query);
        match self {
            WebSearchEngine::Google => format!("https://www.google.com/search?q={encoded}"),
            WebSearchEngine::Bing => format!("https://www.bing.com/search?q={encoded}"),
            WebSearchEngine::DuckDuckGo => format!("https://duckduckgo.com/?q={encoded}"),
            WebSearchEngine::YouTube => format!("https://www.youtube.com/results?search_query={encoded}"),
        }
    }
}

/// Minimal RFC 3986 query encoding (no external dependencies).
pub fn urlencode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SemanticSettings {
    pub enabled: bool,
    #[serde(default)]
    pub model: Option<String>,
}

/// Persisted user settings (v2). Stored as JSON in %APPDATA%/protonsearch/v2/settings.json.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub schema_version: u32,
    #[serde(default)]
    pub user_name: Option<String>,
    pub hotkey: String,
    pub theme: ThemeMode,
    pub density: Density,
    pub motion: MotionPreference,
    pub launch_at_login: bool,
    pub indexing: IndexingSettings,
    pub privacy: PrivacySettings,
    pub agent: AgentSettings,
    pub semantic: SemanticSettings,
    /// "Search the web for ..." offer in the launcher (default on).
    #[serde(default = "default_true")]
    pub web_search_enabled: bool,
    #[serde(default)]
    pub web_search_engine: WebSearchEngine,
    pub onboarding_complete: bool,
    #[serde(default)]
    pub legacy_migrated: bool,
    /// Preferred result limit in the launcher (clamped 5..=100).
    #[serde(default = "default_result_limit")]
    pub result_limit: usize,
}

fn default_result_limit() -> usize {
    30
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: DATA_SCHEMA_VERSION,
            user_name: None,
            hotkey: "Alt+Space".to_string(),
            theme: ThemeMode::System,
            density: Density::Comfortable,
            motion: MotionPreference::System,
            launch_at_login: false,
            indexing: IndexingSettings::default(),
            privacy: PrivacySettings::default(),
            agent: AgentSettings::default(),
            semantic: SemanticSettings::default(),
            web_search_enabled: true,
            web_search_engine: WebSearchEngine::default(),
            onboarding_complete: false,
            legacy_migrated: false,
            result_limit: default_result_limit(),
        }
    }
}

/// Patch semantics: only Some fields are applied.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    #[serde(default)]
    pub user_name: Option<Option<String>>,
    #[serde(default)]
    pub hotkey: Option<String>,
    #[serde(default)]
    pub theme: Option<ThemeMode>,
    #[serde(default)]
    pub density: Option<Density>,
    #[serde(default)]
    pub motion: Option<MotionPreference>,
    #[serde(default)]
    pub launch_at_login: Option<bool>,
    #[serde(default)]
    pub indexing: Option<IndexingSettings>,
    #[serde(default)]
    pub privacy: Option<PrivacySettings>,
    #[serde(default)]
    pub agent: Option<AgentSettings>,
    #[serde(default)]
    pub semantic: Option<SemanticSettings>,
    #[serde(default)]
    pub web_search_enabled: Option<bool>,
    #[serde(default)]
    pub web_search_engine: Option<WebSearchEngine>,
    #[serde(default)]
    pub result_limit: Option<usize>,
}

impl Settings {
    /// Apply a patch in place; returns true when something changed.
    pub fn apply(&mut self, patch: &SettingsPatch) -> bool {
        let before = self.clone();
        if let Some(name) = &patch.user_name {
            self.user_name = name.clone();
        }
        if let Some(v) = &patch.hotkey {
            self.hotkey = v.clone();
        }
        if let Some(v) = patch.theme {
            self.theme = v;
        }
        if let Some(v) = patch.density {
            self.density = v;
        }
        if let Some(v) = patch.motion {
            self.motion = v;
        }
        if let Some(v) = patch.launch_at_login {
            self.launch_at_login = v;
        }
        if let Some(v) = &patch.indexing {
            self.indexing = v.clone();
        }
        if let Some(v) = &patch.privacy {
            self.privacy = v.clone();
        }
        if let Some(v) = &patch.agent {
            self.agent = v.clone();
        }
        if let Some(v) = &patch.semantic {
            self.semantic = v.clone();
        }
        if let Some(v) = patch.web_search_enabled {
            self.web_search_enabled = v;
        }
        if let Some(v) = patch.web_search_engine {
            self.web_search_engine = v;
        }
        if let Some(v) = patch.result_limit {
            self.result_limit = v.clamp(5, 100);
        }
        self.privacy.telemetry = false;
        *self != before
    }
}

// ---------------------------------------------------------------------------
// Onboarding / activity
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootSuggestion {
    pub path: String,
    pub label: String,
    pub exists: bool,
    pub default_mode: RootMode,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacySummary {
    pub detected: bool,
    #[serde(default)]
    pub app_data_path: Option<String>,
    #[serde(default)]
    pub indexed_files: u64,
    #[serde(default)]
    pub content_documents: u64,
    #[serde(default)]
    pub browser_items: u64,
    #[serde(default)]
    pub quicklinks: u64,
    #[serde(default)]
    pub snippets: u64,
    #[serde(default)]
    pub settings: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingState {
    pub complete: bool,
    pub legacy: LegacySummary,
    pub suggested_roots: Vec<RootSuggestion>,
    pub defaults: Settings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingInput {
    #[serde(default)]
    pub user_name: Option<String>,
    pub roots: Vec<RootConfig>,
    pub content_enabled: bool,
    pub ocr_enabled: bool,
    pub hotkey: String,
    pub agent_opt_in: bool,
    #[serde(default)]
    pub skipped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    Search,
    Open,
    Launch,
    Index,
    Workflow,
    Error,
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityItem {
    pub id: i64,
    pub timestamp: i64,
    pub kind: ActivityKind,
    pub title: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityQuery {
    #[serde(default)]
    pub limit: usize,
    #[serde(default)]
    pub since: Option<i64>,
    #[serde(default)]
    pub kinds: Vec<ActivityKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexCommand {
    Pause,
    Resume,
    Rebuild,
    RebuildContent,
    RebuildOcr,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenOutcome {
    pub ok: bool,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Agent sidecar protocol (gate 6; contract fixed early on purpose)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunRequest {
    pub run_id: String,
    pub prompt: String,
    #[serde(default)]
    pub context_paths: Vec<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub max_steps: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum AgentEvent {
    Started {
        run_id: String,
        runtime: String,
        model: String,
    },
    Plan {
        steps: Vec<String>,
    },
    Progress {
        message: String,
        fraction: Option<f32>,
    },
    ApprovalRequest {
        approval_id: String,
        title: String,
        detail: String,
        action: ActionSpec,
        destructive: bool,
    },
    ToolResult {
        tool: String,
        ok: bool,
        preview: String,
    },
    Output {
        text: String,
        final_answer: bool,
    },
    Error {
        code: String,
        message: String,
    },
    Finished {
        ok: bool,
        duration_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentApprovalResponse {
    pub run_id: String,
    pub approval_id: String,
    pub approved: bool,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_request_roundtrip_is_camel_case() {
        let req = SearchRequest::new(7, "quarterly report");
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"requestId\":7"), "{json}");
        assert!(json.contains("\"protocol\":1"), "{json}");
        let back: SearchRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn legacy_requests_without_optional_fields_parse() {
        let req: SearchRequest = serde_json::from_str(r#"{"requestId":1,"query":"x"}"#).unwrap();
        assert_eq!(req.limit, 30);
        assert_eq!(req.scope, SearchScope::All);
    }

    #[test]
    fn unknown_extra_fields_are_ignored_by_default() {
        // serde ignores unknown fields unless deny_unknown_fields is set; keep it
        // that way for forward compatibility with newer UIs.
        let req: SearchRequest =
            serde_json::from_str(r#"{"requestId":2,"query":"x","futureField":true}"#).unwrap();
        assert_eq!(req.request_id, 2);
    }

    #[test]
    fn settings_patch_applies_only_some_fields() {
        let mut s = Settings::default();
        let patch = SettingsPatch {
            theme: Some(ThemeMode::Dark),
            privacy: Some(PrivacySettings {
                telemetry: true,
                ..Default::default()
            }),
            result_limit: Some(500),
            ..Default::default()
        };
        assert!(s.apply(&patch));
        assert_eq!(s.theme, ThemeMode::Dark);
        assert!(!s.privacy.telemetry, "telemetry must never be enabled");
        assert_eq!(s.result_limit, 100, "result limit is clamped");
        assert_eq!(s.hotkey, "Alt+Space");
    }

    #[test]
    fn action_spec_is_structured_not_shell() {
        let action = ActionSpec::OpenPath {
            path: "C:\\Users\\x\\a b.txt".into(),
            line: Some(12),
            column: None,
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"open_path\""));
        assert!(json.contains("line"));
    }

    #[test]
    fn icons_serialize_with_type_tag() {
        let icon = IconRef::FileType { ext: "pdf".into() };
        let json = serde_json::to_string(&icon).unwrap();
        assert_eq!(json, r#"{"type":"file_type","ext":"pdf"}"#);
    }

    #[test]
    fn source_kind_group_labels_are_stable() {
        assert_eq!(SourceKind::Application.group_label(), "APPLICATIONS");
        assert_eq!(SourceKind::ContentMatch.group_label(), "DOCUMENTS");
    }

    #[test]
    fn agent_events_use_camel_case_payload_fields() {
        let event = AgentEvent::Started {
            run_id: "run-1".into(),
            runtime: "pydantic-ai".into(),
            model: "test".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"runId\":\"run-1\""), "{json}");
        let approval = AgentEvent::ApprovalRequest {
            approval_id: "a1".into(),
            title: "Open file".into(),
            detail: "path".into(),
            action: ActionSpec::None,
            destructive: false,
        };
        let json = serde_json::to_string(&approval).unwrap();
        assert!(json.contains("\"approvalId\":\"a1\""), "{json}");
    }

    #[test]
    fn default_settings_are_private_by_default() {
        let s = Settings::default();
        assert!(!s.privacy.telemetry);
        assert!(!s.privacy.browser_history);
        assert!(!s.privacy.git_indexing);
        assert!(!s.agent.enabled);
        assert!(!s.semantic.enabled);
        assert!(!s.indexing.content_enabled);
        assert!(!s.indexing.ocr_enabled);
    }
}