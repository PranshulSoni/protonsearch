//! Engine facade: query coordination, ranking, persistence and background
//! indexing. OWNER: L0 (integration layer). Public methods are the contract used
//! by the Tauri shell and the benchmark harness.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::Context;
use ps_ipc::*;
use ps_index::{EntryKind, FilenameIndex};

use crate::catalog;
use crate::db::EngineDb;

pub const INDEX_FILE: &str = "filename-index.bin";
const DB_FILE: &str = "index.db";
const MAX_CONTENT_PER_PASS: usize = 4000;
const MAX_OCR_PER_PASS: usize = 400;
const MAX_WARNINGS: usize = 24;

const STATE_STARTING: u8 = 0;
const STATE_READY: u8 = 1;
const STATE_INDEXING: u8 = 2;
const STATE_PAUSED: u8 = 3;
const STATE_ERROR: u8 = 4;

/// Unix milliseconds.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Directory that owns all v2 data (settings.json, index.db, filename-index.bin).
    pub data_dir: PathBuf,
    /// Legacy v1 database, opened read-only for one-time migration.
    pub legacy_db: Option<PathBuf>,
    pub version: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            data_dir: crate::paths::v2_data_dir(),
            legacy_db: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct Ranked {
    tier: u8,
    priority: u8,
    score: f32,
    result: SearchResult,
}

pub struct Engine {
    config: EngineConfig,
    settings: RwLock<Settings>,
    index: RwLock<FilenameIndex>,
    content: Mutex<Option<ps_content::ContentIndex>>,
    db: EngineDb,
    sources: RwLock<ps_sources::Sources>,
    state: AtomicU8,
    phase: Mutex<String>,
    filename_entries: AtomicU64,
    content_documents: AtomicU64,
    ocr_documents: AtomicU64,
    last_updated: AtomicI64,
    warnings: Mutex<Vec<String>>,
    background: Mutex<Option<JoinHandle<()>>>,
    paused: AtomicBool,
    shutdown: AtomicBool,
    force_rescan: AtomicBool,
    legacy_summary: Mutex<Option<LegacySummary>>,
    /// One-shot confirmation state for destructive native commands: a request
    /// must be repeated through confirm_destructive within 15 seconds.
    pending_destructive: Mutex<Option<(NativeCommand, i64)>>,
}

impl Engine {
    // -- lifecycle ---------------------------------------------------------

    pub fn open(config: EngineConfig) -> anyhow::Result<Self> {
        let data_dir = config.data_dir.clone();
        std::fs::create_dir_all(&data_dir)
            .with_context(|| format!("creating data directory {}", data_dir.display()))?;
        let settings = crate::settings::load(&data_dir.join("settings.json"));

        let mut warnings = Vec::new();
        let index = match FilenameIndex::load(&data_dir.join(INDEX_FILE)) {
            Ok(index) => index,
            Err(err) => {
                if data_dir.join(INDEX_FILE).exists() {
                    warnings.push(format!("filename index could not be loaded ({err}); rebuilding"));
                }
                FilenameIndex::new()
            }
        };
        let filename_entries = index.len() as u64;

        let db_path = data_dir.join(DB_FILE);
        let db = EngineDb::open(&db_path)?;
        let content = match ps_content::ContentIndex::open(&db_path) {
            Ok(index) => Some(index),
            Err(err) => {
                warnings.push(format!("content index unavailable ({err})"));
                None
            }
        };
        let content_documents = content
            .as_ref()
            .map(|c| c.stats().documents)
            .unwrap_or(0);
        let ocr_documents = db.ocr_count();

        let needs_work = settings.onboarding_complete
            && settings
                .indexing
                .roots
                .iter()
                .any(|root| root.enabled && !root.path.trim().is_empty());
        let initial_state = if needs_work { STATE_STARTING } else { STATE_READY };

        Ok(Self {
            config,
            settings: RwLock::new(settings),
            index: RwLock::new(index),
            content: Mutex::new(content),
            db,
            sources: RwLock::new(ps_sources::Sources::default()),
            state: AtomicU8::new(initial_state),
            phase: Mutex::new("waiting".to_string()),
            filename_entries: AtomicU64::new(filename_entries),
            content_documents: AtomicU64::new(content_documents),
            ocr_documents: AtomicU64::new(ocr_documents),
            last_updated: AtomicI64::new(0),
            warnings: Mutex::new(warnings),
            background: Mutex::new(None),
            paused: AtomicBool::new(false),
            shutdown: AtomicBool::new(false),
            force_rescan: AtomicBool::new(false),
            legacy_summary: Mutex::new(None),
            pending_destructive: Mutex::new(None),
        })
    }

    pub fn version(&self) -> &str {
        &self.config.version
    }

    pub fn data_dir(&self) -> &Path {
        &self.config.data_dir
    }

    /// Start background indexing. Fast, idempotent and never blocks the caller.
    pub fn spawn_background(self: Arc<Self>) {
        let mut guard = match self.background.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        if let Some(handle) = guard.as_ref() {
            if !handle.is_finished() {
                return;
            }
        }
        self.shutdown.store(false, Ordering::SeqCst);
        let engine = Arc::clone(&self);
        *guard = Some(std::thread::spawn(move || run_background(engine)));
    }

    // -- search ------------------------------------------------------------

    pub fn search(&self, req: &SearchRequest) -> SearchResponse {
        let started = Instant::now();
        let limit = req.limit.clamp(5, 100);
        let settings = self.settings_snapshot();
        let (prefix_scope, query_text) = effective_query(&req.query, req.scope);
        let parsed = crate::query::ParsedQuery::parse(&query_text);
        let scope = if parsed.scope != SearchScope::All {
            parsed.scope
        } else {
            prefix_scope
        };
        let filters = merged_filters(&parsed.filters, &req.filters);
        let filename_query = if parsed.filename_query.trim().is_empty() {
            query_text.clone()
        } else {
            parsed.filename_query.clone()
        };
        let terms = parsed.terms.clone();
        let mut timings: Vec<TierTiming> = Vec::new();
        let mut ranked: Vec<Ranked> = Vec::new();

        // Empty query: recent items only (launcher shows the RECENT group).
        if terms.is_empty() {
            let sources = self.sources.read().unwrap();
            let mut recent: Vec<&ps_sources::RecentEntry> = sources.recent.iter().collect();
            recent.sort_by(|a, b| b.last_used_ms.cmp(&a.last_used_ms));
            for entry in recent.into_iter().take(5) {
                ranked.push(Ranked {
                    tier: 0,
                    priority: 1,
                    score: 1.0,
                    result: recent_result(entry),
                });
            }
            drop(sources);
            let results = finalize(ranked, limit);
            return self.response(req, query_text, results, timings, started, &settings);
        }

        // Tier 0: calculator, native commands, quicklinks/snippets, apps, recents.
        let tier_start = Instant::now();
        let mut tier0_hits = 0usize;
        if scope_allows(scope, SourceKind::Calculator) {
            if let Some(calc) = ps_sources::calc::evaluate(&query_text)
                .or_else(|| ps_sources::calc::convert(&query_text))
            {
                ranked.push(Ranked {
                    tier: 0,
                    priority: 0,
                    score: 1000.0,
                    result: calculator_result(calc),
                });
                tier0_hits += 1;
            }
        }
        if scope_allows(scope, SourceKind::Command) || scope == SearchScope::All {
            let sources = self.sources.read().unwrap();
            for action in catalog::match_actions(&query_text).into_iter().take(4) {
                ranked.push(Ranked {
                    tier: 0,
                    priority: 1,
                    score: 900.0 - ranked.len() as f32,
                    result: command_result(action),
                });
                tier0_hits += 1;
            }
            for workflow in match_workflows(&sources.workflows, &terms).into_iter().take(3) {
                ranked.push(Ranked {
                    tier: 0,
                    priority: 1,
                    score: 880.0,
                    result: workflow_result(workflow),
                });
                tier0_hits += 1;
            }
            drop(sources);
        }
        if scope_allows(scope, SourceKind::Application) {
            let sources = self.sources.read().unwrap();
            for (app, score) in match_apps(&sources.apps, &terms).into_iter().take(5) {
                ranked.push(Ranked {
                    tier: 0,
                    priority: 2,
                    score,
                    result: app_result(app),
                });
                tier0_hits += 1;
            }
            if terms.len() == 1 {
                for entry in sources
                    .recent
                    .iter()
                    .filter(|r| r.name.to_lowercase().contains(&terms[0]))
                    .take(3)
                {
                    ranked.push(Ranked {
                        tier: 0,
                        priority: 4,
                        score: 600.0,
                        result: recent_result(entry),
                    });
                    tier0_hits += 1;
                }
            }
            drop(sources);
        }
        timings.push(TierTiming {
            tier: 0,
            label: "exact + actions + apps".to_string(),
            took_ms: tier_start.elapsed().as_secs_f64() as f32 * 1000.0,
            hits: tier0_hits,
        });

        // Tier 1: filename index.
        if scope_allows(scope, SourceKind::File) || scope_allows(scope, SourceKind::Folder) {
            let tier_start = Instant::now();
            let extensions: Vec<String> = filters
                .extensions
                .iter()
                .map(|ext| ext.trim_start_matches('.').to_lowercase())
                .collect();
            let kind_filter = match (filters.kinds.contains(&SourceKind::Folder), filters.kinds.contains(&SourceKind::File)) {
                (true, false) => Some(EntryKind::Folder),
                (false, true) => Some(EntryKind::File),
                _ => None,
            };
            let entry_filter = ps_index::EntryFilter {
                kind: kind_filter,
                extensions: &extensions,
                under_path: filters.under_path.as_deref(),
                modified_after: filters.modified_after,
                modified_before: filters.modified_before,
                min_size: filters.min_size,
                max_size: filters.max_size,
            };
            let file_hits = {
                let index = self.index.read().unwrap();
                index.search_filtered(&filename_query, limit.saturating_mul(2), &entry_filter)
            };
            let mut hits = 0usize;
            for hit in &file_hits {
                let index = self.index.read().unwrap();
                if let Some((result, tier)) = file_result(&index, hit, &settings) {
                    let priority = if result.kind == SourceKind::Folder { 3 } else { 3 };
                    ranked.push(Ranked {
                        tier,
                        priority,
                        score: hit.score,
                        result,
                    });
                    hits += 1;
                }
            }
            timings.push(TierTiming {
                tier: 1,
                label: "filename index".to_string(),
                took_ms: tier_start.elapsed().as_secs_f64() as f32 * 1000.0,
                hits,
            });
        }

        // Tier 1: indexed content, clipboard, browser, git.
        if settings.indexing.content_enabled && content_scope_allows(scope) {
            let tier_start = Instant::now();
            let mut hits = 0usize;
            if let Some(result) = self.with_content(|content| content.search(&query_text, 12)) {
                match result {
                    Ok(content_hits) => {
                        for hit in content_hits {
                            ranked.push(Ranked {
                                tier: 1,
                                priority: 3,
                                score: 500.0,
                                result: content_result(hit, &settings),
                            });
                            hits += 1;
                        }
                    }
                    Err(err) => self.push_warning(format!("content search failed: {err}")),
                }
            }
            timings.push(TierTiming {
                tier: 1,
                label: "indexed content".to_string(),
                took_ms: tier_start.elapsed().as_secs_f64() as f32 * 1000.0,
                hits,
            });
        }

        let tier_start = Instant::now();
        let sources = self.sources.read().unwrap();
        let mut source_hits = 0usize;
        if scope_allows(scope, SourceKind::Clipboard) {
            for entry in match_clipboard(&sources.clipboard, &terms).into_iter().take(4) {
                ranked.push(Ranked {
                    tier: 1,
                    priority: 8,
                    score: 400.0,
                    result: clipboard_result(entry),
                });
                source_hits += 1;
            }
        }
        if scope_allows(scope, SourceKind::BrowserHistory) {
            for entry in match_browser(&sources.browser, &terms).into_iter().take(4) {
                ranked.push(Ranked {
                    tier: 1,
                    priority: 6,
                    score: 380.0,
                    result: browser_result(entry),
                });
                source_hits += 1;
            }
        }
        if scope_allows(scope, SourceKind::GitCommit) {
            for entry in match_git(&sources.git, &terms).into_iter().take(4) {
                ranked.push(Ranked {
                    tier: 1,
                    priority: 7,
                    score: 360.0,
                    result: git_result(entry),
                });
                source_hits += 1;
            }
        }
        drop(sources);
        if source_hits > 0 {
            timings.push(TierTiming {
                tier: 1,
                label: "sources".to_string(),
                took_ms: tier_start.elapsed().as_secs_f64() as f32 * 1000.0,
                hits: source_hits,
            });
        }

        // Tier 2: OCR text (already extracted; never run OCR during a query).
        if settings.indexing.ocr_enabled && scope_allows(scope, SourceKind::OcrText) {
            let tier_start = Instant::now();
            let mut hits = 0usize;
            match self.db.ocr_search(&query_text, 5) {
                Ok(ocr_hits) => {
                    for hit in ocr_hits {
                        ranked.push(Ranked {
                            tier: 2,
                            priority: 3,
                            score: 300.0,
                            result: ocr_result(hit, &settings),
                        });
                        hits += 1;
                    }
                }
                Err(err) => self.push_warning(format!("OCR search failed: {err}")),
            }
            timings.push(TierTiming {
                tier: 2,
                label: "OCR text".to_string(),
                took_ms: tier_start.elapsed().as_secs_f64() as f32 * 1000.0,
                hits,
            });
        }

        let results = finalize(ranked, limit);
        let response = self.response(req, query_text.clone(), results, timings, started, &settings);
        if settings.privacy.activity_history && !query_text.trim().is_empty() {
            let count = response.total;
            let _ = self.db.log_activity(
                ActivityKind::Search,
                query_text.trim(),
                Some(&format!("{count} results")),
                None,
            );
        }
        response
    }

    fn response(
        &self,
        req: &SearchRequest,
        query: String,
        results: Vec<SearchResult>,
        timings: Vec<TierTiming>,
        started: Instant,
        settings: &Settings,
    ) -> SearchResponse {
        let state = self.state.load(Ordering::Relaxed);
        let status = match state {
            STATE_INDEXING | STATE_STARTING => SearchStatus::Indexing {
                indexed_files: self.filename_entries.load(Ordering::Relaxed),
                phase: self.phase.lock().map(|p| p.clone()).unwrap_or_default(),
            },
            STATE_ERROR => SearchStatus::Degraded {
                reason: self
                    .warnings
                    .lock()
                    .map(|w| w.last().cloned().unwrap_or_default())
                    .unwrap_or_default(),
            },
            _ => SearchStatus::Ready {
                indexed_files: self.filename_entries.load(Ordering::Relaxed),
                content_docs: self.content_documents.load(Ordering::Relaxed),
            },
        };
        let _ = settings;
        SearchResponse {
            protocol: PROTOCOL_VERSION,
            request_id: req.request_id,
            query,
            total: results.len(),
            results,
            took_ms: started.elapsed().as_secs_f64() as f32 * 1000.0,
            timings,
            status,
        }
    }

    // -- status / settings -------------------------------------------------

    pub fn status(&self) -> EngineStatus {
        let settings = self.settings_snapshot();
        let index = self.index.read().unwrap();
        let counts = index.root_counts();
        let roots: Vec<RootStatus> = settings
            .indexing
            .roots
            .iter()
            .map(|root| {
                let normalized = ps_index::normalize_root(Path::new(&root.path));
                let indexed = index
                    .roots()
                    .iter()
                    .position(|candidate| candidate == &normalized)
                    .and_then(|position| counts.get(position).copied())
                    .unwrap_or(0);
                RootStatus {
                    path: root.path.clone(),
                    mode: root.mode,
                    enabled: root.enabled,
                    indexed,
                    errors: 0,
                    state: if root.enabled { "ready" } else { "disabled" }.to_string(),
                }
            })
            .collect();
        drop(index);
        let sources = self.sources.read().unwrap();
        let source_status = vec![
            SourceStatus {
                id: "apps".into(),
                label: "Applications".into(),
                enabled: true,
                item_count: sources.apps.len() as u64,
                state: if sources.apps.is_empty() { "starting" } else { "ready" }.into(),
                detail: None,
            },
            SourceStatus {
                id: "browser".into(),
                label: "Browser history & bookmarks".into(),
                enabled: settings.privacy.browser_history,
                item_count: sources.browser.len() as u64,
                state: if settings.privacy.browser_history { "ready" } else { "disabled" }.into(),
                detail: None,
            },
            SourceStatus {
                id: "clipboard".into(),
                label: "Clipboard".into(),
                enabled: settings.privacy.clipboard_history,
                item_count: sources.clipboard.len() as u64,
                state: if settings.privacy.clipboard_history { "ready" } else { "disabled" }.into(),
                detail: None,
            },
            SourceStatus {
                id: "recent".into(),
                label: "Recent files".into(),
                enabled: true,
                item_count: sources.recent.len() as u64,
                state: "ready".into(),
                detail: None,
            },
            SourceStatus {
                id: "git".into(),
                label: "Git repositories".into(),
                enabled: settings.privacy.git_indexing,
                item_count: sources.git.len() as u64,
                state: if settings.privacy.git_indexing { "ready" } else { "disabled" }.into(),
                detail: None,
            },
        ];
        drop(sources);

        EngineStatus {
            protocol: PROTOCOL_VERSION,
            version: self.version().to_string(),
            state: indexer_state(self.state.load(Ordering::Relaxed)),
            filename_entries: self.filename_entries.load(Ordering::Relaxed),
            content_documents: self.content_documents.load(Ordering::Relaxed),
            ocr_documents: self.ocr_documents.load(Ordering::Relaxed),
            roots,
            sources: source_status,
            agent: Some(AgentStatus {
                state: if settings.agent.enabled {
                    AgentState::Stopped
                } else {
                    AgentState::NotConfigured
                },
                runtime: None,
                model: settings.agent.model.clone(),
                detail: Some(if settings.agent.enabled {
                    "Agent workflows are enabled but the sidecar is not running.".into()
                } else {
                    "Optional Python sidecar is not configured.".into()
                }),
            }),
            last_updated: self.last_updated.load(Ordering::Relaxed),
            warnings: self.warnings.lock().map(|w| w.clone()).unwrap_or_default(),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings_snapshot()
    }

    pub fn update_settings(&self, patch: SettingsPatch) -> anyhow::Result<Settings> {
        let (next, changed) = {
            let mut guard = self
                .settings
                .write()
                .map_err(|_| anyhow::anyhow!("settings lock poisoned"))?;
            let changed = guard.apply(&patch);
            (guard.clone(), changed)
        };
        if changed {
            self.save_settings(&next)?;
            if patch.indexing.is_some() || patch.privacy.is_some() {
                if next.indexing.paused {
                    self.paused.store(true, Ordering::SeqCst);
                    self.set_state(STATE_PAUSED);
                } else {
                    self.paused.store(false, Ordering::SeqCst);
                    self.force_rescan.store(false, Ordering::SeqCst);
                }
            }
        }
        Ok(next)
    }

    pub fn activity(&self, query: ActivityQuery) -> Vec<ActivityItem> {
        self.db.activity(&query).unwrap_or_default()
    }

    pub fn clear_activity(&self) -> anyhow::Result<u64> {
        self.db.clear_activity()
    }

    pub fn onboarding(&self) -> OnboardingState {
        let settings = self.settings_snapshot();
        OnboardingState {
            complete: settings.onboarding_complete,
            legacy: self.legacy_summary(),
            suggested_roots: suggested_roots(),
            defaults: settings,
        }
    }

    pub fn complete_onboarding(&self, input: OnboardingInput) -> anyhow::Result<Settings> {
        let mut settings = self.settings_snapshot();
        settings.user_name = input
            .user_name
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
            .map(|name| name.chars().take(40).collect());
        if !input.hotkey.trim().is_empty() {
            settings.hotkey = input.hotkey.trim().to_string();
        }
        settings.indexing.roots = input
            .roots
            .into_iter()
            .filter(|root| !root.path.trim().is_empty())
            .map(|root| RootConfig {
                path: root.path.trim().to_string(),
                mode: root.mode,
                enabled: root.enabled,
            })
            .collect();
        settings.indexing.content_enabled = input.content_enabled;
        settings.indexing.ocr_enabled = input.ocr_enabled || (input.skipped && false);
        settings.agent.enabled = input.agent_opt_in;
        settings.onboarding_complete = true;
        settings.privacy.telemetry = false;
        self.write_settings(&settings)?;
        self.force_rescan.store(true, Ordering::SeqCst);
        self.set_state(STATE_STARTING);
        Ok(settings)
    }

    pub fn execute_action(&self, action: ActionSpec) -> OpenOutcome {
        let settings = self.settings_snapshot();
        let (kind, title, path) = describe_action(&action);
        // Server-side authorisation for destructive commands: the WebView (or any
        // other caller of the invoke surface) cannot one-shot power off the PC.
        if let ActionSpec::RunNative { command } = &action {
            if is_destructive(*command) {
                let pending = self
                    .pending_destructive
                    .lock()
                    .ok()
                    .and_then(|guard| *guard);
                let confirmed = pending
                    .map(|(pending_command, at)| pending_command == *command && now_ms() - at < 15_000)
                    .unwrap_or(false);
                if !confirmed {
                    if let Ok(mut guard) = self.pending_destructive.lock() {
                        *guard = Some((*command, now_ms()));
                    }
                    return OpenOutcome {
                        ok: false,
                        message: format!(
                            "Confirmation required for {command:?}. Ask again to execute."
                        ),
                    };
                }
                if let Ok(mut guard) = self.pending_destructive.lock() {
                    *guard = None;
                }
            }
        }
        let outcome = match &action {
            ActionSpec::RunNative { command } => match command {
                NativeCommand::PauseIndexing => {
                    let _ = self.index_command(IndexCommand::Pause);
                    OpenOutcome {
                        ok: true,
                        message: "Indexing paused".into(),
                    }
                }
                NativeCommand::ResumeIndexing => {
                    let _ = self.index_command(IndexCommand::Resume);
                    OpenOutcome {
                        ok: true,
                        message: "Indexing resumed".into(),
                    }
                }
                other => ps_sources::actions::run_native(*other),
            },
            _ => ps_sources::actions::execute(&action),
        };
        if outcome.ok && settings.privacy.activity_history {
            let _ = self.db.log_activity(
                kind,
                &title,
                Some(&outcome.message),
                path.as_deref(),
            );
        }
        outcome
    }

    /// Execute a previously requested destructive command after the user
    /// confirmed it. Only the command that was requested within the last 15 s
    /// is accepted; everything else is rejected.
    pub fn confirm_destructive(&self, command: NativeCommand) -> OpenOutcome {
        if !is_destructive(command) {
            return OpenOutcome {
                ok: false,
                message: "That action does not require confirmation.".into(),
            };
        }
        let accepted = self
            .pending_destructive
            .lock()
            .ok()
            .and_then(|mut guard| guard.take())
            .map(|(pending, at)| pending == command && now_ms() - at < 15_000)
            .unwrap_or(false);
        if !accepted {
            return OpenOutcome {
                ok: false,
                message: "Confirmation expired or did not match. Ask again.".into(),
            };
        }
        let outcome = ps_sources::actions::run_native(command);
        if outcome.ok {
            let settings = self.settings_snapshot();
            if settings.privacy.activity_history {
                let _ = self.db.log_activity(
                    ActivityKind::Workflow,
                    &format!("{command:?}"),
                    Some(&outcome.message),
                    None,
                );
            }
        }
        outcome
    }

    pub fn index_command(&self, command: IndexCommand) -> anyhow::Result<()> {
        match command {
            IndexCommand::Pause => {
                self.paused.store(true, Ordering::SeqCst);
                self.set_state(STATE_PAUSED);
            }
            IndexCommand::Resume => {
                self.paused.store(false, Ordering::SeqCst);
                self.set_state(STATE_INDEXING);
                // The Tauri shell restarts indexing on resume (spawn_background).
            }
            IndexCommand::Rebuild => {
                *self.index.write().unwrap() = FilenameIndex::new();
                self.filename_entries.store(0, Ordering::SeqCst);
                let path = self.config.data_dir.join(INDEX_FILE);
                let _ = std::fs::remove_file(path);
                self.force_rescan.store(true, Ordering::SeqCst);
                self.set_state(STATE_STARTING);
            }
            IndexCommand::RebuildContent => {
                let _ = self.with_content(|content| content.clear());
                self.content_documents.store(0, Ordering::SeqCst);
                self.force_rescan.store(true, Ordering::SeqCst);
            }
            IndexCommand::RebuildOcr => {
                self.db.ocr_clear()?;
                self.ocr_documents.store(0, Ordering::SeqCst);
            }
            IndexCommand::Shutdown => {
                self.shutdown.store(true, Ordering::SeqCst);
                if let Ok(mut guard) = self.background.lock() {
                    if let Some(handle) = guard.take() {
                        let _ = handle.join();
                    }
                }
            }
        }
        self.last_updated.store(now_ms(), Ordering::SeqCst);
        Ok(())
    }

    // -- internals ---------------------------------------------------------

    fn settings_snapshot(&self) -> Settings {
        self.settings
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    fn save_settings(&self, settings: &Settings) -> anyhow::Result<()> {
        self.write_settings(settings)
    }

    fn write_settings(&self, settings: &Settings) -> anyhow::Result<()> {
        crate::settings::save(&self.config.data_dir.join("settings.json"), settings)?;
        if let Ok(mut guard) = self.settings.write() {
            *guard = settings.clone();
        }
        Ok(())
    }

    /// Run a closure with the content index, if it is available. The lock is
    /// held only for the duration of the call (search/extraction are bounded).
    fn with_content<R>(&self, f: impl FnOnce(&ps_content::ContentIndex) -> R) -> Option<R> {
        let guard = self.content.lock().ok()?;
        guard.as_ref().map(f)
    }

    fn set_state(&self, state: u8) {
        self.state.store(state, Ordering::SeqCst);
    }

    fn set_phase(&self, phase: &str) {
        if let Ok(mut guard) = self.phase.lock() {
            *guard = phase.to_string();
        }
    }

    fn push_warning(&self, warning: String) {
        if let Ok(mut guard) = self.warnings.lock() {
            if guard.len() >= MAX_WARNINGS {
                guard.remove(0);
            }
            if !guard.contains(&warning) {
                guard.push(warning);
            }
        }
    }

    fn save_index(&self) {
        let index = self.index.read().unwrap();
        if let Err(err) = index.save(&self.config.data_dir.join(INDEX_FILE)) {
            drop(index);
            self.push_warning(format!("could not save filename index: {err}"));
        }
    }

    fn configured_roots(&self) -> Vec<PathBuf> {
        self.settings_snapshot()
            .indexing
            .roots
            .into_iter()
            .filter(|root| root.enabled && !root.path.trim().is_empty())
            .map(|root| PathBuf::from(root.path))
            .collect()
    }

    fn legacy_summary(&self) -> LegacySummary {
        if let Ok(guard) = self.legacy_summary.lock() {
            if let Some(summary) = guard.as_ref() {
                return summary.clone();
            }
        }
        let summary = self
            .config
            .legacy_db
            .as_deref()
            .map(read_legacy_summary)
            .unwrap_or_default();
        if let Ok(mut guard) = self.legacy_summary.lock() {
            *guard = Some(summary.clone());
        }
        summary
    }

    fn migrate_legacy(&self, legacy: &Path) {
        let settings = self.settings_snapshot();
        if settings.legacy_migrated {
            return;
        }
        self.set_phase("migrating");
        let db_path = self.config.data_dir.join(DB_FILE);
        let options = crate::migration::MigrationOptions {
            import_clipboard: false,
            import_ai_keys: false,
            dry_run: false,
        };
        match crate::migration::migrate_legacy_db(legacy, &db_path, &options) {
            Ok(report) => {
                self.push_warning(format!(
                    "imported legacy index: {} files ({} folders), {} documents, {} browser items, {} quicklinks, {} snippets ({} ms)",
                    report.files,
                    report.dirs,
                    report.content_docs,
                    report.browser_items,
                    report.quicklinks,
                    report.snippets,
                    report.duration_ms
                ));
                for warning in report.warnings {
                    self.push_warning(warning);
                }
                if !report.source_unchanged {
                    self.push_warning(
                        "legacy database changed during migration - the old index must be restored from backup"
                            .to_string(),
                    );
                }
                if let Ok(entries) = load_migrated_files(&db_path) {
                    if !entries.is_empty() {
                        let mut index = self.index.write().unwrap();
                        index.replace_all(entries);
                    }
                }
                let documents = self
                    .with_content(|content| content.stats().documents)
                    .unwrap_or(0);
                self.content_documents.store(documents, Ordering::SeqCst);
            }
            Err(err) => {
                if std::env::var_os("PROTONSEARCH_VERBOSE").is_some() {
                    eprintln!("[migration] legacy import failed: {err:#}");
                }
                self.push_warning(format!("legacy import failed: {err}"));
                // Keep legacy_migrated=false so the next launch can retry.
                return;
            }
        }
        let mut next = self.settings_snapshot();
        next.legacy_migrated = true;
        let _ = self.write_settings(&next);
    }
}

// ---------------------------------------------------------------------------
// Background pipeline
// ---------------------------------------------------------------------------

fn run_background(engine: Arc<Engine>) {
    engine.set_state(STATE_INDEXING);
    engine.set_phase("preparing");
    engine.last_updated.store(now_ms(), Ordering::SeqCst);

    if let Some(legacy) = engine.config.legacy_db.clone() {
        if legacy.is_file() {
            engine.migrate_legacy(&legacy);
        }
    }

    if engine.shutdown.load(Ordering::SeqCst) {
        return;
    }
    engine.set_phase("sources");
    {
        let settings = engine.settings_snapshot();
        let legacy = engine.config.legacy_db.clone();
        let (sources, warnings) = ps_sources::Sources::load(&settings, legacy.as_deref());
        if let Ok(mut guard) = engine.sources.write() {
            *guard = sources;
        }
        for warning in warnings {
            engine.push_warning(warning);
        }
    }

    if engine.shutdown.load(Ordering::SeqCst) {
        return;
    }

    let roots = engine.configured_roots();
    if !roots.is_empty() {
        let current_roots: Vec<String> = roots
            .iter()
            .map(|root| ps_index::normalize_root(root))
            .collect();
        let rescan = engine.force_rescan.swap(false, Ordering::SeqCst)
            || {
                let index = engine.index.read().unwrap();
                index.is_empty() || index.roots() != current_roots.as_slice()
            };
        if rescan {
            engine.set_phase("scanning files");
            scan_into_index(&engine, &roots);
            engine.save_index();
        }
        engine.set_phase("indexing content");
        index_content(&engine, &roots);
        engine.set_phase("indexing screenshots");
        index_ocr(&engine, &roots);
    }

    engine.set_phase("ready");
    engine.set_state(if engine.paused.load(Ordering::SeqCst) {
        STATE_PAUSED
    } else {
        STATE_READY
    });
    engine.last_updated.store(now_ms(), Ordering::SeqCst);
}

fn scan_into_index(engine: &Arc<Engine>, roots: &[PathBuf]) {
    let settings = engine.settings_snapshot();
    let excluded = settings.indexing.excluded_paths.clone();
    let junk: Vec<String> = Vec::new();
    let cancelled = || engine.shutdown.load(Ordering::SeqCst);
    let counter = Arc::clone(engine);
    let mut progress = move |count: usize| {
        counter.filename_entries.store(count as u64, Ordering::Relaxed);
    };
    let options = ps_index::ScanOptions {
        roots,
        excluded: &excluded,
        junk_dirs: &junk,
        max_entries: None,
        cancelled: &cancelled,
        progress: &mut progress,
    };
    let (mut index, stats) = ps_index::scan(options);
    let root_strings: Vec<String> = roots
        .iter()
        .map(|root| ps_index::normalize_root(root))
        .collect();
    index.set_roots(root_strings);
    engine.filename_entries.store(index.len() as u64, Ordering::Relaxed);
    if stats.errors > 0 {
        engine.push_warning(format!("{} directories or files could not be read", stats.errors));
    }
    *engine.index.write().unwrap() = index;
}

fn index_content(engine: &Arc<Engine>, roots: &[PathBuf]) {
    let settings = engine.settings_snapshot();
    if !settings.indexing.content_enabled {
        return;
    }
    let modes = root_modes(&settings);
    let candidates: Vec<PathBuf> = {
        let index = engine.index.read().unwrap();
        index
            .entries()
            .iter()
            .filter(|entry| entry.kind == EntryKind::File)
            .filter(|entry| entry.size > 0 && entry.size <= 32 * 1024 * 1024)
            .filter(|entry| ps_content::is_supported(Path::new(&entry.path)))
            .filter(|entry| mode_for(&modes, &entry.path) >= 1)
            .map(|entry| PathBuf::from(&entry.path))
            .take(MAX_CONTENT_PER_PASS)
            .collect()
    };
    if candidates.is_empty() {
        return;
    }
    if engine.with_content(|_| ()).is_none() {
        return;
    }
    let mut written = 0usize;
    for path in candidates {
        if engine.shutdown.load(Ordering::SeqCst) || engine.paused.load(Ordering::SeqCst) {
            break;
        }
        match engine.with_content(|content| content.index_file(&path, ps_content::DEFAULT_MAX_BYTES)) {
            Some(Ok(true)) => written += 1,
            Some(Ok(false)) => {}
            Some(Err(err)) => engine
                .push_warning(format!("content extraction failed for {}: {err}", path.display())),
            None => break,
        }
    }
    let documents = engine
        .with_content(|content| content.stats().documents)
        .unwrap_or(0);
    engine.content_documents.store(documents, Ordering::SeqCst);
    let _ = roots;
    engine.last_updated.store(now_ms(), Ordering::SeqCst);
    let _ = written;
}

fn index_ocr(engine: &Arc<Engine>, roots: &[PathBuf]) {
    let settings = engine.settings_snapshot();
    if !settings.indexing.ocr_enabled || !ps_ocr::available() {
        return;
    }
    let modes = root_modes(&settings);
    let candidates: Vec<(String, u64, i64)> = {
        let index = engine.index.read().unwrap();
        index
            .entries()
            .iter()
            .filter(|entry| entry.kind == EntryKind::File)
            .filter(|entry| {
                ps_ocr::IMAGE_EXTENSIONS.contains(&entry.ext.to_ascii_lowercase().as_str())
            })
            .filter(|entry| mode_for(&modes, &entry.path) >= 2)
            .map(|entry| (entry.path.clone(), entry.size, entry.modified))
            .take(MAX_OCR_PER_PASS)
            .collect()
    };
    let mut indexed = 0usize;
    for (path, size, modified) in candidates {
        if engine.shutdown.load(Ordering::SeqCst) || engine.paused.load(Ordering::SeqCst) {
            break;
        }
        let hash = quick_hash(&path, size, modified);
        if engine.db.ocr_has(&path, &hash) {
            continue;
        }
        match ps_ocr::extract_image(Path::new(&path), 16_777_216) {
            Ok(Some(document)) if !document.text.trim().is_empty() => {
                let _ = engine.db.ocr_upsert(
                    &path,
                    &hash,
                    &document.text,
                    &document.language,
                    document.width,
                    document.height,
                );
                indexed += 1;
            }
            Ok(_) => {}
            Err(err) => engine.push_warning(format!("OCR failed for {path}: {err}")),
        }
    }
    engine
        .ocr_documents
        .store(engine.db.ocr_count(), Ordering::SeqCst);
    engine.last_updated.store(now_ms(), Ordering::SeqCst);
    let _ = roots;
    let _ = indexed;
}

fn quick_hash(path: &str, size: u64, modified: i64) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    size.hash(&mut hasher);
    modified.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// 0 = names only, 1 = content, 2 = content + OCR for a path under a root.
fn root_modes(settings: &Settings) -> Vec<(String, u8)> {
    settings
        .indexing
        .roots
        .iter()
        .filter(|root| root.enabled)
        .map(|root| {
            let mut normalized = root.path.trim_end_matches(['\\', '/']).to_lowercase();
            normalized.push('\\');
            let level = match root.mode {
                RootMode::NamesOnly => 0,
                RootMode::Content => 1,
                RootMode::ContentAndOcr => 2,
            };
            (normalized, level)
        })
        .collect()
}

fn mode_for(modes: &[(String, u8)], path: &str) -> u8 {
    let lower = path.to_lowercase();
    modes
        .iter()
        .filter(|(root, _)| lower.starts_with(root.as_str()))
        .map(|(_, level)| *level)
        .max()
        .unwrap_or(0)
}

fn load_migrated_files(db_path: &Path) -> anyhow::Result<Vec<ps_index::IndexEntry>> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let has_table: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='files'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false);
    if !has_table {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare(
        "SELECT path, name, extension, modified, size, is_dir FROM files ORDER BY path LIMIT 500000",
    )?;
    let rows = stmt.query_map([], |row| {
        let is_dir: i64 = row.get(5)?;
        Ok(ps_index::IndexEntry {
            path: row.get(0)?,
            name: row.get(1)?,
            ext: row.get(2)?,
            kind: if is_dir != 0 {
                EntryKind::Folder
            } else {
                EntryKind::File
            },
            size: row.get::<_, i64>(4)? as u64,
            modified: row.get(3)?,
        })
    })?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

fn read_legacy_summary(path: &Path) -> LegacySummary {
    let mut summary = LegacySummary {
        detected: path.is_file(),
        app_data_path: path.parent().map(|parent| parent.display().to_string()),
        ..LegacySummary::default()
    };
    if !summary.detected {
        return summary;
    }
    let Ok(conn) = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) else {
        return summary;
    };
    let count = |table: &str| -> u64 {
        conn.query_row(
            &format!("SELECT COUNT(*) FROM {table}"),
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as u64
    };
    let exists = |table: &str| -> bool {
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type IN ('table','view') AND name = ?1",
            [table],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false)
    };
    if exists("files") {
        summary.indexed_files = count("files");
    }
    if exists("files_fts") {
        summary.content_documents = count("files_fts");
    }
    if exists("browser_items") {
        summary.browser_items = count("browser_items");
    }
    if exists("quicklinks") {
        summary.quicklinks = count("quicklinks");
    }
    if exists("snippets") {
        summary.snippets = count("snippets");
    }
    if exists("ai_settings") {
        summary.settings = count("ai_settings");
    }
    summary
}

// ---------------------------------------------------------------------------
// Query helpers
// ---------------------------------------------------------------------------

/// Strips a leading scope prefix and returns the remaining free text.
fn effective_query(raw: &str, fallback: SearchScope) -> (SearchScope, String) {
    let trimmed = raw.trim_start();
    let lower = trimmed.to_lowercase();
    const PREFIXES: [(&str, SearchScope); 14] = [
        ("file:", SearchScope::Files),
        ("files:", SearchScope::Files),
        ("folder:", SearchScope::Folders),
        ("content:", SearchScope::Content),
        ("app:", SearchScope::Apps),
        ("apps:", SearchScope::Apps),
        ("clip:", SearchScope::Clipboard),
        ("clipboard:", SearchScope::Clipboard),
        ("history:", SearchScope::Browser),
        ("bookmarks:", SearchScope::Browser),
        ("git:", SearchScope::Git),
        ("commits:", SearchScope::Git),
        ("todos:", SearchScope::Git),
        ("ocr:", SearchScope::Ocr),
    ];
    for (prefix, scope) in PREFIXES {
        if lower.starts_with(prefix) {
            return (scope, trimmed[prefix.len()..].trim().to_string());
        }
    }
    (fallback, trimmed.to_string())
}

fn scope_allows(scope: SearchScope, kind: SourceKind) -> bool {
    if scope == SearchScope::All {
        return true;
    }
    matches!(
        (scope, kind),
        (SearchScope::Files, SourceKind::File)
            | (SearchScope::Files, SourceKind::Folder)
            | (SearchScope::Folders, SourceKind::Folder)
            | (SearchScope::Apps, SourceKind::Application)
            | (SearchScope::Clipboard, SourceKind::Clipboard)
            | (SearchScope::Browser, SourceKind::BrowserHistory)
            | (SearchScope::Browser, SourceKind::Bookmark)
            | (SearchScope::Git, SourceKind::GitCommit)
            | (SearchScope::Git, SourceKind::GitRepository)
            | (SearchScope::Git, SourceKind::GitTodo)
            | (SearchScope::Ocr, SourceKind::OcrText)
            | (SearchScope::Actions, SourceKind::Action)
            | (SearchScope::Actions, SourceKind::Command)
            | (SearchScope::Commands, SourceKind::Command)
            | (SearchScope::Content, SourceKind::ContentMatch)
    )
}

fn content_scope_allows(scope: SearchScope) -> bool {
    matches!(
        scope,
        SearchScope::All | SearchScope::Content | SearchScope::Files
    )
}

/// Delegate ordering, dedupe and truncation to the shared ranker (engine-core)
/// so every tier obeys the same deterministic contract.
fn finalize(ranked: Vec<Ranked>, limit: usize) -> Vec<SearchResult> {
    let items: Vec<crate::rank::RankedResult> = ranked
        .into_iter()
        .map(|item| crate::rank::RankedResult {
            tier: item.tier,
            score: item.score,
            priority: item.priority,
            result: item.result,
        })
        .collect();
    crate::rank::merge_results(items, limit)
}

/// Merge UI-supplied filters with filters parsed from the query text; both are
/// restrictive, so a field present in either applies.
fn merged_filters(parsed: &SearchFilters, request: &SearchFilters) -> SearchFilters {
    let mut merged = request.clone();
    if !parsed.kinds.is_empty() {
        merged.kinds = parsed.kinds.clone();
    }
    if !parsed.extensions.is_empty() {
        merged.extensions = parsed.extensions.clone();
    }
    if parsed.under_path.is_some() {
        merged.under_path = parsed.under_path.clone();
    }
    merged.modified_after = parsed.modified_after.or(request.modified_after);
    merged.modified_before = parsed.modified_before.or(request.modified_before);
    merged.min_size = parsed.min_size.or(request.min_size);
    merged.max_size = parsed.max_size.or(request.max_size);
    merged
}

fn match_apps<'a>(
    apps: &'a [ps_sources::AppEntry],
    terms: &[String],
) -> Vec<(&'a ps_sources::AppEntry, f32)> {
    let mut hits: Vec<(&ps_sources::AppEntry, f32)> = apps
        .iter()
        .filter_map(|app| score_text(&app.name, terms).map(|score| (app, score)))
        .collect();
    hits.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.name.cmp(&b.0.name))
    });
    hits
}

fn match_clipboard<'a>(
    items: &'a [ps_sources::ClipboardEntry],
    terms: &[String],
) -> Vec<&'a ps_sources::ClipboardEntry> {
    let mut hits: Vec<(&ps_sources::ClipboardEntry, i64)> = items
        .iter()
        .filter(|item| terms.iter().all(|term| item.text.to_lowercase().contains(term)))
        .map(|item| (item, item.timestamp_ms))
        .collect();
    hits.sort_by(|a, b| b.1.cmp(&a.1));
    hits.into_iter().map(|(item, _)| item).collect()
}

fn match_browser<'a>(
    items: &'a [ps_sources::BrowserEntry],
    terms: &[String],
) -> Vec<&'a ps_sources::BrowserEntry> {
    let mut hits: Vec<(&ps_sources::BrowserEntry, i64)> = items
        .iter()
        .filter(|item| {
            let haystack = format!("{} {}", item.title.to_lowercase(), item.url.to_lowercase());
            terms.iter().all(|term| haystack.contains(term))
        })
        .map(|item| (item, item.last_visit_ms))
        .collect();
    hits.sort_by(|a, b| b.1.cmp(&a.1));
    hits.into_iter().map(|(item, _)| item).collect()
}

fn match_git<'a>(
    entries: &'a [ps_sources::GitEntry],
    terms: &[String],
) -> Vec<&'a ps_sources::GitEntry> {
    entries
        .iter()
        .filter(|entry| {
            let haystack = match entry {
                ps_sources::GitEntry::Repo { name, path, .. } => {
                    format!("{} {}", name.to_lowercase(), path.to_lowercase())
                }
                ps_sources::GitEntry::Commit { message, author, hash, .. } => {
                    format!(
                        "{} {} {}",
                        message.to_lowercase(),
                        author.to_lowercase(),
                        hash.to_lowercase()
                    )
                }
                ps_sources::GitEntry::Todo { text, path, .. } => {
                    format!("{} {}", text.to_lowercase(), path.to_lowercase())
                }
            };
            terms.iter().all(|term| haystack.contains(term))
        })
        .collect()
}

fn match_workflows<'a>(
    workflows: &'a [ps_sources::WorkflowEntry],
    terms: &[String],
) -> Vec<&'a ps_sources::WorkflowEntry> {
    workflows
        .iter()
        .filter(|workflow| {
            let haystack = format!(
                "{} {}",
                workflow.name.to_lowercase(),
                workflow.keyword.to_lowercase()
            );
            terms.iter().all(|term| haystack.contains(term))
        })
        .collect()
}

/// Score a display text against query terms: exact 1000, prefix 900, word
/// prefix 800, substring 700, else None. Shorter text wins ties.
fn score_text(text: &str, terms: &[String]) -> Option<f32> {
    if terms.is_empty() {
        return None;
    }
    let lower = text.to_lowercase();
    if terms.len() == 1 {
        let term = &terms[0];
        if lower == *term {
            return Some(1000.0);
        }
        if lower.starts_with(term.as_str()) {
            return Some(950.0 - text.len() as f32 * 0.1);
        }
        if lower
            .split(|c: char| !c.is_alphanumeric())
            .any(|word| word.starts_with(term.as_str()))
        {
            return Some(900.0 - text.len() as f32 * 0.1);
        }
        if lower.contains(term.as_str()) {
            return Some(820.0 - text.len() as f32 * 0.1);
        }
        return None;
    }
    if terms.iter().all(|term| lower.contains(term.as_str())) {
        Some(700.0 - text.len() as f32 * 0.1)
    } else {
        None
    }
}

fn display_path(path: &str, settings: &Settings) -> String {
    let mut best_relative: Option<String> = None;
    for root in &settings.indexing.roots {
        let trimmed = root.path.trim_end_matches(['\\', '/']);
        if let Some(rest) = path.strip_prefix(trimmed) {
            let rest = rest.trim_start_matches(['\\', '/']);
            let candidate = rest.to_string();
            if best_relative
                .as_ref()
                .map(|current| candidate.len() < current.len())
                .unwrap_or(true)
            {
                best_relative = Some(candidate);
            }
        }
    }
    let relative = best_relative.unwrap_or_else(|| path.to_string());
    let parts: Vec<&str> = relative
        .split(['\\', '/'])
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() <= 1 {
        return relative.replace(['\\', '/'], " / ");
    }
    parts[..parts.len() - 1].join(" / ")
}

fn file_result(
    index: &FilenameIndex,
    hit: &ps_index::FileHit,
    settings: &Settings,
) -> Option<(SearchResult, u8)> {
    let position = hit.entry_index;
    if position >= index.len() {
        return None;
    }
    let path = index.entry_path(position).into_owned();
    let name = index.entry_name(position).to_string();
    let kind = index.entry_kind(position);
    let ext = index.entry_ext(position).to_string();
    let is_folder = kind == EntryKind::Folder;
    let result_kind = if is_folder {
        SourceKind::Folder
    } else {
        SourceKind::File
    };
    let icon = if is_folder {
        IconRef::Kind {
            kind: SourceKind::Folder,
        }
    } else {
        IconRef::FileType {
            ext: if ext.is_empty() {
                "file".to_string()
            } else {
                ext.clone()
            },
        }
    };
    let actions = vec![
        ActionItem {
            id: "reveal".into(),
            label: if is_folder {
                "Open in Explorer".into()
            } else {
                "Reveal in Explorer".into()
            },
            action: ActionSpec::RevealPath { path: path.clone() },
            destructive: false,
            shortcut: None,
        },
        ActionItem {
            id: "copy-path".into(),
            label: "Copy path".into(),
            action: ActionSpec::CopyText { text: path.clone() },
            destructive: false,
            shortcut: Some("Ctrl+C".into()),
        },
    ];
    let tier = if hit.score >= 900.0 { 0 } else { 1 };
    Some((
        SearchResult {
            id: format!("file:{}", path.to_lowercase()),
            kind: result_kind,
            title: name,
            subtitle: display_path(&path, settings),
            path: Some(path.clone()),
            icon,
            score: hit.score,
            match_ranges: hit
                .ranges
                .iter()
                .map(|(start, end)| TextRange {
                    start: *start,
                    end: *end,
                })
                .collect(),
            meta: ResultMeta {
                size: if is_folder {
                    None
                } else {
                    Some(index.entry_size(position))
                },
                modified: Some(index.entry_modified(position)),
                ..ResultMeta::default()
            },
            action: ActionSpec::OpenPath {
                path,
                line: None,
                column: None,
            },
            actions,
        },
        tier,
    ))
}

fn app_result(app: &ps_sources::AppEntry) -> SearchResult {
    SearchResult {
        id: format!("app:{}", app.id),
        kind: SourceKind::Application,
        title: app.name.clone(),
        subtitle: "Application".into(),
        path: Some(app.target.clone()),
        icon: app
            .icon_path
            .as_ref()
            .map(|path| IconRef::AppIcon { path: path.clone() })
            .unwrap_or(IconRef::Monogram),
        score: 0.0,
        match_ranges: Vec::new(),
        meta: ResultMeta {
            label: Some("App".into()),
            ..ResultMeta::default()
        },
        action: ActionSpec::LaunchApp {
            app_id: app.target.clone(),
            target: None,
        },
        actions: Vec::new(),
    }
}

fn recent_result(entry: &ps_sources::RecentEntry) -> SearchResult {
    let ext = Path::new(&entry.path)
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    SearchResult {
        id: format!("recent:{}", entry.path.to_lowercase()),
        kind: SourceKind::Recent,
        title: entry.name.clone(),
        subtitle: "Recently used".into(),
        path: Some(entry.path.clone()),
        icon: IconRef::FileType { ext },
        score: 0.0,
        match_ranges: Vec::new(),
        meta: ResultMeta {
            modified: Some(entry.last_used_ms),
            ..ResultMeta::default()
        },
        action: ActionSpec::OpenPath {
            path: entry.path.clone(),
            line: None,
            column: None,
        },
        actions: Vec::new(),
    }
}

fn command_result(action: &catalog::CatalogAction) -> SearchResult {
    SearchResult {
        id: action.id.to_string(),
        kind: SourceKind::Command,
        title: action.title.to_string(),
        subtitle: action.subtitle.to_string(),
        path: None,
        icon: IconRef::Kind {
            kind: SourceKind::Command,
        },
        score: 0.0,
        match_ranges: Vec::new(),
        meta: ResultMeta {
            label: Some("Action".into()),
            ..ResultMeta::default()
        },
        action: ActionSpec::RunNative {
            command: action.command,
        },
        actions: if action.destructive {
            vec![ActionItem {
                id: "confirm".into(),
                label: "Runs after confirmation".into(),
                action: ActionSpec::RunNative {
                    command: action.command,
                },
                destructive: true,
                shortcut: None,
            }]
        } else {
            Vec::new()
        },
    }
}

fn workflow_result(workflow: &ps_sources::WorkflowEntry) -> SearchResult {
    let action = match workflow.kind {
        ps_sources::WorkflowKind::Snippet => ActionSpec::CopyText {
            text: workflow.payload.clone(),
        },
        ps_sources::WorkflowKind::Quicklink => ActionSpec::OpenUrl {
            url: workflow.payload.clone(),
        },
    };
    SearchResult {
        id: format!("workflow:{}", workflow.id),
        kind: match workflow.kind {
            ps_sources::WorkflowKind::Snippet => SourceKind::Snippet,
            ps_sources::WorkflowKind::Quicklink => SourceKind::Quicklink,
        },
        title: workflow.name.clone(),
        subtitle: format!("Keyword: {}", workflow.keyword),
        path: None,
        icon: IconRef::Kind {
            kind: SourceKind::Quicklink,
        },
        score: 0.0,
        match_ranges: Vec::new(),
        meta: ResultMeta {
            label: Some("Workflow".into()),
            ..ResultMeta::default()
        },
        action,
        actions: Vec::new(),
    }
}

fn clipboard_result(entry: &ps_sources::ClipboardEntry) -> SearchResult {
    let first_line = entry.text.lines().next().unwrap_or("").trim();
    let title: String = first_line.chars().take(120).collect();
    SearchResult {
        id: format!("clip:{}", entry.id),
        kind: SourceKind::Clipboard,
        title: if title.is_empty() {
            "Clipboard item".to_string()
        } else {
            title
        },
        subtitle: if entry.source_app.is_empty() {
            "Clipboard history".into()
        } else {
            format!("Clipboard - {}", entry.source_app)
        },
        path: None,
        icon: IconRef::Monogram,
        score: 0.0,
        match_ranges: Vec::new(),
        meta: ResultMeta {
            modified: Some(entry.timestamp_ms),
            label: if entry.pinned {
                Some("Pinned".into())
            } else {
                Some("Clipboard".into())
            },
            ..ResultMeta::default()
        },
        action: ActionSpec::CopyText {
            text: entry.text.clone(),
        },
        actions: Vec::new(),
    }
}

fn browser_result(entry: &ps_sources::BrowserEntry) -> SearchResult {
    let is_url = entry.url.starts_with("http://") || entry.url.starts_with("https://");
    SearchResult {
        id: format!("browser:{}", entry.url.to_lowercase()),
        kind: if entry.bookmark {
            SourceKind::Bookmark
        } else {
            SourceKind::BrowserHistory
        },
        title: if entry.title.trim().is_empty() {
            entry.url.clone()
        } else {
            entry.title.clone()
        },
        subtitle: entry.url.clone(),
        path: Some(entry.url.clone()),
        icon: IconRef::Kind {
            kind: SourceKind::Bookmark,
        },
        score: 0.0,
        match_ranges: Vec::new(),
        meta: ResultMeta {
            modified: Some(entry.last_visit_ms),
            label: Some(if entry.bookmark {
                "Bookmark".into()
            } else {
                format!("{} visits", entry.visit_count.max(1))
            }),
            ..ResultMeta::default()
        },
        action: if is_url {
            ActionSpec::OpenUrl {
                url: entry.url.clone(),
            }
        } else {
            ActionSpec::CopyText {
                text: entry.url.clone(),
            }
        },
        actions: Vec::new(),
    }
}

fn git_result(entry: &ps_sources::GitEntry) -> SearchResult {
    match entry {
        ps_sources::GitEntry::Repo { path, name, branch } => SearchResult {
            id: format!("git:repo:{}", path.to_lowercase()),
            kind: SourceKind::GitRepository,
            title: name.clone(),
            subtitle: branch
                .as_ref()
                .map(|branch| format!("Repository - {branch}"))
                .unwrap_or_else(|| "Repository".into()),
            path: Some(path.clone()),
            icon: IconRef::Kind {
                kind: SourceKind::GitRepository,
            },
            score: 0.0,
            match_ranges: Vec::new(),
            meta: ResultMeta::default(),
            action: ActionSpec::OpenPath {
                path: path.clone(),
                line: None,
                column: None,
            },
            actions: Vec::new(),
        },
        ps_sources::GitEntry::Commit {
            repo,
            hash,
            message,
            author,
            date_ms,
        } => SearchResult {
            id: format!("git:commit:{}", hash),
            kind: SourceKind::GitCommit,
            title: message.lines().next().unwrap_or(message).to_string(),
            subtitle: format!(
                "{} - {} - {}",
                author,
                hash.chars().take(8).collect::<String>(),
                repo
            ),
            path: Some(repo.clone()),
            icon: IconRef::Kind {
                kind: SourceKind::GitCommit,
            },
            score: 0.0,
            match_ranges: Vec::new(),
            meta: ResultMeta {
                modified: Some(*date_ms),
                ..ResultMeta::default()
            },
            action: ActionSpec::CopyText {
                text: hash.clone(),
            },
            actions: Vec::new(),
        },
        ps_sources::GitEntry::Todo {
            repo,
            path,
            line,
            text,
        } => SearchResult {
            id: format!("git:todo:{}:{}", path, line),
            kind: SourceKind::GitTodo,
            title: text.clone(),
            subtitle: format!("{}:{} - {}", path, line, repo),
            path: Some(path.clone()),
            icon: IconRef::FileType {
                ext: Path::new(path)
                    .extension()
                    .map(|ext| ext.to_string_lossy().to_lowercase())
                    .unwrap_or_default(),
            },
            score: 0.0,
            match_ranges: Vec::new(),
            meta: ResultMeta {
                line: Some(*line),
                ..ResultMeta::default()
            },
            action: ActionSpec::OpenPath {
                path: path.clone(),
                line: Some(*line),
                column: None,
            },
            actions: Vec::new(),
        },
    }
}

fn content_result(hit: ps_content::ContentHit, settings: &Settings) -> SearchResult {
    let ext = Path::new(&hit.path)
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    SearchResult {
        id: format!("content:{}", hit.path.to_lowercase()),
        kind: SourceKind::ContentMatch,
        title: hit.title,
        subtitle: display_path(&hit.path, settings),
        path: Some(hit.path.clone()),
        icon: IconRef::FileType { ext },
        score: 0.0,
        match_ranges: Vec::new(),
        meta: ResultMeta {
            label: hit
                .snippet_line
                .map(|line| format!("line {line}"))
                .or_else(|| Some("In document".into())),
            line: hit.snippet_line,
            snippet: Some(hit.snippet),
            ..ResultMeta::default()
        },
        action: ActionSpec::OpenPath {
            path: hit.path,
            line: hit.snippet_line,
            column: None,
        },
        actions: Vec::new(),
    }
}

fn ocr_result(hit: crate::db::OcrHit, settings: &Settings) -> SearchResult {
    let ext = Path::new(&hit.path)
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let first_line: String = hit.text.lines().next().unwrap_or("").chars().take(120).collect();
    SearchResult {
        id: format!("ocr:{}", hit.path.to_lowercase()),
        kind: SourceKind::OcrText,
        title: if first_line.is_empty() {
            hit.path.clone()
        } else {
            first_line
        },
        subtitle: display_path(&hit.path, settings),
        path: Some(hit.path.clone()),
        icon: IconRef::FileType { ext },
        score: 0.0,
        match_ranges: Vec::new(),
        meta: ResultMeta {
            line: hit.line,
            label: Some("Text in screenshot".into()),
            ..ResultMeta::default()
        },
        action: ActionSpec::OpenPath {
            path: hit.path,
            line: None,
            column: None,
        },
        actions: Vec::new(),
    }
}

fn calculator_result(calc: ps_sources::calc::CalcResult) -> SearchResult {
    SearchResult {
        id: "calculator".into(),
        kind: SourceKind::Calculator,
        title: calc.formatted,
        subtitle: calc.label,
        path: None,
        icon: IconRef::Kind {
            kind: SourceKind::Calculator,
        },
        score: 0.0,
        match_ranges: Vec::new(),
        meta: ResultMeta {
            label: Some("Copy".into()),
            ..ResultMeta::default()
        },
        action: ActionSpec::CopyText {
            text: format!("{}", calc.value),
        },
        actions: Vec::new(),
    }
}

fn describe_action(action: &ActionSpec) -> (ActivityKind, String, Option<String>) {
    match action {
        ActionSpec::OpenPath { path, .. } => (
            ActivityKind::Open,
            Path::new(path)
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone()),
            Some(path.clone()),
        ),
        ActionSpec::RevealPath { path } => {
            (ActivityKind::Open, "Reveal in Explorer".into(), Some(path.clone()))
        }
        ActionSpec::OpenUrl { url } => (ActivityKind::Open, url.clone(), None),
        ActionSpec::LaunchApp { app_id, .. } => (ActivityKind::Launch, app_id.clone(), None),
        ActionSpec::CopyText { text } => (ActivityKind::Open, text.chars().take(80).collect(), None),
        ActionSpec::RunNative { command } => (ActivityKind::Workflow, format!("{command:?}"), None),
        ActionSpec::RunWorkflow { id } => (ActivityKind::Workflow, id.clone(), None),
        ActionSpec::None => (ActivityKind::System, "No action".into(), None),
    }
}

fn is_destructive(command: NativeCommand) -> bool {
    matches!(
        command,
        NativeCommand::Shutdown | NativeCommand::Restart | NativeCommand::EmptyRecycleBin
    )
}

fn indexer_state(state: u8) -> IndexerState {
    match state {
        STATE_READY => IndexerState::Ready,
        STATE_INDEXING => IndexerState::Indexing,
        STATE_PAUSED => IndexerState::Paused,
        STATE_ERROR => IndexerState::Error,
        _ => IndexerState::Starting,
    }
}

fn suggested_roots() -> Vec<RootSuggestion> {
    let home = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_default();
    let candidates = [
        ("Desktop", "Desktop", RootMode::NamesOnly),
        ("Documents", "Documents", RootMode::Content),
        ("Downloads", "Downloads", RootMode::NamesOnly),
        ("Pictures", "Pictures", RootMode::ContentAndOcr),
    ];
    candidates
        .iter()
        .map(|(folder, label, mode)| {
            let path = home.join(folder);
            RootSuggestion {
                path: path.display().to_string(),
                label: label.to_string(),
                exists: path.is_dir(),
                default_mode: *mode,
            }
        })
        .collect()
}
