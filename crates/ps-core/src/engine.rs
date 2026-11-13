//! Engine facade: query coordination, ranking, persistence and background
//! indexing. OWNER: L0 (integration layer). Public methods are the contract used
//! by the Tauri shell and the benchmark harness.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::thread::JoinHandle;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::Context;
use ps_index::{EntryKind, FilenameIndex};
use ps_ipc::*;

use crate::catalog;
use crate::db::EngineDb;
use crate::rank::{
    relevance_class, usage_signal, MatchKind, CATEGORY_ACTION, CATEGORY_APP, CATEGORY_BROWSER,
    CATEGORY_CALCULATOR, CATEGORY_CLIPBOARD, CATEGORY_CONTENT, CATEGORY_FILE, CATEGORY_FOLDER, CATEGORY_GIT,
    CATEGORY_WEB, CLASS_FALLBACK,
};

mod preview;

pub const INDEX_FILE: &str = "filename-index.bin";
const DB_FILE: &str = "index.db";
const MAX_CONTENT_PER_PASS: usize = 4000;
const MAX_WARNINGS: usize = 24;

// -- Preview limits (Gate 2) --
const PREVIEW_DEFAULT_CHARS: usize = 8_000;
const PREVIEW_MAX_CHARS: usize = 32_000;
const PREVIEW_MIN_CHARS: usize = 200;

/// A persisted filename index older than this is rescanned at startup, and a
/// running app refreshes the index and app list this often, so renamed, moved
/// or deleted items and (un)installed apps do not linger.
const REFRESH_INTERVAL_MS: i64 = 30 * 60 * 1000;
/// Results checked against the disk before they are shown (the visible rows
/// plus keyboard reach); missing ones are dropped and forgotten.
const VERIFIED_RESULTS: usize = 12;
const STATE_STARTING: u8 = 0;
const STATE_READY: u8 = 1;
const STATE_INDEXING: u8 = 2;
const STATE_PAUSED: u8 = 3;
const STATE_ERROR: u8 = 4;
/// Folder of the Tesseract executable saved from the app; None lets ps-ocr detect it.
fn tesseract_dir(settings: &Settings) -> Option<PathBuf> {
    settings
        .tesseract_path
        .as_deref()
        .and_then(|path| Path::new(path).parent())
        .map(Path::to_path_buf)
}

/// The ps-ocr engine id and model folder for a saved OCR engine. The settings check and
/// indexing both use it, so the engine that is checked is the one that reads the images.
fn ocr_backend(engine: OcrEngine, data_dir: &Path, settings: &Settings) -> (&'static str, Option<PathBuf>) {
    match engine {
        OcrEngine::Windows => ("windows", None),
        OcrEngine::PpOcrV6 => ("pp_ocr_v6", Some(data_dir.join("models").join("ppocrv6"))),
        OcrEngine::Indra => ("indra", Some(data_dir.join("models").join("indra"))),
        OcrEngine::Tesseract => ("tesseract", tesseract_dir(settings)),
    }
}

fn ensure_ocr_engine_available(engine: OcrEngine, data_dir: &Path, settings: &Settings) -> anyhow::Result<()> {
    let (engine_name, model_dir) = ocr_backend(engine, data_dir, settings);
    if !ps_ocr::is_engine_available(engine_name, model_dir.as_deref()) {
        anyhow::bail!(
            "OCR engine {:?} is unavailable. Select Windows OCR in settings; optional engines are not installed.",
            engine
        );
    }
    Ok(())
}

/// Test seam: unit tests must never install a real Win32 clipboard hook (it is
/// a process-global hook thread and can corrupt a short-lived test process on
/// teardown). Tests use an inert handle so the engine lifecycle logic is still
/// exercised end to end. Production always uses the real listener.
#[cfg(test)]
static FAKE_CLIPBOARD_HOOK: AtomicBool = AtomicBool::new(true);

enum ClipboardHook {
    Real(ps_windows::ClipboardListener),
    #[cfg(test)]
    Fake,
}

/// Format toggles for the clipboard hook (captureText/captureImages are format
/// toggles only; the master gate is privacy.clipboard_history && !clipboard.pause).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ClipboardCapture {
    capture_text: bool,
    capture_images: bool,
}

impl ClipboardHook {
    fn start(
        db_path: PathBuf,
        image_dir: PathBuf,
        capture: ClipboardCapture,
        on_change: Option<Arc<dyn Fn() + Send + Sync>>,
    ) -> anyhow::Result<Self> {
        #[cfg(test)]
        if FAKE_CLIPBOARD_HOOK.load(Ordering::Relaxed) {
            return Ok(ClipboardHook::Fake);
        }
        let options = ps_sources::clipboard::ClipboardCaptureOptions {
            capture_text: capture.capture_text,
            capture_images: capture.capture_images,
        };
        Ok(ClipboardHook::Real(
            ps_sources::clipboard::start_listener_with_callback(db_path, image_dir, options, on_change)?,
        ))
    }

    fn stop(self) {
        match self {
            ClipboardHook::Real(listener) => listener.stop(),
            #[cfg(test)]
            ClipboardHook::Fake => {}
        }
    }
}

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
    /// Last tie-break inside a source (lower first).
    order: u32,
    result: SearchResult,
}

/// How often and how recently something was opened or launched.
#[derive(Debug, Clone, Copy, Default)]
struct Usage {
    count: u32,
    last_ms: i64,
}

/// History key for a launched app.
fn app_usage_key(target: &str) -> String {
    format!("app:{}", target.to_lowercase())
}

use preview::ResultRegistry;

pub struct Engine {
    config: EngineConfig,
    settings: RwLock<Settings>,
    index: RwLock<FilenameIndex>,
    /// Content index handle. Arc so a query never holds the engine lock while
    /// the background indexer is extracting a document (task-42 p95).
    content: RwLock<Option<Arc<ps_content::ContentIndex>>>,
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
    /// Set when the OCR engine changes; cleared once indexing is ready again.
    reindexing: AtomicBool,
    /// Images still queued in the running OCR pass; 0 when idle.
    ocr_pending: AtomicU64,
    legacy_summary: Mutex<Option<LegacySummary>>,
    /// Recently returned results, so a preview request can resolve resultId -> row.
    recent_results: Mutex<ResultRegistry>,
    /// Live Win32 clipboard listener (only one per process). Held so it can be
    /// stopped explicitly on shutdown or when the privacy setting is disabled.
    clipboard_listener: Mutex<Option<(ClipboardHook, ClipboardCapture)>>,
    clipboard_callback: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
    /// One-shot confirmation state for destructive native commands: a request
    /// must be repeated through confirm_destructive within 15 seconds.
    pending_destructive: Mutex<Option<(NativeCommand, i64)>>,
    /// Open/launch history per lower-case path (or app_usage_key): the
    /// secondary ranking signal. Seeded from the activity log and the Windows
    /// Recent folder, updated whenever a result is opened.
    usage: RwLock<HashMap<String, Usage>>,
    /// Folded app names for matching, keyed by a fingerprint of the app list.
    prepared_apps: Mutex<Option<(u64, Arc<Vec<crate::appmatch::PreparedApp>>)>>,
    /// The periodic index/app refresher is running.
    refresher_started: AtomicBool,
    /// Check shown results against the disk (off only for tests whose
    /// fixtures use paths that do not exist).
    verify_paths: AtomicBool,
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
        let db_path = data_dir.join(DB_FILE);
        let db = EngineDb::open(&db_path)?;
        // Heal the case where a previous run imported legacy rows into the v2
        // database but never persisted the in-memory index: the rows are still
        // there, so search/counts must not start at zero.
        let index = if index.is_empty() {
            match load_migrated_files(&db_path) {
                Ok(entries) if !entries.is_empty() => {
                    warnings.push(format!(
                        "recovered {} migrated filename entries from the v2 database",
                        entries.len()
                    ));
                    FilenameIndex::from_entries(entries)
                }
                _ => index,
            }
        } else {
            index
        };
        let filename_entries = index.len() as u64;
        let content = match ps_content::ContentIndex::open(&db_path) {
            Ok(index) => Some(index),
            Err(err) => {
                warnings.push(format!("content index unavailable ({err})"));
                None
            }
        };
        let content_documents = content.as_ref().map(|c| c.stats().documents).unwrap_or(0);
        let ocr_documents = db.ocr_count();
        let mut usage: HashMap<String, Usage> = HashMap::new();
        for (kind, key, count, last_ms) in db.usage_history().unwrap_or_default() {
            let key = if kind == "launch" { app_usage_key(&key) } else { key.to_lowercase() };
            let slot = usage.entry(key).or_default();
            slot.count = slot.count.saturating_add(count);
            slot.last_ms = slot.last_ms.max(last_ms);
        }

        // A pending one-time legacy migration is real background work, so the
        // UI must show Starting until the pipeline has actually finished.
        let legacy_pending = !settings.legacy_migrated
            && config
                .legacy_db
                .as_deref()
                .map(|path| path.is_file())
                .unwrap_or(false);
        let has_roots = settings
            .indexing
            .roots
            .iter()
            .any(|root| root.enabled && !root.path.trim().is_empty());
        let needs_work = legacy_pending || (settings.onboarding_complete && has_roots);
        let initial_state = if needs_work { STATE_STARTING } else { STATE_READY };

        let this = Self {
            config,
            settings: RwLock::new(settings),
            index: RwLock::new(index),
            content: RwLock::new(content.map(Arc::new)),
            db,
            // Last known app list: apps are searchable the moment the app
            // starts, not only after the background Start Menu scan.
            sources: RwLock::new(ps_sources::Sources {
                apps: load_app_cache(&data_dir),
                ..ps_sources::Sources::default()
            }),
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
            reindexing: AtomicBool::new(false),
            ocr_pending: AtomicU64::new(0),
            legacy_summary: Mutex::new(None),
            recent_results: Mutex::new(ResultRegistry::default()),
            clipboard_listener: Mutex::new(None),
            clipboard_callback: Mutex::new(None),
            pending_destructive: Mutex::new(None),
            usage: RwLock::new(usage),
            prepared_apps: Mutex::new(None),
            refresher_started: AtomicBool::new(false),
            verify_paths: AtomicBool::new(true),
        };
        this.sync_clipboard_listener();
        Ok(this)
    }

    pub fn version(&self) -> &str {
        &self.config.version
    }

    pub fn data_dir(&self) -> &Path {
        &self.config.data_dir
    }

    pub fn set_clipboard_change_callback(&self, cb: Arc<dyn Fn() + Send + Sync>) {
        if let Ok(mut slot) = self.clipboard_callback.lock() {
            *slot = Some(cb);
        }
        self.sync_clipboard_listener();
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
        // Starting -> Indexing -> Ready: the thread publishes Indexing once it
        // begins real work and always finalizes the state when it exits.
        if self.paused.load(Ordering::SeqCst) {
            self.set_state(STATE_PAUSED);
        } else {
            self.set_phase("starting");
            self.set_state(STATE_STARTING);
        }
        let engine = Arc::clone(&self);
        *guard = Some(std::thread::spawn(move || run_background(engine)));
        drop(guard);
        if !self.refresher_started.swap(true, Ordering::SeqCst) {
            let engine = Arc::downgrade(&self);
            let spawned = std::thread::Builder::new()
                .name("ps-refresh".into())
                .spawn(move || refresh_loop(engine));
            if spawned.is_err() {
                self.refresher_started.store(false, Ordering::SeqCst);
            }
        }
    }

    fn background_running(&self) -> bool {
        self.background
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|handle| !handle.is_finished()))
            .unwrap_or(false)
    }

    /// Queues a full pass (rescan, content, OCR). A pass that is already running takes
    /// it on before it reports Ready (see run_background); paused indexing stays Paused.
    fn request_pass(&self) {
        self.force_rescan.store(true, Ordering::SeqCst);
        if !self.paused.load(Ordering::SeqCst) && !self.background_running() {
            self.set_state(STATE_STARTING);
        }
    }

    // -- search ------------------------------------------------------------

    pub fn search(&self, req: &SearchRequest) -> SearchResponse {
        let started = Instant::now();
        let limit = req.limit.clamp(5, 100);
        let settings = self.settings_snapshot();
        let (prefix_scope, query_text) = effective_query(&req.query, req.scope);
        // Natural-language imperative prefixes ("open chrome", "launch ...")
        // are stripped for app/file/source matching. A query that is only a
        // verb is left untouched so literal searches keep working.
        let intent_text = strip_imperative_verbs(&query_text);
        let parsed = crate::query::ParsedQuery::parse(intent_text.as_ref());
        let scope = if req.scope != SearchScope::All {
            req.scope
        } else if parsed.scope != SearchScope::All {
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

        // Empty query: the launcher shows contextual recent content. For the
        // CLIPBOARD scope that is recent clipboard history (privacy-gated,
        // pinned-first/newest, categorized) - not recent files.
        if terms.is_empty() {
            if scope == SearchScope::Clipboard {
                let tier_start = Instant::now();
                let mut hits = 0usize;
                if settings.privacy.clipboard_history {
                    let db_path = self.clipboard_db_path();
                    for (i, entry) in ps_sources::clipboard::list_recent(&db_path, 20)
                        .into_iter()
                        .enumerate()
                    {
                        ranked.push(Ranked {
                            tier: 0,
                            priority: 8,
                            score: 1000.0 - (i as f32 * 1.0),
                            order: 0,
                            result: clipboard_result(&entry),
                        });
                        hits += 1;
                    }
                }
                timings.push(TierTiming {
                    tier: 0,
                    label: "clipboard history".to_string(),
                    took_ms: tier_start.elapsed().as_secs_f64() as f32 * 1000.0,
                    hits,
                });
                let results = finalize(ranked, limit);
                self.remember_results(&results);
                return self.response(req, query_text, results, timings, started, &settings);
            }
            if scope == SearchScope::Ocr {
                let tier_start = Instant::now();
                let mut hits = 0usize;
                if let Ok(ocr_hits) = self.db.ocr_recent(limit) {
                    for hit in ocr_hits {
                        ranked.push(Ranked {
                            tier: 0,
                            priority: 8,
                            score: 1000.0 - (hits as f32 * 1.0),
                            order: 0,
                            result: ocr_result(hit, &settings),
                        });
                        hits += 1;
                    }
                }
                if hits < limit {
                    let sources = self.sources.read().unwrap();
                    for entry in &sources.recent {
                        if hits >= limit {
                            break;
                        }
                        let p = Path::new(&entry.path);
                        let ext = p
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        if ps_ocr::IMAGE_EXTENSIONS.contains(&ext.as_str())
                            || ps_content::is_supported(p)
                        {
                            // After the OCR rows above.
                            ranked.push(Ranked {
                                tier: 1,
                                priority: 7,
                                score: 800.0 - (hits as f32 * 1.0),
                                order: 0,
                                result: recent_result(entry),
                            });
                            hits += 1;
                        }
                    }
                }
                timings.push(TierTiming {
                    tier: 0,
                    label: "OCR & documents".to_string(),
                    took_ms: tier_start.elapsed().as_secs_f64() as f32 * 1000.0,
                    hits,
                });
                let results = finalize(ranked, limit);
                self.remember_results(&results);
                return self.response(req, query_text, results, timings, started, &settings);
            }
            if scope == SearchScope::Browser {
                let tier_start = Instant::now();
                let mut hits = 0usize;
                if settings.privacy.browser_history {
                    let sources = self.sources.read().unwrap();
                    let want_bookmarks = filters.kinds.is_empty()
                        || filters.kinds.contains(&SourceKind::Bookmark);
                    let want_history = filters.kinds.is_empty()
                        || filters.kinds.contains(&SourceKind::BrowserHistory);

                    // History lives in the database; read the newest pages from it.
                    let stored = self.db.browser(&[], 2_000);
                    let mut bookmarks: Vec<&ps_sources::BrowserEntry> = stored
                        .iter()
                        .filter(|e| e.bookmark && want_bookmarks)
                        .collect();
                    bookmarks.sort_by(|a, b| {
                        b.last_visit_ms
                            .cmp(&a.last_visit_ms)
                            .then_with(|| b.visit_count.cmp(&a.visit_count))
                    });

                    let mut history: Vec<&ps_sources::BrowserEntry> = stored
                        .iter()
                        .filter(|e| !e.bookmark && want_history)
                        .collect();
                    history.sort_by(|a, b| {
                        b.last_visit_ms
                            .cmp(&a.last_visit_ms)
                            .then_with(|| b.visit_count.cmp(&a.visit_count))
                    });

                    let top_bookmark_slots = if want_bookmarks && want_history {
                        2.min(bookmarks.len())
                    } else if want_bookmarks {
                        limit
                    } else {
                        0
                    };

                    for (i, entry) in bookmarks.iter().take(top_bookmark_slots).enumerate() {
                        ranked.push(Ranked {
                            tier: 0,
                            priority: 6,
                            score: 2000.0 - i as f32,
                            order: i as u32,
                            result: browser_result(entry, &sources.favicons),
                        });
                        hits += 1;
                    }
                    for (i, entry) in history.iter().take(limit).enumerate() {
                        ranked.push(Ranked {
                            tier: 0,
                            priority: 6,
                            score: 1900.0 - i as f32,
                            order: i as u32,
                            result: browser_result(entry, &sources.favicons),
                        });
                        hits += 1;
                    }
                    if want_bookmarks && history.len() + top_bookmark_slots < limit {
                        for (i, entry) in bookmarks.iter().skip(top_bookmark_slots).take(limit).enumerate() {
                            ranked.push(Ranked {
                                tier: 0,
                                priority: 6,
                                score: 500.0 - i as f32,
                                order: (top_bookmark_slots + i) as u32,
                                result: browser_result(entry, &sources.favicons),
                            });
                            hits += 1;
                        }
                    }
                }
                timings.push(TierTiming {
                    tier: 0,
                    label: "browser history".to_string(),
                    took_ms: tier_start.elapsed().as_secs_f64() as f32 * 1000.0,
                    hits,
                });
                let results = finalize(ranked, limit);
                self.remember_results(&results);
                return self.response(req, query_text, results, timings, started, &settings);
            }
            if scope == SearchScope::Git {
                // The Git card: every repository, then recent commits.
                let tier_start = Instant::now();
                let mut hits = 0usize;
                if settings.privacy.git_indexing {
                    let sources = self.sources.read().unwrap();
                    for (i, entry) in git_activity(&sources.git, limit).into_iter().enumerate() {
                        ranked.push(Ranked {
                            tier: 0,
                            priority: 7,
                            score: 1000.0 - i as f32,
                            order: i as u32,
                            result: git_result(entry),
                        });
                        hits += 1;
                    }
                }
                timings.push(TierTiming {
                    tier: 0,
                    label: "git activity".to_string(),
                    took_ms: tier_start.elapsed().as_secs_f64() as f32 * 1000.0,
                    hits,
                });
                let results = finalize(ranked, limit);
                self.remember_results(&results);
                return self.response(req, query_text, results, timings, started, &settings);
            }
            let sources = self.sources.read().unwrap();
            let mut recent: Vec<&ps_sources::RecentEntry> = sources.recent.iter().collect();
            recent.sort_by(|a, b| b.last_used_ms.cmp(&a.last_used_ms));
            for entry in recent.into_iter().filter(|_| scope == SearchScope::All).take(5 + VERIFIED_RESULTS) {
                ranked.push(Ranked {
                    tier: 0,
                    priority: 1,
                    score: 1.0,
                    order: 0,
                    result: recent_result(entry),
                });
            }
            drop(sources);
            let results = self.drop_missing(finalize(ranked, limit + VERIFIED_RESULTS), 5.min(limit));
            self.remember_results(&results);
            return self.response(req, query_text, results, timings, started, &settings);
        }

        // Ranking for All / Files / Folders / Apps: the tier is the relevance
        // class of how the NAME matched (rank::relevance_class), the priority the
        // category (Apps, Folders, Files, documents/OCR, other sources) and the
        // score the open/launch history plus closeness inside the class; the
        // order field holds each source's stable tie-breaks. Relevance always
        // wins over category. Other filters keep their established ordering.
        let ranked_scope = matches!(
            scope,
            SearchScope::All | SearchScope::Files | SearchScope::Folders | SearchScope::Apps
        );
        let now = now_ms();
        let usage = self.usage.read().unwrap_or_else(|err| err.into_inner());
        let history = |key: &str| {
            usage
                .get(key)
                .map(|entry| usage_signal(entry.count, entry.last_ms, now))
                .unwrap_or(0.0)
        };
        let name_query = ps_index::NameQuery::new(&filename_query);
        // Class of a non-file row from its title; text that matched only
        // elsewhere (a URL, a commit body, clipboard text) is a content match.
        let title_class = |title: &str| {
            let kind = name_query
                .as_ref()
                .and_then(|query| query.matches(title))
                .map(|(score, _)| MatchKind::from(ps_index::NameMatch::from_score(score)))
                .unwrap_or(MatchKind::ContentOnly);
            relevance_class(kind, scope)
        };

        // Calculator, native commands, workflows, web fallback, apps, recents.
        let tier_start = Instant::now();
        let mut tier0_hits = 0usize;
        if scope_allows(scope, SourceKind::Calculator) {
            if let Some(calc) =
                ps_sources::calc::evaluate(&query_text).or_else(|| ps_sources::calc::convert(&query_text))
            {
                ranked.push(Ranked {
                    tier: 0,
                    priority: CATEGORY_CALCULATOR,
                    score: 1000.0,
                    order: 0,
                    result: calculator_result(calc),
                });
                tier0_hits += 1;
            }
        }
        if scope_allows(scope, SourceKind::Command) || scope == SearchScope::All {
            let sources = self.sources.read().unwrap();
            let actions = catalog::match_actions(&query_text).into_iter().take(4).map(command_result);
            let workflows = match_workflows(&sources.workflows, &terms)
                .into_iter()
                .take(3)
                .map(workflow_result);
            for (i, result) in actions.chain(workflows).enumerate() {
                let (tier, priority, score) = if ranked_scope {
                    (title_class(&result.title), CATEGORY_ACTION, 0.0)
                } else {
                    (0, 1, 900.0 - i as f32)
                };
                ranked.push(Ranked {
                    tier,
                    priority,
                    score,
                    order: i as u32,
                    result,
                });
                tier0_hits += 1;
            }
            drop(sources);
        }
        // Direct web search: offered in the All and Web scopes so the "Web"
        // filter is genuinely useful without a browser-history entry. In All it
        // is the fallback after every real match.
        let web_scope_ok = matches!(scope, SearchScope::All | SearchScope::Browser);
        if settings.web_search_enabled && web_scope_ok {
            let trimmed = query_text.trim();
            if !trimmed.is_empty() && trimmed.chars().count() <= 160 {
                let (tier, priority, score) = if scope == SearchScope::All {
                    (CLASS_FALLBACK, CATEGORY_WEB, 0.0)
                } else {
                    (0, 9, 200.0)
                };
                ranked.push(Ranked {
                    tier,
                    priority,
                    score,
                    order: 0,
                    result: web_search_result(trimmed, settings.web_search_engine),
                });
                tier0_hits += 1;
            }
        }
        if scope_allows(scope, SourceKind::Application) {
            let sources = self.sources.read().unwrap();
            let prepared = self.prepared_apps(&sources.apps);
            let folded = crate::appmatch::fold_query(&filename_query);
            let mut hits: Vec<(usize, crate::appmatch::AppHit, f32)> = prepared
                .iter()
                .enumerate()
                .filter_map(|(i, app)| crate::appmatch::match_app(app, &folded).map(|hit| (i, hit)))
                .map(|(i, hit)| (i, hit, history(&app_usage_key(&sources.apps[i].target))))
                .collect();
            hits.sort_by(|a, b| {
                a.1.kind
                    .cmp(&b.1.kind)
                    .then_with(|| b.2.total_cmp(&a.2))
                    .then_with(|| b.1.quality.total_cmp(&a.1.quality))
                    .then_with(|| a.0.cmp(&b.0))
            });
            hits.truncate(if scope == SearchScope::Apps { limit } else { 8 });
            let first_app = ranked.len();
            for (order, (i, hit, signal)) in hits.into_iter().enumerate() {
                let mut result = app_result(&sources.apps[i]);
                result.match_ranges = hit
                    .ranges
                    .iter()
                    .map(|&(start, end)| TextRange { start, end })
                    .collect();
                ranked.push(Ranked {
                    tier: relevance_class(hit.kind.into(), scope),
                    priority: CATEGORY_APP,
                    score: signal * 1000.0 + hit.quality * 999.0,
                    order: order as u32,
                    result,
                });
                tier0_hits += 1;
            }
            disambiguate_apps(&mut ranked[first_app..]);
            // Recently used files (Windows Recent folder): matched by name like
            // indexed files. A row for a path the index also returns collapses
            // into the index row (merge_results dedupes by path).
            if scope == SearchScope::All {
                if let Some(query) = &name_query {
                    for (i, entry) in sources.recent.iter().enumerate() {
                        let Some((score, ranges)) = query.matches(&entry.name) else {
                            continue;
                        };
                        let mut result = recent_result(entry);
                        result.match_ranges = ranges
                            .iter()
                            .map(|&(start, end)| TextRange { start, end })
                            .collect();
                        let kind = MatchKind::from(ps_index::NameMatch::from_score(score));
                        ranked.push(Ranked {
                            tier: relevance_class(kind, scope),
                            priority: CATEGORY_FILE,
                            score: history(&entry.path.to_lowercase()) * 1000.0 + score * 999.0,
                            order: u32::MAX / 2 + i as u32,
                            result,
                        });
                        tier0_hits += 1;
                    }
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

        // Filename index (names only: a matching folder higher up the path does
        // not make the entries below it match).
        if scope_allows(scope, SourceKind::File) || scope_allows(scope, SourceKind::Folder) {
            let tier_start = Instant::now();
            let mut extensions: Vec<String> = if scope == SearchScope::Photos {
                ["png", "jpg", "jpeg", "bmp", "gif", "webp", "tif", "tiff"]
                    .into_iter()
                    .filter(|ext| {
                        filters.extensions.is_empty()
                            || filters
                                .extensions
                                .iter()
                                .any(|filter| filter.trim_start_matches('.').eq_ignore_ascii_case(ext))
                    })
                    .map(str::to_string)
                    .collect()
            } else if scope == SearchScope::Ocr {
                [
                    "png", "jpg", "jpeg", "bmp", "gif", "webp", "tif", "tiff",
                    "pdf", "docx", "doc", "docm", "dotx", "pptx", "pptm", "xlsx", "xlsm",
                    "txt", "text", "rtf", "md", "markdown", "epub", "csv", "log",
                ]
                .into_iter()
                .filter(|ext| {
                    filters.extensions.is_empty()
                        || filters
                            .extensions
                            .iter()
                            .any(|filter| filter.trim_start_matches('.').eq_ignore_ascii_case(ext))
                })
                .map(str::to_string)
                .collect()
            } else {
                filters
                    .extensions
                    .iter()
                    .map(|ext| ext.trim_start_matches('.').to_lowercase())
                    .collect()
            };
            if (scope == SearchScope::Photos || scope == SearchScope::Ocr) && extensions.is_empty() {
                // ps-index treats an empty extension list as unrestricted.
                extensions.push("__no_matching_extension__".to_string());
            }
            // Strict scope semantics: Files never returns folders and Folders
            // never returns files (All keeps both groups separate). An explicit
            // kind: filter can only narrow, never widen, the scope.
            let scope_kind = match scope {
                SearchScope::Files | SearchScope::Photos | SearchScope::Ocr => Some(EntryKind::File),
                SearchScope::Folders => Some(EntryKind::Folder),
                _ => None,
            };
            let explicit_kind = match (
                filters.kinds.contains(&SourceKind::Folder),
                filters.kinds.contains(&SourceKind::File),
            ) {
                (true, false) => Some(EntryKind::Folder),
                (false, true) => Some(EntryKind::File),
                _ => None,
            };
            let kind_filter = scope_kind.or(explicit_kind);
            let entry_filter = ps_index::EntryFilter {
                kind: kind_filter,
                extensions: &extensions,
                under_path: filters.under_path.as_deref(),
                modified_after: filters.modified_after,
                modified_before: filters.modified_before,
                min_size: filters.min_size,
                max_size: filters.max_size,
            };
            let mut rows: Vec<(SearchResult, u8, f32)> = Vec::new();
            {
                let index = self.index.read().unwrap();
                for hit in index.search_filtered(&filename_query, limit.saturating_mul(2), &entry_filter) {
                    if let Some((result, tier)) = file_result(&index, &hit, &settings) {
                        rows.push((result, tier, hit.score));
                    }
                }
            }
            let hits = rows.len();
            if ranked_scope {
                // Tie-breaks inside a class: shallower folders first (the
                // "main" one), then newer, then shorter names, then path.
                let depth = |result: &SearchResult| {
                    if result.kind == SourceKind::Folder {
                        path_depth(result.path.as_deref().unwrap_or(""))
                    } else {
                        0
                    }
                };
                let mut sorted: Vec<usize> = (0..rows.len()).collect();
                sorted.sort_by(|&a, &b| {
                    let (left, right) = (&rows[a].0, &rows[b].0);
                    depth(left)
                        .cmp(&depth(right))
                        .then_with(|| right.meta.modified.cmp(&left.meta.modified))
                        .then_with(|| left.title.chars().count().cmp(&right.title.chars().count()))
                        .then_with(|| left.id.cmp(&right.id))
                });
                let mut order = vec![0u32; rows.len()];
                for (position, &row) in sorted.iter().enumerate() {
                    order[row] = position as u32;
                }
                for (i, (result, _, index_score)) in rows.into_iter().enumerate() {
                    let kind = MatchKind::from(ps_index::NameMatch::from_score(index_score));
                    let priority = if result.kind == SourceKind::Folder {
                        CATEGORY_FOLDER
                    } else {
                        CATEGORY_FILE
                    };
                    let key = result.path.as_deref().unwrap_or("").to_lowercase();
                    ranked.push(Ranked {
                        tier: relevance_class(kind, scope),
                        priority,
                        score: history(&key) * 1000.0 + index_score * 999.0,
                        order: order[i],
                        result,
                    });
                }
            } else {
                for (result, tier, _) in rows {
                    ranked.push(Ranked {
                        tier,
                        priority: 3,
                        score: result.score,
                        order: 0,
                        result,
                    });
                }
            }
            timings.push(TierTiming {
                tier: 1,
                label: "filename index".to_string(),
                took_ms: tier_start.elapsed().as_secs_f64() as f32 * 1000.0,
                hits,
            });
        }

        // Budget: a slower source runs only while it could still change the
        // returned rows. For the ranked filters that means fewer than `limit`
        // rows already rank in a better class than a content-only match.
        let content_class = relevance_class(MatchKind::ContentOnly, scope);
        let count_above = |ceiling: f32| {
            ranked
                .iter()
                .filter(|entry| entry.tier <= 1 && entry.score > ceiling)
                .count()
        };
        let better_than_content = ranked.iter().filter(|entry| entry.tier < content_class).count();
        let content_worth_it = if ranked_scope {
            better_than_content < limit
        } else {
            count_above(500.0) < limit || scope == SearchScope::Ocr
        };
        let sources_worth_it = ranked_scope || count_above(400.0) < limit;
        let ocr_worth_it = if ranked_scope {
            better_than_content < limit
        } else {
            count_above(300.0) < limit || scope == SearchScope::Ocr
        };

        // Indexed document text, clipboard, browser, git.
        if content_worth_it && settings.indexing.content_enabled && content_scope_allows(scope) {
            let tier_start = Instant::now();
            let mut hits = 0usize;
            if let Some(content) = self.content_handle() {
                let remaining = limit.saturating_sub(ranked.len()).clamp(4, 12);
                let result = content.search(intent_text.as_ref(), remaining);
                match result {
                    Ok(content_hits) => {
                        for (i, hit) in content_hits.into_iter().enumerate() {
                            let (tier, priority, score) = if scope == SearchScope::Ocr {
                                (0, 3, 800.0)
                            } else if ranked_scope {
                                (content_class, CATEGORY_CONTENT, 0.0)
                            } else {
                                (1, 3, 500.0)
                            };
                            ranked.push(Ranked {
                                tier,
                                priority,
                                score,
                                order: i as u32,
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
        let mut source_hits = 0usize;
        // Clipboard results are read live from the v2 database: the in-memory
        // sources snapshot is loaded once at startup and would be stale.
        if sources_worth_it
            && settings.privacy.clipboard_history
            && scope_allows(scope, SourceKind::Clipboard)
        {
            let db_path = self.clipboard_db_path();
            let query = if terms.is_empty() {
                ""
            } else {
                filename_query.as_str()
            };
            for (i, entry) in ps_sources::clipboard::search(&db_path, query, 20)
                .into_iter()
                .take(10)
                .enumerate()
            {
                let result = clipboard_result(&entry);
                let (tier, priority, score) = if ranked_scope {
                    (title_class(&result.title), CATEGORY_CLIPBOARD, 0.0)
                } else {
                    (1, 8, 1000.0 - i as f32)
                };
                ranked.push(Ranked {
                    tier,
                    priority,
                    score,
                    order: i as u32,
                    result,
                });
                source_hits += 1;
            }
        }
        let sources = self.sources.read().unwrap();
        if sources_worth_it
            && settings.privacy.browser_history
            && (scope_allows(scope, SourceKind::BrowserHistory)
                || scope_allows(scope, SourceKind::Bookmark))
        {
            let browser_limit = if scope == SearchScope::Browser { limit } else { 4 };
            for (i, entry) in match_browser(&self.db.browser(&terms, 500), &terms)
                .into_iter()
                .filter(|entry| {
                    let kind = if entry.bookmark {
                        SourceKind::Bookmark
                    } else {
                        SourceKind::BrowserHistory
                    };
                    filters.kinds.is_empty() || filters.kinds.contains(&kind)
                })
                .take(browser_limit)
                .enumerate()
            {
                let result = browser_result(entry, &sources.favicons);
                let (tier, priority, score) = if ranked_scope {
                    (title_class(&result.title), CATEGORY_BROWSER, 0.0)
                } else {
                    (1, 6, 380.0)
                };
                ranked.push(Ranked {
                    tier,
                    priority,
                    score,
                    order: i as u32,
                    result,
                });
                source_hits += 1;
            }
        }
        if sources_worth_it && scope_allows(scope, SourceKind::GitCommit) {
            for (i, entry) in match_git(&sources.git, &terms).into_iter().take(4).enumerate() {
                let result = git_result(entry);
                let (tier, priority, score) = if ranked_scope {
                    (title_class(&result.title), CATEGORY_GIT, 0.0)
                } else {
                    (1, 7, 360.0)
                };
                ranked.push(Ranked {
                    tier,
                    priority,
                    score,
                    order: i as u32,
                    result,
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

        // OCR text (already extracted; never run OCR during a query).
        if ocr_worth_it && settings.indexing.ocr_enabled && scope_allows(scope, SourceKind::OcrText) {
            let tier_start = Instant::now();
            let mut hits = 0usize;
            let ocr_hits = if scope == SearchScope::Photos {
                self.db.ocr_search_photos(&query_text, 5, &filters)
            } else if scope == SearchScope::Ocr {
                self.db.ocr_search(&query_text, 10)
            } else {
                self.db.ocr_search(&query_text, 5)
            };
            match ocr_hits {
                Ok(ocr_hits) => {
                    for (i, hit) in ocr_hits.into_iter().enumerate() {
                        let (tier, priority, score) = if scope == SearchScope::Ocr {
                            (0, 3, 900.0)
                        } else if ranked_scope {
                            (content_class, CATEGORY_CONTENT, 0.0)
                        } else {
                            (2, 3, 300.0)
                        };
                        ranked.push(Ranked {
                            tier,
                            priority,
                            score,
                            order: i as u32,
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
        drop(usage);

        let results = self.drop_missing(finalize(ranked, limit + VERIFIED_RESULTS), limit);
        self.remember_results(&results);
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
        let indexed_files = self.filename_count();
        let status = match state {
            STATE_INDEXING | STATE_STARTING => SearchStatus::Indexing {
                indexed_files,
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
                indexed_files,
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

    // -- preview -----------------------------------------------------------

    /// Folded app names for matching, rebuilt only when the app list changes.
    fn prepared_apps(&self, apps: &[ps_sources::AppEntry]) -> Arc<Vec<crate::appmatch::PreparedApp>> {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for app in apps {
            app.name.hash(&mut hasher);
            app.target.hash(&mut hasher);
        }
        let fingerprint = hasher.finish();
        let mut slot = self.prepared_apps.lock().unwrap_or_else(|err| err.into_inner());
        if let Some((kept, prepared)) = slot.as_ref() {
            if *kept == fingerprint {
                return Arc::clone(prepared);
            }
        }
        let prepared: Arc<Vec<crate::appmatch::PreparedApp>> = Arc::new(
            apps.iter()
                .map(|app| crate::appmatch::PreparedApp::new(&app.name, &app.target))
                .collect(),
        );
        *slot = Some((fingerprint, Arc::clone(&prepared)));
        prepared
    }

    /// Deleted, moved or renamed items must not linger as results: the first
    /// rows (the visible ones plus keyboard reach) are checked against the
    /// disk, missing ones are dropped and forgotten by the filename index and
    /// the recent list. One metadata lookup per row.
    fn drop_missing(&self, mut results: Vec<SearchResult>, limit: usize) -> Vec<SearchResult> {
        if !self.verify_paths.load(Ordering::Relaxed) {
            results.truncate(limit);
            return results;
        }
        let mut kept: Vec<SearchResult> = Vec::with_capacity(limit.min(results.len()));
        let mut stale: Vec<String> = Vec::new();
        for result in results {
            if kept.len() >= limit {
                break;
            }
            if kept.len() < VERIFIED_RESULTS {
                if let Some(path) = checkable_path(&result) {
                    if !Path::new(path).exists() {
                        stale.push(path.to_string());
                        continue;
                    }
                }
            }
            kept.push(result);
        }
        if !stale.is_empty() {
            if let Ok(mut index) = self.index.write() {
                for path in &stale {
                    index.remove_path(path);
                }
            }
            if let Ok(mut sources) = self.sources.write() {
                sources
                    .recent
                    .retain(|entry| !stale.iter().any(|path| path.eq_ignore_ascii_case(&entry.path)));
            }
            self.sync_filename_entries();
        }
        kept
    }

    /// Count an opened path or launched app toward its history signal.
    fn record_usage(&self, action: &ActionSpec) {
        let key = match action {
            ActionSpec::OpenPath { path, .. } | ActionSpec::RevealPath { path } => path.to_lowercase(),
            ActionSpec::LaunchApp { app_id, .. } => app_usage_key(app_id),
            _ => return,
        };
        if let Ok(mut usage) = self.usage.write() {
            let entry = usage.entry(key).or_default();
            entry.count = entry.count.saturating_add(1);
            entry.last_ms = now_ms();
        }
    }

    fn remember_results(&self, results: &[SearchResult]) {
        // Preview only needs the display/metadata fields; drop the icon payload
        // and action list from the registry copy so a keystroke-search does not
        // clone tens of KB of app icons into memory.
        let rows: Vec<SearchResult> = results.iter().map(trim_for_registry).collect();
        if let Ok(mut registry) = self.recent_results.lock() {
            registry.remember(&rows);
        }
    }

    /// Build a bounded preview for a result id returned by search().
    ///
    /// Synchronous and bounded (extraction caps); the UI path must
    /// call it from a blocking worker, never from the search hot path. Unknown
    /// or expired ids return an honest payload instead of blank content.
    pub fn preview(&self, req: &PreviewRequest) -> PreviewPayload {
        let max_chars = req
            .max_chars
            .unwrap_or(PREVIEW_DEFAULT_CHARS)
            .clamp(PREVIEW_MIN_CHARS, PREVIEW_MAX_CHARS);
        let payload = match self.resolve_result(&req.result_id) {
            Some(result) => preview::build_preview(&result, max_chars, &self.db, &self.clipboard_db_path()),
            None => PreviewPayload {
                result_id: req.result_id.clone(),
                title: String::new(),
                subtitle: String::new(),
                format: "unavailable".to_string(),
                text: None,
                snippet: None,
                highlight_ranges: Vec::new(),
                image_data_url: None,
                ocr_text: None,
                fields: Vec::new(),
                unsupported_reason: Some(
                    "This result is no longer available. Run the search again.".to_string(),
                ),
                status: PreviewStatus::Unsupported,
                truncated: false,
                error: None,
                image_mime: None,
            },
        };
        payload
    }

    #[cfg(test)]
    pub(crate) fn registry_len(&self) -> usize {
        self.recent_results
            .lock()
            .map(|registry| registry.len())
            .unwrap_or(0)
    }

    /// Resolve a result id to the row the UI saw. The registry covers the normal
    /// flow; the path-prefixed fallback keeps previews working across restarts
    /// and re-searches for file/content/OCR rows.
    fn resolve_result(&self, result_id: &str) -> Option<SearchResult> {
        if let Ok(registry) = self.recent_results.lock() {
            if let Some(result) = registry.get(result_id) {
                return Some(result.clone());
            }
        }
        for (prefix, kind) in [
            ("file:", SourceKind::File),
            ("content:", SourceKind::ContentMatch),
            ("ocr:", SourceKind::OcrText),
        ] {
            if let Some(path) = result_id.strip_prefix(prefix) {
                let name = Path::new(path)
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string());
                return Some(SearchResult {
                    id: result_id.to_string(),
                    kind,
                    title: name,
                    subtitle: String::new(),
                    path: Some(path.to_string()),
                    icon: IconRef::Monogram,
                    score: 0.0,
                    match_ranges: Vec::new(),
                    meta: ResultMeta::default(),
                    action: ActionSpec::OpenPath {
                        path: path.to_string(),
                        line: None,
                        column: None,
                    },
                    actions: Vec::new(),
                });
            }
        }
        None
    }

    // -- status / settings -------------------------------------------------

    pub fn status(&self) -> EngineStatus {
        let settings = self.settings_snapshot();
        let index = self.index.read().unwrap();
        // The index is the source of truth for the user-visible count; refresh
        // the cached atomic here so a missed update elsewhere can never make the
        // UI show "0 items" while entries are searchable.
        let filename_entries = index.len() as u64;
        self.filename_entries.store(filename_entries, Ordering::Relaxed);
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
        let clipboard_items = ps_sources::clipboard::count(&self.clipboard_db_path());
        let sources = self.sources.read().unwrap();
        let source_status = vec![
            SourceStatus {
                id: "apps".into(),
                label: "Applications".into(),
                enabled: true,
                item_count: sources.apps.len() as u64,
                state: if sources.apps.is_empty() {
                    "starting"
                } else {
                    "ready"
                }
                .into(),
                detail: None,
            },
            SourceStatus {
                id: "browser".into(),
                label: "Browser history & bookmarks".into(),
                enabled: settings.privacy.browser_history,
                item_count: self.db.browser_count(),
                state: if settings.privacy.browser_history {
                    "ready"
                } else {
                    "disabled"
                }
                .into(),
                detail: None,
            },
            SourceStatus {
                id: "clipboard".into(),
                label: "Clipboard".into(),
                enabled: settings.privacy.clipboard_history,
                item_count: clipboard_items,
                state: if !settings.privacy.clipboard_history {
                    "disabled"
                } else if settings.clipboard.pause {
                    "paused"
                } else {
                    "ready"
                }
                .into(),
                detail: if !settings.privacy.clipboard_history {
                    None
                } else if settings.clipboard.pause {
                    Some("capture paused".into())
                } else {
                    Some("capturing clipboard updates".into())
                },
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
                state: if settings.privacy.git_indexing {
                    "ready"
                } else {
                    "disabled"
                }
                .into(),
                detail: None,
            },
        ];
        drop(sources);

        EngineStatus {
            protocol: PROTOCOL_VERSION,
            version: self.version().to_string(),
            state: indexer_state(self.state.load(Ordering::Relaxed)),
            filename_entries,
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
            reindexing: self.reindexing.load(Ordering::SeqCst),
            ocr_pending: self.ocr_pending.load(Ordering::SeqCst),
            warnings: self.warnings.lock().map(|w| w.clone()).unwrap_or_default(),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings_snapshot()
    }

    pub fn update_settings(&self, patch: SettingsPatch) -> anyhow::Result<Settings> {
        if let Some(engine) = patch.ocr_engine {
            ensure_ocr_engine_available(engine, &self.config.data_dir, &self.settings_snapshot())?;
        }
        let previous_engine = self.settings_snapshot().ocr_engine;
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
            if next.ocr_engine != previous_engine {
                // OCR results are keyed by engine, so the next pass re-reads every image.
                self.reindexing.store(true, Ordering::SeqCst);
                self.request_pass();
            }
            if patch.privacy.is_some() || patch.clipboard.is_some() {
                // Enabling/disabling/pausing clipboard capture takes effect at once.
                self.sync_clipboard_listener();
            }
            if patch.privacy.is_some() {
                if next.privacy.browser_history {
                    let (browser, warnings) = ps_sources::browser::scan_configured(&next.privacy);
                    self.set_browser(browser);
                    for warning in warnings {
                        self.push_warning(warning);
                    }
                } else if let Ok(mut sources) = self.sources.write() {
                    let _ = self.db.store_browser(&[]);
                    sources.favicons.clear();
                }
            }
            if patch.indexing.is_some() || patch.privacy.is_some() {
                if next.indexing.paused {
                    self.paused.store(true, Ordering::SeqCst);
                    self.set_state(STATE_PAUSED);
                } else {
                    self.paused.store(false, Ordering::SeqCst);
                    // Unpausing with no worker running is "Starting" until the
                    // shell re-spawns the background pipeline, not "Indexing".
                    let running = self
                        .background
                        .lock()
                        .ok()
                        .and_then(|guard| guard.as_ref().map(|handle| !handle.is_finished()))
                        .unwrap_or(false);
                    if !running {
                        self.set_state(STATE_STARTING);
                    }
                }
            }
        }
        Ok(next)
    }

    pub fn activity(&self, query: ActivityQuery) -> Vec<ActivityItem> {
        self.db.activity(&query).unwrap_or_default()
    }

    pub fn clear_activity(&self) -> anyhow::Result<u64> {
        if let Ok(mut usage) = self.usage.write() {
            usage.clear();
        }
        self.db.clear_activity()
    }

    pub fn list_ocr_engines(&self) -> Vec<ps_ipc::OcrEngineDetails> {
        let models_dir = self.config.data_dir.join("models");
        let ppocr_available = ps_ocr::is_engine_available("pp_ocr_v6", Some(&models_dir.join("ppocrv6")));
        let indra_available = ps_ocr::is_engine_available("indra", Some(&models_dir.join("indra")));
        let tesseract_available =
            ps_ocr::is_engine_available("tesseract", tesseract_dir(&self.settings_snapshot()).as_deref());
        let windows_available = ps_ocr::is_engine_available("windows", None);

        // Speeds: `ps-bench ocr` warm mean over 60 real screenshots, one thread (2026-11-13).
        // Accuracy: characters read correctly on the synthetic .bench/ocr200 corpus.
        // Re-measure before changing these labels.
        vec![
            ps_ipc::OcrEngineDetails {
                engine: ps_ipc::OcrEngine::PpOcrV6,
                name: "PP-OCRv6".to_string(),
                availability: if ppocr_available {
                    ps_ipc::OcrEngineAvailability::Available
                } else {
                    ps_ipc::OcrEngineAvailability::NotInstalled
                },
                size_label: "~6.1 MB".to_string(),
                speed_label: "Most accurate (~165ms/img)".to_string(),
                badge: "Recommended".to_string(),
                description: "Compact model pack for screenshots, PDFs and printed text.".to_string(),
                highlights: vec![
                    "High accuracy on screenshots, PDFs & text".to_string(),
                    "Most accurate in our test: 96% of characters read correctly".to_string(),
                    "Compact ~6.1 MB model download".to_string(),
                ],
                license: Some("Apache 2.0".to_string()),
                download_size_bytes: Some(6_100_000),
            },
            ps_ipc::OcrEngineDetails {
                engine: ps_ipc::OcrEngine::Windows,
                name: "Windows OCR".to_string(),
                availability: if windows_available {
                    ps_ipc::OcrEngineAvailability::Available
                } else {
                    ps_ipc::OcrEngineAvailability::NotInstalled
                },
                size_label: "Built-in (0 MB)".to_string(),
                speed_label: "Fastest (~55ms/img)".to_string(),
                badge: "Built-in".to_string(),
                description: "Built into Windows 10 and 11. Nothing to download.".to_string(),
                highlights: vec![
                    "Built into Windows 10 & 11".to_string(),
                    "Zero setup or downloads required".to_string(),
                    "100% offline and low memory footprint".to_string(),
                ],
                license: None,
                download_size_bytes: None,
            },
            ps_ipc::OcrEngineDetails {
                engine: ps_ipc::OcrEngine::Indra,
                name: "Indra OCR".to_string(),
                availability: if indra_available {
                    ps_ipc::OcrEngineAvailability::Available
                } else {
                    ps_ipc::OcrEngineAvailability::NotInstalled
                },
                size_label: "~27 MB".to_string(),
                speed_label: "~150ms/img".to_string(),
                badge: "Multilingual & Skewed".to_string(),
                description: "Stronger on skewed, rotated and multilingual text.".to_string(),
                highlights: vec![
                    "Specialized in skewed and rotated text".to_string(),
                    "Extended multilingual script coverage".to_string(),
                    "Research and non-commercial use".to_string(),
                ],
                license: Some("Research & non-commercial".to_string()),
                download_size_bytes: Some(27_000_000),
            },
            ps_ipc::OcrEngineDetails {
                engine: ps_ipc::OcrEngine::Tesseract,
                name: "Tesseract OCR".to_string(),
                availability: if tesseract_available {
                    ps_ipc::OcrEngineAvailability::Available
                } else {
                    ps_ipc::OcrEngineAvailability::NotInstalled
                },
                size_label: "External CLI".to_string(),
                speed_label: "Slowest (~650ms/img)".to_string(),
                badge: "Extensible".to_string(),
                description: "Open-source engine that uses your installed Tesseract.".to_string(),
                highlights: vec![
                    "Uses system Tesseract installation".to_string(),
                    "Extensible with custom traineddata packs".to_string(),
                    "Traditional open-source engine".to_string(),
                ],
                license: Some("Apache 2.0".to_string()),
                download_size_bytes: None,
            },
        ]
    }

    /// Installs Tesseract with winget, then saves the executable it finds for OCR.
    pub fn install_tesseract_ocr(&self) -> anyhow::Result<Settings> {
        let path = ps_ocr::tesseract::Tesseract::install_with_winget()?;
        self.update_settings(SettingsPatch {
            tesseract_path: Some(path.display().to_string()),
            ..Default::default()
        })
    }

    pub fn download_ocr_engine(&self, engine: ps_ipc::OcrEngine) -> anyhow::Result<Settings> {
        match engine {
            ps_ipc::OcrEngine::PpOcrV6 => {
                let model_dir = self.config.data_dir.join("models").join("ppocrv6");
                let parent = self.config.data_dir.join("models");
                std::fs::create_dir_all(&parent)?;
                ps_ocr::ppocr_pack::install_pp_ocrv6_tiny(&model_dir)?;
                self.update_settings(SettingsPatch {
                    ocr_engine: Some(ps_ipc::OcrEngine::PpOcrV6),
                    ..Default::default()
                })
            }
            ps_ipc::OcrEngine::Indra => {
                let model_dir = self.config.data_dir.join("models").join("indra");
                let parent = self.config.data_dir.join("models");
                std::fs::create_dir_all(&parent)?;
                ps_ocr::ppocr_pack::install_indra(&model_dir)?;
                self.update_settings(SettingsPatch {
                    ocr_engine: Some(ps_ipc::OcrEngine::Indra),
                    ..Default::default()
                })
            }
            ps_ipc::OcrEngine::Windows => {
                self.update_settings(SettingsPatch {
                    ocr_engine: Some(ps_ipc::OcrEngine::Windows),
                    ..Default::default()
                })
            }
            _ => anyhow::bail!("Direct download for {:?} is not supported yet.", engine),
        }
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
        ensure_ocr_engine_available(input.ocr_engine, &self.config.data_dir, &settings)?;
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
        settings.ocr_engine = input.ocr_engine;
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
                let pending = self.pending_destructive.lock().ok().and_then(|guard| *guard);
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
                            "Confirmation required to {}. Ask again to execute.",
                            native_action_label(*command)
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
            let _ = self
                .db
                .log_activity(kind, &title, Some(&outcome.message), path.as_deref());
            self.record_usage(&action);
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
                // Indexing only while a worker is actually running; otherwise
                // Starting until the shell re-spawns the background pipeline.
                let running = self
                    .background
                    .lock()
                    .ok()
                    .and_then(|guard| guard.as_ref().map(|handle| !handle.is_finished()))
                    .unwrap_or(false);
                self.set_state(if running { STATE_INDEXING } else { STATE_STARTING });
            }
            IndexCommand::Rebuild => {
                {
                    *self.index.write().unwrap() = FilenameIndex::new();
                }
                self.sync_filename_entries();
                let path = self.config.data_dir.join(INDEX_FILE);
                let _ = std::fs::remove_file(path);
                self.request_pass();
            }
            IndexCommand::RebuildContent => {
                if let Some(content) = self.content_handle() {
                    let _ = content.clear();
                }
                self.content_documents.store(0, Ordering::SeqCst);
                self.request_pass();
            }
            IndexCommand::RebuildOcr => {
                self.db.ocr_clear()?;
                self.ocr_documents.store(0, Ordering::SeqCst);
                self.request_pass();
            }
            IndexCommand::Shutdown => {
                self.shutdown.store(true, Ordering::SeqCst);
                self.stop_clipboard_listener();
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

    /// Clone the content index handle. The engine lock is released before the
    /// caller runs search/extraction, so a slow document extraction in the
    /// background indexer can never block a query.
    fn content_handle(&self) -> Option<Arc<ps_content::ContentIndex>> {
        self.content.read().ok().and_then(|guard| guard.clone())
    }

    fn set_state(&self, state: u8) {
        self.state.store(state, Ordering::SeqCst);
        if state == STATE_READY {
            self.reindexing.store(false, Ordering::SeqCst);
        }
    }

    fn set_phase(&self, phase: &str) {
        if let Ok(mut guard) = self.phase.lock() {
            *guard = phase.to_string();
        }
    }

    /// Recompute the cached filename count from the index. Call after every
    /// mutation of the index (load, migration replace_all, scan, rebuild).
    fn sync_filename_entries(&self) {
        if let Ok(index) = self.index.read() {
            self.filename_entries.store(index.len() as u64, Ordering::Relaxed);
        }
    }
    fn backfill_ocr_metadata(&self) -> anyhow::Result<()> {
        let mut after_id = 0i64;
        loop {
            if self.shutdown.load(Ordering::SeqCst) || self.paused.load(Ordering::SeqCst) {
                break;
            }
            let pending = self.db.ocr_paths_needing_metadata(after_id, 128)?;
            let Some((last_id, _)) = pending.last() else { break };
            after_id = *last_id;
            let index = self.index.read().unwrap();
            let updates: Vec<_> = pending
                .into_iter()
                .filter_map(|(id, path)| {
                    index
                        .metadata_for_path(&path)
                        .map(|(size, modified)| (id, path, size, modified))
                })
                .collect();
            self.db.ocr_backfill_metadata(&updates)?;
        }
        Ok(())
    }

    /// Current filename count read from the index itself, healing the cached
    /// atomic. Used by the status/search hot paths so the footer can never show
    /// "0 items" while entries are searchable.
    fn filename_count(&self) -> u64 {
        match self.index.read() {
            Ok(index) => {
                let len = index.len() as u64;
                self.filename_entries.store(len, Ordering::Relaxed);
                len
            }
            Err(_) => self.filename_entries.load(Ordering::Relaxed),
        }
    }

    fn clipboard_db_path(&self) -> PathBuf {
        self.config.data_dir.join(DB_FILE)
    }

    /// Replace the stored browser history (it lives in the database, not in memory)
    /// and the favicons read for it. The icon files are read before the lock is taken.
    fn set_browser(&self, browser: Vec<ps_sources::BrowserEntry>) {
        let favicons = favicon_data_urls(&browser);
        let _ = self.db.store_browser(&browser);
        if let Ok(mut sources) = self.sources.write() {
            sources.favicons = favicons;
        }
    }

    /// Master gate for clipboard capture: privacy.clipboard_history must be on
    /// AND clipboard.pause off; captureText/captureImages are format toggles.
    /// Idempotent: never creates a second listener, restarts only when the
    /// capture options change, and always stops the hook thread when disabled.
    fn sync_clipboard_listener(&self) {
        let settings = self.settings_snapshot();
        let desired = if settings.privacy.clipboard_history && !settings.clipboard.pause {
            Some(ClipboardCapture {
                capture_text: settings.clipboard.capture_text,
                capture_images: settings.clipboard.capture_images,
            })
        } else {
            None
        };
        let mut guard = match self.clipboard_listener.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        match desired {
            None => {
                if let Some((hook, _)) = guard.take() {
                    drop(guard);
                    hook.stop();
                }
            }
            Some(capture) => {
                if let Some((_, current)) = guard.as_ref() {
                    if *current == capture {
                        return;
                    }
                }
                if let Some((hook, _)) = guard.take() {
                    drop(guard);
                    hook.stop();
                    guard = match self.clipboard_listener.lock() {
                        Ok(guard) => guard,
                        Err(_) => return,
                    };
                }
                let db_path = self.clipboard_db_path();
                let image_dir = self.config.data_dir.join("clipboard-images");
                let on_change = self.clipboard_callback.lock().ok().and_then(|g| g.clone());
                match ClipboardHook::start(db_path, image_dir, capture, on_change) {
                    Ok(hook) => *guard = Some((hook, capture)),
                    Err(err) => {
                        drop(guard);
                        self.push_warning(format!(
                            "clipboard listener unavailable ({err}); history shows imported items only"
                        ));
                    }
                }
            }
        }
    }

    fn stop_clipboard_listener(&self) {
        if let Ok(mut guard) = self.clipboard_listener.lock() {
            if let Some((hook, _)) = guard.take() {
                drop(guard);
                hook.stop();
            }
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
                        {
                            let mut index = self.index.write().unwrap();
                            index.replace_all(entries);
                        }
                        // Migrated rows are searchable right away: publish the
                        // count now instead of waiting for a rescan, and persist
                        // the index so a restart does not start at zero.
                        self.sync_filename_entries();
                        self.save_index();
                        self.last_updated.store(now_ms(), Ordering::SeqCst);
                    }
                }
                let documents = self
                    .content_handle()
                    .map(|content| content.stats().documents)
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

impl Drop for Engine {
    fn drop(&mut self) {
        // Never leak the Win32 clipboard hook thread when an Engine is dropped
        // without an explicit Shutdown command.
        self.stop_clipboard_listener();
    }
}

// ---------------------------------------------------------------------------
// Background pipeline
// ---------------------------------------------------------------------------

/// Background pipeline entry point. Always finalizes the visible state, even
/// when the work returns early (shutdown) or panics, so the UI can never stay
/// stuck on "Indexing…" after the pipeline is done.
fn run_background(engine: Arc<Engine>) {
    loop {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_background_inner(&engine);
        }));
        if outcome.is_err() {
            engine.push_warning(
                "background indexing stopped unexpectedly; the visible index may be partial".to_string(),
            );
        }
        // A rebuild or OCR engine change requested while this pass ran gets a pass of its
        // own before Ready: the shell cannot start one while this thread is alive.
        if outcome.is_ok() && pass_requested(&engine) {
            continue;
        }
        finish_background(&engine);
        // Same for a request that landed while finishing.
        if outcome.is_err() || !pass_requested(&engine) {
            return;
        }
    }
}

fn pass_requested(engine: &Engine) -> bool {
    engine.force_rescan.load(Ordering::SeqCst)
        && !engine.shutdown.load(Ordering::SeqCst)
        && !engine.paused.load(Ordering::SeqCst)
}

fn run_background_inner(engine: &Arc<Engine>) {
    // Starting -> Indexing: published only once the thread is doing real work.
    engine.set_state(STATE_INDEXING);
    engine.set_phase("preparing");
    engine.last_updated.store(now_ms(), Ordering::SeqCst);

    if let Some(legacy) = engine.config.legacy_db.clone() {
        if legacy.is_file() {
            engine.migrate_legacy(&legacy);
            let _ = engine.db.checkpoint();
        }
    }

    // Clipboard capture is live from startup when enabled.
    engine.sync_clipboard_listener();

    if engine.shutdown.load(Ordering::SeqCst) {
        return;
    }
    engine.set_phase("sources");
    {
        let settings = engine.settings_snapshot();
        let legacy = engine.config.legacy_db.clone();
        let (mut sources, warnings) = ps_sources::Sources::load(&settings, legacy.as_deref());
        if !sources.apps.is_empty() {
            save_app_cache(&engine.config.data_dir, &sources.apps);
        }
        // Files opened through Explorer count as recently used.
        if let Ok(mut usage) = engine.usage.write() {
            for entry in &sources.recent {
                let slot = usage.entry(entry.path.to_lowercase()).or_default();
                slot.count = slot.count.max(1);
                slot.last_ms = slot.last_ms.max(entry.last_used_ms);
            }
        }
        let favicons = favicon_data_urls(&sources.browser);
        // The scanned history moves to the database; only its favicons stay in memory.
        let _ = engine.db.store_browser(&std::mem::take(&mut sources.browser));
        if let Ok(mut guard) = engine.sources.write() {
            *guard = sources;
            guard.favicons = favicons;
        }
        for warning in warnings {
            engine.push_warning(warning);
        }
    }

    if engine.shutdown.load(Ordering::SeqCst) {
        return;
    }

    let roots = engine.configured_roots();
    // Taken even with no roots, so only a request made during this pass is left pending.
    let forced = engine.force_rescan.swap(false, Ordering::SeqCst);
    if !roots.is_empty() {
        let current_roots: Vec<String> = roots.iter().map(|root| ps_index::normalize_root(root)).collect();
        let empty = engine.index.read().unwrap().is_empty();
        let rescan = forced || {
            let index = engine.index.read().unwrap();
            empty
                || index.roots() != current_roots.as_slice()
                || now_ms() - index.built_at() > REFRESH_INTERVAL_MS
        };
        if rescan {
            engine.set_phase("scanning files");
            // Only a first scan counts up from zero; refreshing an existing
            // index keeps the shown count steady until the new one swaps in.
            scan_into_index(engine, &roots, empty);
            engine.save_index();
        }
    }

    // Backfill after scanning, before content/OCR work, so legacy rows can
    // use the fresh filename index without delaying heavier indexing stages.
    match engine.db.ocr_metadata_pending() {
        Ok(count) if count > 0 => {
            engine.set_phase("updating OCR metadata");
            if let Err(err) = engine.backfill_ocr_metadata() {
                engine.push_warning(format!("OCR metadata update failed: {err}"));
            } else if !engine.shutdown.load(Ordering::SeqCst) && !engine.paused.load(Ordering::SeqCst) {
                match engine.db.ocr_metadata_pending() {
                    Ok(remaining) if remaining > 0 => engine.push_warning(format!(
                        "{remaining} OCR rows still need filename metadata; they will be retried on the next indexing run"
                    )),
                    Err(err) => engine.push_warning(format!("OCR metadata check failed: {err}")),
                    _ => {}
                }
            }
        }
        Err(err) => engine.push_warning(format!("OCR metadata check failed: {err}")),
        _ => {}
    }

    if !roots.is_empty() {
        engine.set_phase("indexing content");
        index_content(engine, &roots);
        engine.set_phase("indexing screenshots");
        index_ocr(engine, &roots);
    }
}

/// Publish the terminal state and counters. Runs on every background exit path.
fn finish_background(engine: &Arc<Engine>) {
    let _ = engine.db.checkpoint();
    engine.sync_filename_entries();
    let shutdown = engine.shutdown.load(Ordering::SeqCst);
    let paused = engine.paused.load(Ordering::SeqCst);
    engine.set_phase(if shutdown {
        "stopped"
    } else if paused {
        "paused"
    } else {
        "ready"
    });
    engine.set_state(if paused && !shutdown {
        STATE_PAUSED
    } else {
        STATE_READY
    });
    engine.last_updated.store(now_ms(), Ordering::SeqCst);
}

const APP_CACHE_FILE: &str = "apps-cache.json";

fn load_app_cache(data_dir: &Path) -> Vec<ps_sources::AppEntry> {
    std::fs::read(data_dir.join(APP_CACHE_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save_app_cache(data_dir: &Path, apps: &[ps_sources::AppEntry]) {
    if let Ok(bytes) = serde_json::to_vec(apps) {
        let _ = std::fs::write(data_dir.join(APP_CACHE_FILE), bytes);
    }
}

/// Keeps the filename index and the app list current while the app runs, so
/// created, renamed, moved or deleted items and (un)installed apps show up
/// without a manual rebuild. Holds only a weak handle: it never keeps the
/// engine alive and stops with it.
fn refresh_loop(engine: std::sync::Weak<Engine>) {
    let mut last_refresh = Instant::now();
    loop {
        std::thread::sleep(std::time::Duration::from_secs(5));
        let Some(engine) = engine.upgrade() else {
            return;
        };
        if engine.shutdown.load(Ordering::SeqCst) {
            engine.refresher_started.store(false, Ordering::SeqCst);
            return;
        }
        if (last_refresh.elapsed().as_millis() as i64) < REFRESH_INTERVAL_MS
            || engine.paused.load(Ordering::SeqCst)
            || engine.background_running()
        {
            continue;
        }
        last_refresh = Instant::now();
        let roots = engine.configured_roots();
        if !roots.is_empty() {
            scan_into_index(&engine, &roots, false);
            if !engine.shutdown.load(Ordering::SeqCst) {
                engine.save_index();
            }
        }
        let (apps, _) = ps_sources::apps::scan();
        if !apps.is_empty() {
            save_app_cache(&engine.config.data_dir, &apps);
            if let Ok(mut sources) = engine.sources.write() {
                sources.apps = apps;
            }
        }
        let settings = engine.settings_snapshot();
        if settings.privacy.browser_history {
            let (browser, _) = ps_sources::browser::scan_configured(&settings.privacy);
            engine.set_browser(browser);
        }
    }
}

/// Rebuild the filename index from disk. `report_progress` publishes the
/// running count (first scan); a quiet refresh keeps the shown count steady.
/// A scan cut short by shutdown never replaces the live index.
fn scan_into_index(engine: &Arc<Engine>, roots: &[PathBuf], report_progress: bool) {
    let settings = engine.settings_snapshot();
    let excluded = settings.indexing.excluded_paths.clone();
    let junk: Vec<String> = Vec::new();
    let cancelled = || engine.shutdown.load(Ordering::SeqCst);
    let counter = Arc::clone(engine);
    let mut progress = move |count: usize| {
        if report_progress {
            counter.filename_entries.store(count as u64, Ordering::Relaxed);
        }
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
    if engine.shutdown.load(Ordering::SeqCst) {
        return;
    }
    let root_strings: Vec<String> = roots.iter().map(|root| ps_index::normalize_root(root)).collect();
    index.set_roots(root_strings);
    if stats.errors > 0 {
        engine.push_warning(format!("{} directories or files could not be read", stats.errors));
    }
    *engine.index.write().unwrap() = index;
    engine.sync_filename_entries();
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
    let Some(content) = engine.content_handle() else {
        return;
    };
    let mut written = 0usize;
    let mut processed = 0usize;
    for path in candidates {
        if engine.shutdown.load(Ordering::SeqCst) || engine.paused.load(Ordering::SeqCst) {
            break;
        }
        processed += 1;
        if processed % 256 == 0 {
            // Auto-checkpoint is off: fold the WAL here so a long content pass
            // cannot grow it without bound.
            let _ = engine.db.checkpoint();
        }
        match content.index_file(&path, ps_content::DEFAULT_MAX_BYTES) {
            Ok(true) => written += 1,
            Ok(false) => {}
            Err(err) => {
                engine.push_warning(format!("content extraction failed for {}: {err}", path.display()))
            }
        }
    }
    let documents = content.stats().documents;
    engine.content_documents.store(documents, Ordering::SeqCst);
    let _ = engine.db.checkpoint();
    let _ = roots;
    engine.last_updated.store(now_ms(), Ordering::SeqCst);
    let _ = written;
}

fn index_ocr(engine: &Arc<Engine>, roots: &[PathBuf]) {
    let settings = engine.settings_snapshot();
    if !settings.indexing.ocr_enabled {
        return;
    }
    let (engine_name, model_dir) = ocr_backend(settings.ocr_engine, &engine.config.data_dir, &settings);
    if !ps_ocr::is_engine_available(engine_name, model_dir.as_deref()) {
        engine.push_warning(format!(
            "OCR engine {:?} is unavailable or missing local assets; select an available engine in settings.",
            settings.ocr_engine
        ));
        return;
    }
    let modes = root_modes(&settings);
    let candidates = {
        let index = engine.index.read().unwrap();
        ocr_candidates(&index, &modes)
    };
    engine.ocr_pending.store(candidates.len() as u64, Ordering::SeqCst);
    let queue = Mutex::new(candidates.into_iter());
    let processed = AtomicU64::new(0);
    std::thread::scope(|scope| {
        for _ in 0..ocr_worker_count() {
            scope.spawn(|| ocr_worker(engine, &queue, &processed, engine_name, model_dir.as_deref()));
        }
    });
    engine.ocr_pending.store(0, Ordering::SeqCst);
    engine
        .ocr_documents
        .store(engine.db.ocr_count(), Ordering::SeqCst);
    let _ = engine.db.checkpoint();
    engine.last_updated.store(now_ms(), Ordering::SeqCst);
    let _ = roots;
}

/// Parallel OCR calls for the screenshot pass: half the logical cores, never above
/// ps-ocr's cap (calls beyond the cap fail fast there instead of queueing).
fn ocr_worker_count() -> usize {
    std::thread::available_parallelism().map_or(1, |n| (n.get() / 2).clamp(1, ps_ocr::MAX_OCR_WORKERS))
}

/// Takes candidates from the shared queue until it is empty or indexing is paused
/// or shut down. A failing file only adds a warning.
fn ocr_worker(
    engine: &Engine,
    queue: &Mutex<std::vec::IntoIter<(String, u64, i64)>>,
    processed: &AtomicU64,
    engine_name: &str,
    model_dir: Option<&Path>,
) {
    loop {
        if engine.shutdown.load(Ordering::SeqCst) || engine.paused.load(Ordering::SeqCst) {
            break;
        }
        let Some((path, size, modified)) = queue.lock().unwrap().next() else {
            break;
        };
        let processed_now = processed.fetch_add(1, Ordering::SeqCst) + 1;
        engine.ocr_pending.fetch_sub(1, Ordering::SeqCst);
        if processed_now % 128 == 0 {
            let _ = engine.db.checkpoint();
        }
        if processed_now % 16 == 0 {
            // Live count, so a rebuild shows progress instead of 0 until the pass ends.
            engine.ocr_documents.store(engine.db.ocr_count(), Ordering::SeqCst);
        }
        // Keyed by engine: switching engines makes every stored result stale.
        let hash = format!("{engine_name}:{}", quick_hash(&path, size, modified));
        if engine.db.ocr_has(&path, &hash) {
            if let Err(err) = engine.db.ocr_update_metadata(&path, size, modified) {
                engine.push_warning(format!("OCR metadata update failed for {path}: {err}"));
            }
            continue;
        }
        match ps_ocr::extract_image_with_engine(Path::new(&path), 16_777_216, engine_name, model_dir) {
            Ok(Some(document)) if !document.text.trim().is_empty() => {
                if let Err(err) = engine.db.ocr_upsert(
                    &path,
                    &hash,
                    &document.text,
                    &document.language,
                    document.width,
                    document.height,
                    size,
                    modified,
                ) {
                    engine.push_warning(format!("OCR indexing failed for {path}: {err}"));
                }
            }
            Ok(_) => {
                if let Err(err) = engine.db.ocr_delete(&path) {
                    engine.push_warning(format!("OCR cleanup failed for {path}: {err}"));
                }
            }
            Err(err) => engine.push_warning(format!("OCR failed for {path}: {err}")),
        }
    }
}

fn ocr_candidates(index: &FilenameIndex, modes: &[(String, u8)]) -> Vec<(String, u64, i64)> {
    index
        .entries()
        .iter()
        .filter(|entry| entry.kind == EntryKind::File)
        .filter(|entry| ps_ocr::IMAGE_EXTENSIONS.contains(&entry.ext.to_ascii_lowercase().as_str()))
        // OCR runs on any content-enabled root (>= 1) when OCR is globally enabled.
        // ContentAndOcr (level 2) is the preferred mode, but NamesOnly roots (level 0)
        // are intentionally excluded so users can keep fast-only roots fast.
        .filter(|entry| mode_for(modes, &entry.path) >= 1)
        .map(|entry| (entry.path.clone(), entry.size, entry.modified))
        .collect()
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
            // Normalize every separator, not just the trailing one: a root
            // configured with forward slashes must still enable content/OCR
            // indexing for the native paths the scanner stores.
            let mut normalized = root.path.replace('/', "\\").to_lowercase();
            while normalized.ends_with('\\') {
                normalized.pop();
            }
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
    let lower = path.replace('/', "\\").to_lowercase();
    modes
        .iter()
        .filter(|(root, _)| lower.starts_with(root.as_str()))
        .map(|(_, level)| *level)
        .max()
        .unwrap_or(0)
}

fn load_migrated_files(db_path: &Path) -> anyhow::Result<Vec<ps_index::IndexEntry>> {
    let conn = rusqlite::Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
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
    let Ok(conn) = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
    else {
        return summary;
    };
    let count = |table: &str| -> u64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get::<_, i64>(0)
        })
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

/// Strip a leading imperative verb ("open", "launch", "start", "run", "show",
/// "go to", "switch to", "open up") and a trailing "please" so natural-language
/// queries match apps and files. A query that is only a verb (or only
/// "please") is returned unchanged so literal searches are not broken.
fn strip_imperative_verbs(raw: &str) -> std::borrow::Cow<'_, str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return std::borrow::Cow::Borrowed(trimmed);
    }
    let mut text = trimmed;
    // Remove a trailing "please" (word boundary required).
    loop {
        let lower = text.to_ascii_lowercase();
        if lower.ends_with("please") {
            let cut = text.len() - "please".len();
            let before_is_boundary = cut == 0 || text.as_bytes()[cut - 1].is_ascii_whitespace();
            if before_is_boundary {
                let candidate = text[..cut].trim_end();
                if candidate.is_empty() {
                    break;
                }
                text = candidate;
                continue;
            }
        }
        break;
    }
    let lower = text.to_ascii_lowercase();
    const VERBS: [&str; 8] = [
        "go to",
        "switch to",
        "open up",
        "open",
        "launch",
        "start",
        "run",
        "show",
    ];
    for verb in VERBS {
        if lower.starts_with(verb) {
            let rest = &text[verb.len()..];
            if rest.starts_with(' ') || rest.starts_with('\t') {
                let rest = rest.trim_start();
                if !rest.is_empty() {
                    return std::borrow::Cow::Owned(rest.to_string());
                }
            }
        }
    }
    if text.len() != trimmed.len() {
        std::borrow::Cow::Owned(text.to_string())
    } else {
        std::borrow::Cow::Borrowed(trimmed)
    }
}

/// Strips a leading scope prefix and returns the remaining free text.
fn effective_query(raw: &str, fallback: SearchScope) -> (SearchScope, String) {
    let trimmed = raw.trim_start();
    let lower = trimmed.to_lowercase();
    const PREFIXES: [(&str, SearchScope); 18] = [
        ("photos:", SearchScope::Photos),
        ("photo:", SearchScope::Photos),
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
        ("web:", SearchScope::Browser),
        ("google:", SearchScope::Browser),
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
            | (SearchScope::Photos, SourceKind::File)
            | (SearchScope::Photos, SourceKind::OcrText)
            | (SearchScope::Folders, SourceKind::Folder)
            | (SearchScope::Apps, SourceKind::Application)
            | (SearchScope::Clipboard, SourceKind::Clipboard)
            | (SearchScope::Browser, SourceKind::BrowserHistory)
            | (SearchScope::Browser, SourceKind::Bookmark)
            | (SearchScope::Git, SourceKind::GitCommit)
            | (SearchScope::Git, SourceKind::GitRepository)
            | (SearchScope::Git, SourceKind::GitTodo)
            | (SearchScope::Ocr, SourceKind::OcrText)
            | (SearchScope::Ocr, SourceKind::ContentMatch)
            | (SearchScope::Ocr, SourceKind::File)
            | (SearchScope::Ocr, SourceKind::Document)
            | (SearchScope::Actions, SourceKind::Action)
            | (SearchScope::Actions, SourceKind::Command)
            | (SearchScope::Commands, SourceKind::Command)
            | (SearchScope::Content, SourceKind::ContentMatch)
    )
}

/// Content-FTS rows are DOCUMENTS, not FILES: the strict FILES chip must not
/// mix document matches into a file list (task-37 decision; ContentMatch stays
/// reachable through ALL and the content: prefix).
fn content_scope_allows(scope: SearchScope) -> bool {
    matches!(scope, SearchScope::All | SearchScope::Content | SearchScope::Ocr)
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
            order: item.order,
            result: item.result,
        })
        .collect();
    crate::rank::merge_results(items, limit)
}

/// Number of folders above a path (for "the shallower folder first").
fn path_depth(path: &str) -> usize {
    path.bytes().filter(|b| *b == b'\\' || *b == b'/').count()
}

/// Same app name from different installs: show where each one lives.
fn disambiguate_apps(rows: &mut [Ranked]) {
    let mut seen: HashMap<String, usize> = HashMap::new();
    for row in rows.iter() {
        *seen.entry(row.result.title.to_lowercase()).or_default() += 1;
    }
    for row in rows.iter_mut() {
        if seen.get(&row.result.title.to_lowercase()).copied().unwrap_or(0) < 2 {
            continue;
        }
        let path = row.result.path.as_deref().unwrap_or("");
        let location = if path.to_ascii_lowercase().starts_with("shell:appsfolder") {
            "Microsoft Store app".to_string()
        } else {
            Path::new(path)
                .parent()
                .map(|folder| folder.display().to_string())
                .unwrap_or_default()
        };
        if !location.is_empty() {
            row.result.subtitle = format!("Application \u{b7} {location}");
        }
    }
}

/// Filesystem path behind a result that can be checked cheaply. Network
/// (UNC) paths are skipped: a stat there can block for seconds.
fn checkable_path(result: &SearchResult) -> Option<&str> {
    let path = result.path.as_deref()?;
    if path.starts_with("\\\\") || path.to_ascii_lowercase().starts_with("shell:") {
        return None;
    }
    match result.kind {
        SourceKind::File
        | SourceKind::Folder
        | SourceKind::Recent
        | SourceKind::Document
        | SourceKind::ContentMatch
        | SourceKind::OcrText
        | SourceKind::Application => Some(path),
        _ => None,
    }
}

/// Merge UI-supplied filters with filters parsed from the query text; both are
/// restrictive, so a field present in either applies.
/// Character-safe truncation for result snippets.
fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut out: String = text.chars().take(max_chars).collect();
    if text.chars().count() > max_chars {
        out.push('…');
    }
    out
}

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

fn match_browser<'a>(
    items: &'a [ps_sources::BrowserEntry],
    terms: &[String],
) -> Vec<&'a ps_sources::BrowserEntry> {
    // Allocation-free matching: the previous version lowercased + formatted a
    // "title url" haystack for every one of the thousands of browser rows on
    // every query, which showed up as a ~10 ms tail in debug builds.
    let mut hits: Vec<(&ps_sources::BrowserEntry, i64)> = items
        .iter()
        .filter(|item| {
            terms
                .iter()
                .all(|term| contains_ci(&item.title, term) || contains_ci(&item.url, term))
        })
        .map(|item| (item, item.last_visit_ms))
        .collect();
    hits.sort_by(|a, b| b.1.cmp(&a.1));
    hits.into_iter().map(|(item, _)| item).collect()
}

/// Case-insensitive substring test (ASCII folded, non-ASCII bytes exact) without
/// allocating a lowercased copy. The needle is expected lower-case.
fn contains_ci(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let hay = haystack.as_bytes();
    let needle = needle.as_bytes();
    if needle.len() > hay.len() {
        return false;
    }
    let first = needle[0];
    let first_upper = first.to_ascii_uppercase();
    let mut start = 0usize;
    while start + needle.len() <= hay.len() {
        let mut offset = start;
        let mut found = None;
        while offset < hay.len() {
            let byte = hay[offset].to_ascii_lowercase();
            if byte == first || byte == first_upper {
                found = Some(offset);
                break;
            }
            offset += 1;
        }
        let Some(position) = found else {
            return false;
        };
        if position + needle.len() > hay.len() {
            return false;
        }
        if hay[position..position + needle.len()]
            .iter()
            .zip(needle.iter())
            .all(|(left, right)| left.to_ascii_lowercase() == *right)
        {
            return true;
        }
        start = position + 1;
    }
    false
}

fn match_git<'a>(entries: &'a [ps_sources::GitEntry], terms: &[String]) -> Vec<&'a ps_sources::GitEntry> {
    entries
        .iter()
        .filter(|entry| {
            let haystack = match entry {
                ps_sources::GitEntry::Repo { name, path, .. } => {
                    format!("{} {}", name.to_lowercase(), path.to_lowercase())
                }
                ps_sources::GitEntry::Commit {
                    message,
                    author,
                    hash,
                    ..
                } => {
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
    // ps-index scores are normalized 0..1; the coordinator uses a 0..1000
    // scale (content 500, sources 400, OCR 300). Scale once here and derive the
    // tier so exact/prefix filename matches really are tier 0.
    let scaled_score = hit.score * 1000.0;
    let tier = if scaled_score >= 900.0 { 0 } else { 1 };
    Some((
        SearchResult {
            id: format!("file:{}", path.to_lowercase()),
            kind: result_kind,
            title: name,
            subtitle: display_path(&path, settings),
            path: Some(path.clone()),
            icon,
            score: scaled_score,
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
        icon: app_icon(app.icon_path.as_deref()),
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

/// Bounded process-wide cache for application icon data URLs. Typing produces a
/// search per keystroke and the same app icons repeat every time; re-reading the
/// PNG and base64-encoding it on every keystroke was measurable in debug builds.
/// Entries are keyed by path + mtime + size, so a re-extracted icon refreshes.
struct CachedIcon {
    modified_ms: i64,
    len: u64,
    icon: IconRef,
}

const APP_ICON_CACHE_CAP: usize = 256;

fn app_icon_cache() -> &'static Mutex<HashMap<String, CachedIcon>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CachedIcon>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Real application icon as a self-contained PNG data URL (no asset-protocol
/// scope needed in the WebView). Falls back to a monogram tile.
fn app_icon(icon_path: Option<&str>) -> IconRef {
    let Some(path) = icon_path else {
        return IconRef::Monogram;
    };
    let metadata = std::fs::metadata(path).ok();
    let modified_ms = metadata
        .as_ref()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0);
    let len = metadata.as_ref().map(|metadata| metadata.len()).unwrap_or(0);
    if let Ok(cache) = app_icon_cache().lock() {
        if let Some(cached) = cache.get(path) {
            if cached.modified_ms == modified_ms && cached.len == len {
                return cached.icon.clone();
            }
        }
    }
    let icon = match std::fs::read(path) {
        Ok(bytes) if bytes.starts_with(&[0x89, b'P', b'N', b'G']) && bytes.len() <= 512 * 1024 => {
            IconRef::AppIconData {
                data: format!("data:image/png;base64,{}", base64_encode(&bytes)),
            }
        }
        _ => IconRef::Monogram,
    };
    if let Ok(mut cache) = app_icon_cache().lock() {
        if cache.len() >= APP_ICON_CACHE_CAP {
            cache.clear();
        }
        cache.insert(
            path.to_string(),
            CachedIcon {
                modified_ms,
                len,
                icon: icon.clone(),
            },
        );
    }
    icon
}

#[cfg(test)]
fn app_icon_cache_len() -> usize {
    app_icon_cache().lock().map(|cache| cache.len()).unwrap_or(0)
}

/// Host -> image data URL for the most recently browsed hosts.
fn favicon_data_urls(browser: &[ps_sources::BrowserEntry]) -> HashMap<String, String> {
    ps_sources::browser::favicons(browser)
        .into_iter()
        .map(|(host, (mime, bytes))| (host, format!("data:{mime};base64,{}", base64_encode(&bytes))))
        .collect()
}

/// Minimal base64 encoder (icons only; no dependency needed).
fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((triple >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(triple & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
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

fn web_search_result(query: &str, engine: ps_ipc::WebSearchEngine) -> SearchResult {
    SearchResult {
        id: format!("web-search:{}", engine.label().to_lowercase()),
        kind: SourceKind::Action,
        title: format!("Search the web for \"{query}\""),
        subtitle: format!("Direct search with {}", engine.label()),
        path: None,
        icon: IconRef::Kind {
            kind: SourceKind::Bookmark,
        },
        score: 0.0,
        match_ranges: Vec::new(),
        meta: ResultMeta {
            label: Some("Web".into()),
            ..ResultMeta::default()
        },
        action: ActionSpec::OpenUrl {
            url: engine.search_url(query),
        },
        actions: Vec::new(),
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

/// Stable upper-case category label for the launcher badge/filter strip.
fn clip_category_label(category: ps_sources::ClipCategory) -> &'static str {
    match category {
        ps_sources::ClipCategory::Link => "LINK",
        ps_sources::ClipCategory::Email => "EMAIL",
        ps_sources::ClipCategory::Path => "PATH",
        ps_sources::ClipCategory::Code => "CODE",
        ps_sources::ClipCategory::Image => "IMAGE",
        ps_sources::ClipCategory::Files => "FILES",
        ps_sources::ClipCategory::Text => "TEXT",
    }
}

/// Registry copy of a search result: everything preview needs, without the
/// (potentially large) icon data URL or the action list.
fn trim_for_registry(result: &SearchResult) -> SearchResult {
    SearchResult {
        id: result.id.clone(),
        kind: result.kind,
        title: result.title.clone(),
        subtitle: result.subtitle.clone(),
        path: result.path.clone(),
        icon: IconRef::Monogram,
        score: result.score,
        match_ranges: result.match_ranges.clone(),
        meta: ResultMeta {
            favicon_data_url: None,
            ..result.meta.clone()
        },
        action: result.action.clone(),
        actions: Vec::new(),
    }
}

fn clipboard_result(entry: &ps_sources::ClipboardEntry) -> SearchResult {
    let is_image_row = entry.category == ps_sources::ClipCategory::Image || entry.image_path.is_some();
    let first_line = entry.text.lines().next().unwrap_or("").trim();
    let mut title: String = first_line.chars().take(120).collect();
    if is_image_row {
        // Image rows carry a placeholder in text ("[image:<hash>]"); show a real
        // title plus dimensions instead.
        title = match (entry.width, entry.height) {
            (Some(width), Some(height)) if width > 0 && height > 0 => {
                format!("Image {width} x {height}")
            }
            _ => "Clipboard image".to_string(),
        };
    }
    let thumbnail_data_url = if is_image_row {
        preview::image_row_data_url(entry.thumbnail_path.as_deref(), entry.image_path.as_deref())
    } else {
        None
    };
    SearchResult {
        id: format!("clip:{}", entry.id),
        kind: SourceKind::Clipboard,
        title: if title.is_empty() {
            "Clipboard item".to_string()
        } else {
            title
        },
        subtitle: {
            let base = if entry.source_app.is_empty() {
                "Clipboard history".to_string()
            } else {
                format!("Clipboard - {}", entry.source_app)
            };
            if entry.pinned {
                format!("{base} - pinned")
            } else {
                base
            }
        },
        path: if is_image_row {
            entry.image_path.clone()
        } else {
            None
        },
        icon: if is_image_row {
            IconRef::FileType {
                ext: "png".to_string(),
            }
        } else {
            IconRef::Monogram
        },
        score: 0.0,
        match_ranges: Vec::new(),
        meta: ResultMeta {
            modified: Some(entry.timestamp_ms),
            // Category drives the UI badge/filter: LINK, EMAIL, PATH, CODE,
            // IMAGE, FILES or TEXT; `pinned` marks pinned items.
            label: Some(clip_category_label(entry.category).to_string()),
            thumbnail_data_url,
            pinned: entry.pinned,
            image_width: entry.width,
            image_height: entry.height,
            mime: entry.mime.clone().or_else(|| {
                if is_image_row {
                    Some("image/png".to_string())
                } else {
                    None
                }
            }),
            ..ResultMeta::default()
        },
        action: if is_image_row {
            match entry.image_path.as_deref() {
                Some(image_path) => ActionSpec::CopyImage {
                    path: image_path.to_string(),
                },
                None => ActionSpec::None,
            }
        } else {
            ActionSpec::CopyText {
                text: entry.text.clone(),
            }
        },
        actions: Vec::new(),
    }
}

fn browser_result(
    entry: &ps_sources::BrowserEntry,
    favicons: &HashMap<String, String>,
) -> SearchResult {
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
                if entry.source.is_empty() {
                    "Bookmark".into()
                } else {
                    format!("{} · Bookmark", entry.source)
                }
            } else if entry.source.is_empty() {
                format!("{} visits", entry.visit_count.max(1))
            } else {
                format!("{} · {} visits", entry.source, entry.visit_count.max(1))
            }),
            favicon_data_url: ps_sources::browser::host_of(&entry.url)
                .and_then(|host| favicons.get(&host).cloned()),
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

/// Rows for the Git card: every repository (newest commit first) so the card can
/// offer them all, then up to `limit` commits newest first.
fn git_activity(entries: &[ps_sources::GitEntry], limit: usize) -> Vec<&ps_sources::GitEntry> {
    use ps_sources::GitEntry;
    let mut commits: Vec<(&GitEntry, i64, &str)> = entries
        .iter()
        .filter_map(|entry| match entry {
            GitEntry::Commit { repo, date_ms, .. } => Some((entry, *date_ms, repo.as_str())),
            _ => None,
        })
        .collect();
    commits.sort_by_key(|(_, date_ms, _)| std::cmp::Reverse(*date_ms));
    let newest = |path: &str| commits.iter().find(|(_, _, repo)| *repo == path).map_or(i64::MIN, |(_, date_ms, _)| *date_ms);
    let mut repos: Vec<(&GitEntry, i64)> = entries
        .iter()
        .filter_map(|entry| match entry {
            GitEntry::Repo { path, .. } => Some((entry, newest(path))),
            _ => None,
        })
        .collect();
    repos.sort_by_key(|(_, date_ms)| std::cmp::Reverse(*date_ms));
    repos
        .into_iter()
        .map(|(entry, _)| entry)
        .chain(commits.into_iter().take(limit).map(|(entry, _, _)| entry))
        .collect()
}

pub fn git_result(entry: &ps_sources::GitEntry) -> SearchResult {
    match entry {
        ps_sources::GitEntry::Repo { path, name, branch, branches } => SearchResult {
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
            meta: ResultMeta {
                branches: branches.clone(),
                ..ResultMeta::default()
            },
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
            action: ActionSpec::CopyText { text: hash.clone() },
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
            // OCR text is the preview body: never drop it from the result.
            snippet: Some(truncate_chars(&hit.text, 400)),
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
        ActionSpec::RevealPath { path } => (
            ActivityKind::Open,
            "Reveal in Explorer".into(),
            Some(path.clone()),
        ),
        ActionSpec::OpenUrl { url } => (ActivityKind::Open, url.clone(), None),
        ActionSpec::LaunchApp { app_id, .. } => (ActivityKind::Launch, app_id.clone(), None),
        ActionSpec::CopyText { text } => (ActivityKind::Open, text.chars().take(80).collect(), None),
        ActionSpec::CopyImage { path } => (
            ActivityKind::Open,
            format!("Copied image: {path}"),
            Some(path.clone()),
        ),
        ActionSpec::RunNative { command } => (ActivityKind::Workflow, format!("{command:?}"), None),
        ActionSpec::RunWorkflow { id } => (ActivityKind::Workflow, id.clone(), None),
        ActionSpec::None => (ActivityKind::System, "No action".into(), None),
    }
}

fn native_action_label(command: NativeCommand) -> &'static str {
    match command {
        NativeCommand::LockWorkstation => "lock this PC",
        NativeCommand::Sleep => "put this PC to sleep",
        NativeCommand::Shutdown => "shut down this PC",
        NativeCommand::Restart => "restart this PC",
        NativeCommand::EmptyRecycleBin => "empty the Recycle Bin",
        NativeCommand::OpenSettings => "open Windows Settings",
        NativeCommand::OpenTaskManager => "open Task Manager",
        NativeCommand::OpenTerminal => "open a terminal",
        NativeCommand::ToggleDarkMode => "toggle the theme",
        NativeCommand::PauseIndexing => "pause indexing",
        NativeCommand::ResumeIndexing => "resume indexing",
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

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("target")
            .join("ps-core-tests")
            .join(format!("engine-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Engine for fixtures with made-up paths: results are not checked
    /// against the disk (see stale_results_are_dropped_and_forgotten).
    fn make_engine(dir: &Path, legacy: Option<PathBuf>) -> Engine {
        let engine = Engine::open(EngineConfig {
            data_dir: dir.to_path_buf(),
            legacy_db: legacy,
            version: "test".to_string(),
        })
        .unwrap();
        engine.verify_paths.store(false, Ordering::Relaxed);
        engine
    }

    // -- filter ranking (All / Files / Folders / Apps) ------------------------

    fn entry(path: &str, kind: EntryKind, modified: i64) -> ps_index::IndexEntry {
        let name = path.rsplit('\\').next().unwrap().to_string();
        let ext = match kind {
            EntryKind::File => name.rsplit_once('.').map(|(_, ext)| ext.to_string()).unwrap_or_default(),
            _ => String::new(),
        };
        ps_index::IndexEntry {
            path: path.into(),
            name,
            ext,
            kind,
            size: 1,
            modified,
        }
    }

    fn app(name: &str, target: &str) -> ps_sources::AppEntry {
        ps_sources::AppEntry {
            id: target.into(),
            name: name.into(),
            target: target.into(),
            icon_path: None,
            keywords: Vec::new(),
        }
    }

    fn ranked(engine: &Engine, query: &str, scope: SearchScope) -> Vec<(SourceKind, String)> {
        let mut request = SearchRequest::new(1, query);
        request.scope = scope;
        engine
            .search(&request)
            .results
            .into_iter()
            .map(|result| (result.kind, result.title))
            .collect()
    }

    fn ranking_engine(name: &str, entries: Vec<ps_index::IndexEntry>, apps: Vec<ps_sources::AppEntry>) -> (PathBuf, Engine) {
        let dir = scratch_dir(name);
        let engine = make_engine(&dir, None);
        engine.settings.write().unwrap().web_search_enabled = false;
        *engine.index.write().unwrap() = FilenameIndex::from_entries(entries);
        engine.sync_filename_entries();
        engine.sources.write().unwrap().apps = apps;
        (dir, engine)
    }

    #[test]
    fn all_ranks_relevance_first_then_apps_folders_files() {
        let (dir, engine) = ranking_engine(
            "rank-all",
            vec![
                entry(r"C:\work\budget", EntryKind::Folder, 1),
                entry(r"C:\work\budget.xlsx", EntryKind::File, 1),
                entry(r"C:\work\budget planner notes.txt", EntryKind::File, 1),
                entry(r"C:\work\budgets.xlsx", EntryKind::File, 1),
                entry(r"C:\old\b_u_d_g_e_t.txt", EntryKind::File, 99),
            ],
            vec![
                app("Budget", r"C:\apps\budget-app.exe"),
                app("Home Budget Pro", r"C:\apps\hbp.exe"),
                app("Budgeteer", r"C:\apps\bgt.exe"),
            ],
        );
        let rows = ranked(&engine, "budget", SearchScope::All);
        let expected = vec![
            // exact names: app, then folder, then file (the stem is the name)
            (SourceKind::Application, "Budget".to_string()),
            (SourceKind::Folder, "budget".to_string()),
            (SourceKind::File, "budget.xlsx".to_string()),
            // whole-word matches
            (SourceKind::Application, "Home Budget Pro".to_string()),
            (SourceKind::File, "budget planner notes.txt".to_string()),
            // prefixes
            (SourceKind::Application, "Budgeteer".to_string()),
            (SourceKind::File, "budgets.xlsx".to_string()),
            // weak fuzzy last even though it is the newest file
            (SourceKind::File, "b_u_d_g_e_t.txt".to_string()),
        ];
        assert_eq!(rows, expected);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn all_puts_an_installed_app_above_files_named_after_it() {
        // Real-data case: deep toolchain files called chrome.dart buried
        // Google Chrome; the app's own alias (chrome.exe) is its name in All.
        let (dir, engine) = ranking_engine(
            "rank-all-alias",
            vec![
                entry(r"C:\sdk\flutter\lib\web\chrome.dart", EntryKind::File, 50),
                entry(r"C:\sdk\flutter\dev\chrome.dart", EntryKind::File, 50),
            ],
            vec![app("Google Chrome", r"C:\Program Files\Google\Chrome\Application\chrome.exe")],
        );
        let rows = ranked(&engine, "chrome", SearchScope::All);
        assert_eq!(rows[0], (SourceKind::Application, "Google Chrome".to_string()), "{rows:?}");
        assert_eq!(rows.len(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn all_never_puts_a_weaker_app_above_an_exact_file() {
        let (dir, engine) = ranking_engine(
            "rank-all-exact-file",
            vec![entry(r"C:\notes\notes.txt", EntryKind::File, 1)],
            vec![
                app("Sticky Notes", r"C:\apps\stickynotes.exe"),
                app("Notesnook", r"C:\apps\nn.exe"),
            ],
        );
        let rows = ranked(&engine, "notes", SearchScope::All);
        assert_eq!(rows[0], (SourceKind::File, "notes.txt".to_string()), "{rows:?}");
        assert_eq!(rows.len(), 3, "{rows:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn files_filter_follows_its_ladder_and_history() {
        let (dir, engine) = ranking_engine(
            "rank-files",
            vec![
                entry(r"C:\a\report", EntryKind::File, 1),
                entry(r"C:\a\report.pdf", EntryKind::File, 1),
                entry(r"C:\a\annual-report.txt", EntryKind::File, 1),
                entry(r"C:\a\reports.txt", EntryKind::File, 1),
                entry(r"C:\a\annual-reports.txt", EntryKind::File, 1),
                entry(r"C:\a\myreport.txt", EntryKind::File, 1),
                entry(r"C:\a\r_e_p_o_r_t.txt", EntryKind::File, 9_999),
                entry(r"C:\a\report-folder", EntryKind::Folder, 1),
            ],
            vec![app("Report Builder", r"C:\apps\rb.exe")],
        );
        let names: Vec<String> = ranked(&engine, "report", SearchScope::Files).into_iter().map(|(_, t)| t).collect();
        assert_eq!(
            names,
            vec![
                "report",
                "report.pdf",
                "annual-report.txt",
                "reports.txt",
                "annual-reports.txt",
                "myreport.txt",
                "r_e_p_o_r_t.txt",
            ],
            "full name, stem, token, prefix, word prefix, substring, fuzzy; files only"
        );

        // Inside one class: opened files first, then the newest.
        *engine.index.write().unwrap() = FilenameIndex::from_entries(vec![
            entry(r"C:\a\plan.docx", EntryKind::File, 10),
            entry(r"C:\b\plan.docx", EntryKind::File, 50),
            entry(r"C:\c\plan.docx", EntryKind::File, 30),
        ]);
        let paths = |engine: &Engine| -> Vec<String> {
            let mut request = SearchRequest::new(2, "plan");
            request.scope = SearchScope::Files;
            engine.search(&request).results.into_iter().filter_map(|r| r.path).collect()
        };
        assert_eq!(paths(&engine), vec![r"C:\b\plan.docx", r"C:\c\plan.docx", r"C:\a\plan.docx"]);
        engine.usage.write().unwrap().insert(
            r"c:\a\plan.docx".into(),
            Usage {
                count: 6,
                last_ms: now_ms(),
            },
        );
        assert_eq!(paths(&engine)[0], r"C:\a\plan.docx", "frequently opened file first");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn folders_filter_shows_only_folders_shallow_first() {
        let (dir, engine) = ranking_engine(
            "rank-folders",
            vec![
                entry(r"C:\Users\X\Documents\old\backup\Projects", EntryKind::Folder, 9),
                entry(r"C:\Users\X\Documents\Projects", EntryKind::Folder, 1),
                entry(r"C:\Users\X\Documents\Projects.zip", EntryKind::File, 5),
                entry(r"C:\Users\X\Documents\Side Projects", EntryKind::Folder, 1),
            ],
            Vec::new(),
        );
        let mut request = SearchRequest::new(3, "projects");
        request.scope = SearchScope::Folders;
        let results = engine.search(&request).results;
        let paths: Vec<&str> = results.iter().filter_map(|r| r.path.as_deref()).collect();
        assert_eq!(
            paths,
            vec![
                r"C:\Users\X\Documents\Projects",
                r"C:\Users\X\Documents\old\backup\Projects",
                r"C:\Users\X\Documents\Side Projects",
            ]
        );
        assert!(results.iter().all(|r| r.kind == SourceKind::Folder));
        assert_ne!(results[0].subtitle, results[1].subtitle, "same-name folders show their location");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn apps_filter_uses_aliases_history_and_keeps_installs_apart() {
        let (dir, engine) = ranking_engine(
            "rank-apps",
            vec![entry(r"C:\Users\X\chrome", EntryKind::Folder, 1)],
            vec![
                app("Chrome Remote Desktop", r"C:\apps\remoting_host.exe"),
                app("Google Chrome", r"C:\Program Files\Google\Chrome\Application\chrome.exe"),
                app("Visual Studio Code", r"C:\Users\X\AppData\Local\Programs\Microsoft VS Code\Code.exe"),
                app("Python 3.12", r"C:\Python312\python.exe"),
                app("Python 3.12", r"C:\Users\X\AppData\Local\Programs\Python\Python312\python.exe"),
                app("Google Docs", r"C:\apps\docs.exe"),
                app("Google Drive", r"C:\apps\drive.exe"),
            ],
        );
        let apps = ranked(&engine, "chrome", SearchScope::Apps);
        assert_eq!(apps[0].1, "Google Chrome", "the exe alias beats a name prefix: {apps:?}");
        assert!(apps.iter().all(|(kind, _)| *kind == SourceKind::Application));
        assert_eq!(ranked(&engine, "vscode", SearchScope::Apps)[0].1, "Visual Studio Code");
        assert_eq!(ranked(&engine, "vsc", SearchScope::Apps)[0].1, "Visual Studio Code");
        assert_eq!(ranked(&engine, "crome", SearchScope::Apps)[0].1, "Google Chrome", "one typo");

        let mut request = SearchRequest::new(4, "python");
        request.scope = SearchScope::Apps;
        let pythons = engine.search(&request).results;
        assert_eq!(pythons.len(), 2, "two installs stay distinguishable");
        assert_ne!(pythons[0].subtitle, pythons[1].subtitle);

        let google = ranked(&engine, "google d", SearchScope::Apps);
        assert_eq!(google[0].1, "Google Docs", "alphabetical without history");
        engine.usage.write().unwrap().insert(
            app_usage_key(r"C:\apps\drive.exe"),
            Usage {
                count: 12,
                last_ms: now_ms(),
            },
        );
        assert_eq!(ranked(&engine, "google d", SearchScope::Apps)[0].1, "Google Drive", "launched often");

        // Uninstalled: gone from the app list, gone from results.
        engine
            .sources
            .write()
            .unwrap()
            .apps
            .retain(|app| app.name != "Chrome Remote Desktop");
        assert!(!ranked(&engine, "chrome", SearchScope::Apps)
            .iter()
            .any(|(_, title)| title == "Chrome Remote Desktop"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn one_letter_queries_only_match_word_starts() {
        let (dir, engine) = ranking_engine(
            "rank-one-letter",
            vec![
                entry(r"C:\a\report.txt", EntryKind::File, 1),
                entry(r"C:\a\annual-review.txt", EntryKind::File, 1),
                entry(r"C:\a\barn.txt", EntryKind::File, 1),
            ],
            vec![app("Notepad", r"C:\apps\notepad.exe")],
        );
        let rows = ranked(&engine, "r", SearchScope::Files);
        let names: Vec<&str> = rows.iter().map(|(_, title)| title.as_str()).collect();
        assert_eq!(names, vec!["report.txt", "annual-review.txt"]);
        assert!(ranked(&engine, "e", SearchScope::Apps).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn name_and_ocr_matches_for_one_file_show_once() {
        let (dir, engine) = ranking_engine(
            "rank-dedupe",
            vec![
                entry(r"C:\docs\invoice.pdf", EntryKind::File, 1),
                entry(r"C:\docs\scan-0042.png", EntryKind::File, 1),
            ],
            Vec::new(),
        );
        engine.settings.write().unwrap().indexing.ocr_enabled = true;
        for path in [r"C:\docs\invoice.pdf", r"C:\docs\scan-0042.png"] {
            engine
                .db
                .ocr_upsert(path, "hash", "invoice total amount due", "en", 10, 10, 10, 1)
                .unwrap();
        }
        let mut request = SearchRequest::new(5, "invoice");
        request.scope = SearchScope::All;
        let results = engine.search(&request).results;
        let invoice_rows = results
            .iter()
            .filter(|r| r.path.as_deref() == Some(r"C:\docs\invoice.pdf"))
            .count();
        assert_eq!(invoice_rows, 1, "{:?}", results.iter().map(|r| (&r.title, r.kind)).collect::<Vec<_>>());
        assert_eq!(results[0].kind, SourceKind::File, "the name match is the one shown");
        let last = results.last().unwrap();
        assert_eq!(last.kind, SourceKind::OcrText, "text-only match comes after name matches");

        request.scope = SearchScope::Files;
        let files = engine.search(&request).results;
        assert!(files.iter().all(|r| r.kind == SourceKind::File));
        assert!(!files.iter().any(|r| r.path.as_deref() == Some(r"C:\docs\scan-0042.png")), "no content-only rows in Files");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stale_results_are_dropped_and_forgotten() {
        let dir = scratch_dir("rank-stale");
        let files = dir.join("files");
        std::fs::create_dir_all(&files).unwrap();
        let keep = files.join("quarterly-keep.txt");
        let gone = files.join("quarterly-gone.txt");
        std::fs::write(&keep, "x").unwrap();
        std::fs::write(&gone, "x").unwrap();
        let engine = Engine::open(EngineConfig {
            data_dir: dir.clone(),
            legacy_db: None,
            version: "test".into(),
        })
        .unwrap();
        *engine.index.write().unwrap() = FilenameIndex::from_entries(vec![
            entry(keep.to_str().unwrap(), EntryKind::File, 1),
            entry(gone.to_str().unwrap(), EntryKind::File, 1),
        ]);
        std::fs::remove_file(&gone).unwrap();
        let mut request = SearchRequest::new(6, "quarterly");
        request.scope = SearchScope::Files;
        let titles: Vec<String> = engine.search(&request).results.into_iter().map(|r| r.title).collect();
        assert_eq!(titles, vec!["quarterly-keep.txt"]);
        assert!(engine.index.read().unwrap().search("quarterly-gone", 5).is_empty(), "forgotten by the index");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn web_fallback_is_the_last_row_in_all() {
        let (dir, engine) = ranking_engine(
            "rank-web",
            vec![entry(r"C:\a\budget.xlsx", EntryKind::File, 1), entry(r"C:\a\b_u_d_g_e_t.md", EntryKind::File, 1)],
            Vec::new(),
        );
        engine.settings.write().unwrap().web_search_enabled = true;
        let rows = ranked(&engine, "budget", SearchScope::All);
        assert_eq!(rows[0], (SourceKind::File, "budget.xlsx".to_string()));
        assert_eq!(rows.last().unwrap().0, SourceKind::Action, "{rows:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn metadata_backfill_stops_on_cancel_and_resumes_from_pending_rows() {
        let dir = scratch_dir("metadata-backfill-resume");
        let index = FilenameIndex::from_entries(vec![ps_index::IndexEntry {
            path: r"C:\photos\old.png".into(),
            name: "old.png".into(),
            ext: "png".into(),
            kind: EntryKind::File,
            size: 42,
            modified: 1234,
        }]);
        index.save(&dir.join(INDEX_FILE)).unwrap();
        let engine = make_engine(&dir, None);
        engine
            .db
            .ocr_upsert(r"C:\photos\old.png", "hash", "legacy", "en", 1, 1, 0, 0)
            .unwrap();
        rusqlite::Connection::open(dir.join(DB_FILE))
            .unwrap()
            .execute("UPDATE ocr_documents SET path_lower = ''", [])
            .unwrap();

        engine.shutdown.store(true, Ordering::SeqCst);
        engine.backfill_ocr_metadata().unwrap();
        assert_eq!(engine.db.ocr_metadata_pending().unwrap(), 1);

        engine.shutdown.store(false, Ordering::SeqCst);
        engine.backfill_ocr_metadata().unwrap();
        assert_eq!(engine.db.ocr_metadata_pending().unwrap(), 0);
        let hits = engine
            .db
            .ocr_search_photos(
                "legacy",
                5,
                &SearchFilters {
                    under_path: Some(r"c:\photos\old.png".into()),
                    modified_after: Some(1234),
                    min_size: Some(42),
                    ..SearchFilters::default()
                },
            )
            .unwrap();
        assert_eq!(hits.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn background_backfill_uses_index_created_by_fresh_scan() {
        let dir = scratch_dir("metadata-backfill-fresh-scan");
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).unwrap();
        let photo = root.join("old.png");
        std::fs::write(&photo, b"image placeholder").unwrap();
        let engine = std::sync::Arc::new(make_engine(&dir, None));
        let path = photo.to_string_lossy().into_owned();
        engine
            .db
            .ocr_upsert(&path, "hash", "legacy", "en", 1, 1, 0, 0)
            .unwrap();
        let orphan = root.join("removed.png").to_string_lossy().into_owned();
        engine
            .db
            .ocr_upsert(&orphan, "orphan", "orphan legacy", "en", 1, 1, 0, 0)
            .unwrap();
        rusqlite::Connection::open(dir.join(DB_FILE))
            .unwrap()
            .execute("UPDATE ocr_documents SET path_lower = ''", [])
            .unwrap();
        {
            let mut settings = engine.settings.write().unwrap();
            settings.onboarding_complete = true;
            settings.indexing.roots = vec![ps_ipc::RootConfig {
                path: root.to_string_lossy().into_owned(),
                mode: ps_ipc::RootMode::NamesOnly,
                enabled: true,
            }];
        }
        engine.clone().spawn_background();
        for _ in 0..500 {
            if engine.status().state == IndexerState::Ready {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(engine.db.ocr_metadata_pending().unwrap(), 1);
        assert!(engine
            .status()
            .warnings
            .iter()
            .any(|warning| warning.contains("1 OCR rows still need filename metadata")));
        assert_eq!(
            engine
                .db
                .ocr_search_photos(
                    "legacy",
                    5,
                    &SearchFilters {
                        min_size: Some(17),
                        ..SearchFilters::default()
                    }
                )
                .unwrap()
                .len(),
            1
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    fn sample_entries(n: usize) -> Vec<ps_index::IndexEntry> {
        (0..n)
            .map(|i| ps_index::IndexEntry {
                path: format!(r"C:\test\file{i}.txt"),
                name: format!("file{i}.txt"),
                ext: "txt".into(),
                kind: EntryKind::File,
                size: i as u64,
                modified: i as i64,
            })
            .collect()
    }

    fn indexed_files(status: SearchStatus) -> u64 {
        match status {
            SearchStatus::Ready { indexed_files, .. } => indexed_files,
            SearchStatus::Indexing { indexed_files, .. } => indexed_files,
            SearchStatus::Degraded { reason } => panic!("unexpected degraded status: {reason}"),
        }
    }

    fn make_legacy_fixture(path: &Path) {
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.execute_batch(
            "CREATE TABLE files (path TEXT PRIMARY KEY, name TEXT NOT NULL, extension TEXT NOT NULL,
                modified INTEGER NOT NULL, size INTEGER NOT NULL, is_dir INTEGER NOT NULL,
                last_seen_scan INTEGER NOT NULL DEFAULT 0);
             INSERT INTO files VALUES ('C:/Docs/a.txt','a.txt','txt',1700000000,12,0,1);
             INSERT INTO files VALUES ('C:/Docs','Docs','',1700000000,0,1,1);
             INSERT INTO files VALUES ('C:/Docs/b.pdf','b.pdf','pdf',1700000001,1200,0,1);",
        )
        .unwrap();
    }

    #[test]
    fn ocr_workers_take_each_candidate_once_and_stop_when_paused() {
        let dir = scratch_dir("ocr-workers");
        let engine = make_engine(&dir, None);
        // Non-image names: extraction returns early, so only the queue and pause logic run.
        let candidates: Vec<(String, u64, i64)> =
            (0..40).map(|i| (format!(r"C:\pool\note-{i}.txt"), 1, 1)).collect();
        let queue = Mutex::new(candidates.into_iter());
        let processed = AtomicU64::new(0);
        engine.paused.store(true, Ordering::SeqCst);
        ocr_worker(&engine, &queue, &processed, "windows", None);
        assert_eq!(processed.load(Ordering::SeqCst), 0, "a paused worker takes nothing");
        engine.paused.store(false, Ordering::SeqCst);
        std::thread::scope(|scope| {
            for _ in 0..ocr_worker_count() {
                scope.spawn(|| ocr_worker(&engine, &queue, &processed, "windows", None));
            }
        });
        assert_eq!(processed.load(Ordering::SeqCst), 40);
        assert!(queue.lock().unwrap().next().is_none());
    }

    #[test]
    fn ocr_candidates_include_images_past_legacy_limit_and_empty_files() {
        let entries = (0..450)
            .map(|i| ps_index::IndexEntry {
                path: format!(r"C:\images\scan{i}.png"),
                name: format!("scan{i}.png"),
                ext: "png".into(),
                kind: EntryKind::File,
                size: if i == 0 { 0 } else { 1 },
                modified: i as i64,
            })
            .collect();
        let index = FilenameIndex::from_entries(entries);
        let candidates = ocr_candidates(&index, &[(r"c:\images\".into(), 2)]);

        assert_eq!(candidates.len(), 450);
        assert_eq!(candidates[0].1, 0, "empty images remain eligible for OCR");
        assert!(candidates
            .iter()
            .any(|(path, _, _)| path.ends_with("scan449.png")));
    }

    #[test]
    fn status_and_search_report_injected_index_count() {
        let dir = scratch_dir("status-count");
        let engine = make_engine(&dir, None);
        let n = 7usize;
        *engine.index.write().unwrap() = FilenameIndex::from_entries(sample_entries(n));
        // Deliberately do NOT touch the cached counter: status()/search() must
        // read the index itself and heal the cache, so the footer can never
        // show "0 items" while entries are searchable.
        let status = engine.status();
        assert_eq!(status.filename_entries, n as u64, "status count");
        assert_eq!(
            engine.filename_entries.load(Ordering::Relaxed),
            n as u64,
            "status heals the cached counter"
        );
        let resp = engine.search(&SearchRequest::new(1, "file"));
        assert_eq!(indexed_files(resp.status), n as u64, "search status count");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn persisted_index_sets_counter_at_open() {
        let dir = scratch_dir("open-count");
        let index = FilenameIndex::from_entries(sample_entries(5));
        index.save(&dir.join(INDEX_FILE)).unwrap();
        let engine = make_engine(&dir, None);
        assert_eq!(engine.status().filename_entries, 5, "loaded index count");
        assert_eq!(engine.filename_entries.load(Ordering::Relaxed), 5);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migration_publishes_filename_counter() {
        let dir = scratch_dir("migration-count");
        let v2 = dir.join("v2");
        std::fs::create_dir_all(&v2).unwrap();
        let legacy = dir.join("legacy.db");
        make_legacy_fixture(&legacy);
        let engine = make_engine(&v2, Some(legacy.clone()));
        assert_eq!(engine.status().filename_entries, 0, "nothing loaded yet");
        engine.migrate_legacy(&legacy);
        assert_eq!(engine.index.read().unwrap().len(), 3);
        assert_eq!(
            engine.status().filename_entries,
            3,
            "migrated index must be counted immediately"
        );
        assert_eq!(engine.filename_entries.load(Ordering::Relaxed), 3);
        assert!(engine.settings().legacy_migrated);
        let resp = engine.search(&SearchRequest::new(2, "docs"));
        assert_eq!(indexed_files(resp.status), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn finish_background_always_marks_ready_and_syncs() {
        let dir = scratch_dir("finalize");
        let engine = Arc::new(make_engine(&dir, None));
        *engine.index.write().unwrap() = FilenameIndex::from_entries(sample_entries(4));
        engine.set_state(STATE_INDEXING);
        engine.set_phase("indexing content");
        finish_background(&engine);
        let status = engine.status();
        assert_eq!(
            status.state,
            IndexerState::Ready,
            "pipeline exit must not leave the UI on Indexing"
        );
        assert_eq!(status.filename_entries, 4);
        // A paused pipeline finishes as Paused, never stuck on Indexing.
        engine.paused.store(true, Ordering::SeqCst);
        engine.set_state(STATE_INDEXING);
        finish_background(&engine);
        assert_eq!(engine.status().state, IndexerState::Paused);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rebuild_command_zeroes_count_and_status_stays_truthful() {
        let dir = scratch_dir("rebuild");
        let engine = make_engine(&dir, None);
        *engine.index.write().unwrap() = FilenameIndex::from_entries(sample_entries(6));
        engine.sync_filename_entries();
        assert_eq!(engine.status().filename_entries, 6);
        engine.index_command(IndexCommand::Rebuild).unwrap();
        assert_eq!(engine.index.read().unwrap().len(), 0);
        assert_eq!(
            engine.status().filename_entries,
            0,
            "rebuild clears the index, so the count must be 0"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn strip_imperative_verbs_helper() {
        assert_eq!(strip_imperative_verbs("open chrome").as_ref(), "chrome");
        assert_eq!(strip_imperative_verbs("Open Notepad").as_ref(), "Notepad");
        assert_eq!(
            strip_imperative_verbs("launch visual studio code").as_ref(),
            "visual studio code"
        );
        assert_eq!(strip_imperative_verbs("go to downloads").as_ref(), "downloads");
        assert_eq!(strip_imperative_verbs("switch to edge").as_ref(), "edge");
        assert_eq!(strip_imperative_verbs("open notepad please").as_ref(), "notepad");
        assert_eq!(strip_imperative_verbs("chrome please").as_ref(), "chrome");
        // Only-a-verb / only-"please" queries stay literal.
        assert_eq!(strip_imperative_verbs("open").as_ref(), "open");
        assert_eq!(strip_imperative_verbs("open ").as_ref(), "open");
        assert_eq!(strip_imperative_verbs("please").as_ref(), "please");
        // Filters survive stripping.
        assert_eq!(
            strip_imperative_verbs("open report ext:pdf").as_ref(),
            "report ext:pdf"
        );
    }

    #[test]
    fn imperative_verbs_are_stripped_for_apps_and_files() {
        let dir = scratch_dir("verbs");
        let engine = make_engine(&dir, None);
        *engine.index.write().unwrap() = FilenameIndex::from_entries(vec![ps_index::IndexEntry {
            path: r"C:\docs\architecture notes.txt".into(),
            name: "architecture notes.txt".into(),
            ext: "txt".into(),
            kind: EntryKind::File,
            size: 10,
            modified: 1,
        }]);
        engine.sync_filename_entries();
        {
            let mut sources = engine.sources.write().unwrap();
            sources.apps = vec![
                ps_sources::AppEntry {
                    id: "chrome".into(),
                    name: "Google Chrome".into(),
                    target: "chrome.exe".into(),
                    icon_path: None,
                    keywords: Vec::new(),
                },
                ps_sources::AppEntry {
                    id: "vscode".into(),
                    name: "Visual Studio Code".into(),
                    target: "code.exe".into(),
                    icon_path: None,
                    keywords: Vec::new(),
                },
                ps_sources::AppEntry {
                    id: "notepad".into(),
                    name: "Notepad".into(),
                    target: "notepad.exe".into(),
                    icon_path: None,
                    keywords: Vec::new(),
                },
            ];
        }
        let has_app = |resp: &SearchResponse, needle: &str| {
            resp.results
                .iter()
                .any(|r| r.kind == SourceKind::Application && r.title.to_lowercase().contains(needle))
        };
        assert!(
            has_app(&engine.search(&SearchRequest::new(20, "open chrome")), "chrome"),
            "open chrome must find the Chrome app"
        );
        assert!(
            has_app(
                &engine.search(&SearchRequest::new(21, "launch visual studio code")),
                "visual studio code"
            ),
            "launch visual studio code must find VS Code"
        );
        assert!(
            has_app(&engine.search(&SearchRequest::new(22, "open notepad")), "notepad"),
            "open notepad must find Notepad"
        );
        assert!(
            has_app(
                &engine.search(&SearchRequest::new(23, "open notepad please")),
                "notepad"
            ),
            "trailing please must be stripped"
        );

        let file_resp = engine.search(&SearchRequest::new(24, "open architecture notes"));
        assert!(
            file_resp.results.iter().any(|r| r
                .path
                .as_deref()
                .map(|p| p.ends_with("architecture notes.txt"))
                .unwrap_or(false)),
            "open architecture notes must find the file: {:?}",
            file_resp
                .results
                .iter()
                .map(|r| (&r.title, r.kind))
                .collect::<Vec<_>>()
        );

        // A query that is only a verb must stay literal.
        *engine.index.write().unwrap() = FilenameIndex::from_entries(vec![ps_index::IndexEntry {
            path: r"C:\docs\open.txt".into(),
            name: "open.txt".into(),
            ext: "txt".into(),
            kind: EntryKind::File,
            size: 1,
            modified: 1,
        }]);
        engine.sync_filename_entries();
        let resp = engine.search(&SearchRequest::new(25, "open"));
        assert!(
            resp.results.iter().any(|r| r
                .path
                .as_deref()
                .map(|p| p.ends_with("open.txt"))
                .unwrap_or(false)),
            "literal 'open' search must still work"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scope_allow_matrix_is_strict_for_files_and_folders() {
        assert!(scope_allows(SearchScope::Files, SourceKind::File));
        assert!(!scope_allows(SearchScope::Files, SourceKind::Folder));
        assert!(scope_allows(SearchScope::Folders, SourceKind::Folder));
        assert!(!scope_allows(SearchScope::Folders, SourceKind::File));
        assert!(scope_allows(SearchScope::All, SourceKind::File));
        assert!(scope_allows(SearchScope::All, SourceKind::Folder));
        assert!(scope_allows(SearchScope::Photos, SourceKind::File));
        assert!(scope_allows(SearchScope::Photos, SourceKind::OcrText));
        assert!(!scope_allows(SearchScope::Photos, SourceKind::Folder));
        assert!(scope_allows(SearchScope::Ocr, SourceKind::OcrText));
        assert!(scope_allows(SearchScope::Ocr, SourceKind::ContentMatch));
        assert!(scope_allows(SearchScope::Ocr, SourceKind::File));
        assert!(!scope_allows(SearchScope::Ocr, SourceKind::Folder));
    }
    #[test]
    fn photos_scope_returns_raster_files_and_raster_ocr_only() {
        let dir = scratch_dir("photos-scope");
        let engine = make_engine(&dir, None);
        *engine.index.write().unwrap() = FilenameIndex::from_entries(vec![
            ps_index::IndexEntry {
                path: r"C:\photos\sunset.png".into(),
                name: "sunset.png".into(),
                ext: "png".into(),
                kind: EntryKind::File,
                size: 10,
                modified: 1,
            },
            ps_index::IndexEntry {
                path: r"C:\photos\sunset.pdf".into(),
                name: "sunset.pdf".into(),
                ext: "pdf".into(),
                kind: EntryKind::File,
                size: 10,
                modified: 1,
            },
            ps_index::IndexEntry {
                path: r"C:\photos\sunset".into(),
                name: "sunset".into(),
                ext: String::new(),
                kind: EntryKind::Folder,
                size: 0,
                modified: 1,
            },
        ]);
        engine.sync_filename_entries();
        engine
            .db
            .ocr_upsert(
                r"C:\photos\sunset.png",
                "png-hash",
                "violet nebula phrase",
                "en",
                10,
                10,
                10,
                1,
            )
            .unwrap();
        engine
            .db
            .ocr_upsert(
                r"C:\photos\sunset.pdf",
                "pdf-hash",
                "violet nebula phrase",
                "en",
                10,
                10,
                10,
                1,
            )
            .unwrap();
        engine
            .db
            .ocr_upsert(
                r"C:\photos-private\sunset.png",
                "sibling-hash",
                "violet nebula phrase",
                "en",
                10,
                10,
                10,
                1,
            )
            .unwrap();
        assert_eq!(
            engine
                .db
                .ocr_search_photos("violet nebula", 5, &SearchFilters::default())
                .unwrap()
                .len(),
            2
        );
        engine.settings.write().unwrap().indexing.ocr_enabled = true;

        let mut request = SearchRequest::new(40, "sunset");
        request.scope = SearchScope::Photos;
        let results = engine.search(&request).results;
        assert!(!results.is_empty());
        assert!(
            results.iter().all(|result| {
                result.kind == SourceKind::OcrText
                    || result.path.as_deref().is_some_and(|path| {
                        path.to_ascii_lowercase().ends_with(".png")
                            || path.to_ascii_lowercase().ends_with(".jpg")
                            || path.to_ascii_lowercase().ends_with(".jpeg")
                            || path.to_ascii_lowercase().ends_with(".bmp")
                            || path.to_ascii_lowercase().ends_with(".gif")
                            || path.to_ascii_lowercase().ends_with(".webp")
                            || path.to_ascii_lowercase().ends_with(".tif")
                            || path.to_ascii_lowercase().ends_with(".tiff")
                    })
            }),
            "Photos must exclude PDFs and folders: {:?}",
            results.iter().map(|r| (&r.path, r.kind)).collect::<Vec<_>>()
        );

        request.query = "violet nebula".into();
        request.filters.under_path = Some(r"C:\photos".into());
        let response = engine.search(&request);
        let ocr_paths: Vec<_> = response
            .results
            .iter()
            .filter(|result| result.kind == SourceKind::OcrText)
            .filter_map(|result| result.path.as_deref())
            .collect();
        assert_eq!(ocr_paths, vec![r"C:\photos\sunset.png"], "response: {response:?}");
        request.filters.extensions = vec!["pdf".into()];
        request.query = "sunset".into();
        assert!(
            engine.search(&request).results.is_empty(),
            "Photos ext:pdf must not widen the photo scope"
        );
        request.filters.extensions.clear();
        request.filters.under_path = None;
        request.filters.modified_after = Some(2);
        request.query = "violet nebula".into();
        assert!(engine
            .search(&request)
            .results
            .iter()
            .all(|result| result.kind != SourceKind::OcrText));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ocr_scope_returns_images_and_documents_with_ocr_and_content_matches() {
        let dir = scratch_dir("ocr-scope");
        let engine = make_engine(&dir, None);
        *engine.index.write().unwrap() = FilenameIndex::from_entries(vec![
            ps_index::IndexEntry {
                path: r"C:\docs\invoice.png".into(),
                name: "invoice.png".into(),
                ext: "png".into(),
                kind: EntryKind::File,
                size: 10,
                modified: 1,
            },
            ps_index::IndexEntry {
                path: r"C:\docs\invoice.pdf".into(),
                name: "invoice.pdf".into(),
                ext: "pdf".into(),
                kind: EntryKind::File,
                size: 10,
                modified: 1,
            },
            ps_index::IndexEntry {
                path: r"C:\docs\invoice-folder".into(),
                name: "invoice-folder".into(),
                ext: String::new(),
                kind: EntryKind::Folder,
                size: 0,
                modified: 1,
            },
        ]);
        engine.sync_filename_entries();
        engine
            .db
            .ocr_upsert(
                r"C:\docs\invoice.png",
                "png-hash",
                "total amount due $500",
                "en",
                10,
                10,
                10,
                1,
            )
            .unwrap();
        engine.settings.write().unwrap().indexing.ocr_enabled = true;

        let mut request = SearchRequest::new(40, "invoice");
        request.scope = SearchScope::Ocr;
        let results = engine.search(&request).results;
        assert!(!results.is_empty());
        assert!(
            results.iter().all(|result| {
                result.kind == SourceKind::OcrText
                    || result.kind == SourceKind::ContentMatch
                    || result.path.as_deref().is_some_and(|path| {
                        let lower = path.to_ascii_lowercase();
                        lower.ends_with(".png") || lower.ends_with(".pdf")
                    })
            }),
            "OCR scope must only return images and documents: {:?}",
            results.iter().map(|r| (&r.path, r.kind)).collect::<Vec<_>>()
        );
        assert!(!results.iter().any(|r| r.kind == SourceKind::Folder));
        let _ = std::fs::remove_dir_all(&dir);
    }
    #[test]
    fn onboarding_rejects_unavailable_ocr_engine_without_saving() {
        let dir = scratch_dir("onboarding-unsupported-ocr");
        let engine = make_engine(&dir, None);
        let result = engine.complete_onboarding(OnboardingInput {
            user_name: None,
            roots: Vec::new(),
            content_enabled: false,
            ocr_enabled: true,
            // Indra models live in the scratch data dir, so it is never installed here.
            ocr_engine: OcrEngine::Indra,
            hotkey: "Alt+Space".into(),
            agent_opt_in: false,
            skipped: false,
        });
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Select Windows OCR"));
        assert!(!engine.settings().onboarding_complete);
        assert!(!dir.join("settings.json").exists());
        let _ = std::fs::remove_dir_all(dir);
    }
    #[test]
    fn settings_reject_unavailable_ocr_engine() {
        let dir = scratch_dir("settings-unsupported-ocr");
        let engine = make_engine(&dir, None);
        let result = engine.update_settings(SettingsPatch {
            ocr_engine: Some(OcrEngine::Indra),
            ..SettingsPatch::default()
        });
        assert!(result.unwrap_err().to_string().contains("Select Windows OCR"));
        assert_eq!(engine.settings().ocr_engine, OcrEngine::Windows);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ocr_engine_change_reports_reindexing_and_rereads_every_image() {
        let dir = scratch_dir("ocr-engine-change");
        let engine = Arc::new(make_engine(&dir, None));
        // An empty PP-OCRv6 pack counts as installed and fails on every image it reads,
        // so a re-read shows up as an OCR warning.
        let pack = dir.join("models").join("ppocrv6");
        std::fs::create_dir_all(&pack).unwrap();
        for file in ["pp-ocrv6_tiny_det.onnx", "pp-ocrv6_tiny_rec.onnx", "ppocrv6_tiny_dict.txt"] {
            std::fs::write(pack.join(file), b"").unwrap();
        }
        let root = dir.join("shots");
        std::fs::create_dir_all(&root).unwrap();
        let image = root.join("shot.png");
        std::fs::write(&image, b"not a png").unwrap();
        let path = image.to_string_lossy().into_owned();
        {
            let mut settings = engine.settings.write().unwrap();
            settings.indexing.ocr_enabled = true;
            settings.indexing.roots = vec![ps_ipc::RootConfig {
                path: root.to_string_lossy().into_owned(),
                mode: RootMode::ContentAndOcr,
                enabled: true,
            }];
        }
        *engine.index.write().unwrap() = FilenameIndex::from_entries(vec![ps_index::IndexEntry {
            path: path.clone(),
            name: "shot.png".into(),
            ext: "png".into(),
            kind: EntryKind::File,
            size: 9,
            modified: 1,
        }]);
        // Read earlier by Windows OCR.
        let windows_key = format!("windows:{}", quick_hash(&path, 9, 1));
        engine.db.ocr_upsert(&path, &windows_key, "old text", "en", 1, 1, 9, 1).unwrap();

        engine
            .update_settings(SettingsPatch {
                ocr_engine: Some(OcrEngine::PpOcrV6),
                ..SettingsPatch::default()
            })
            .unwrap();
        let status = engine.status();
        assert!(status.reindexing, "an engine change must show Reindexing");
        assert_eq!(status.state, IndexerState::Starting);
        assert!(engine.force_rescan.load(Ordering::SeqCst), "an engine change must queue a pass");

        // The pass reads the image again rather than keeping the Windows OCR result.
        index_ocr(&engine, &[root]);
        let status = engine.status();
        assert!(status
            .warnings
            .iter()
            .any(|warning| warning.starts_with("OCR failed for") && warning.contains("shot.png")));
        assert_eq!(status.ocr_pending, 0);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ocr_backend_follows_the_saved_engine() {
        let dir = scratch_dir("ocr-backend");
        let engine = make_engine(&dir, None);
        let settings = engine.settings();
        let data = Path::new("data");
        assert_eq!(ocr_backend(settings.ocr_engine, data, &settings), ("windows", None));
        assert_eq!(ocr_backend(OcrEngine::PpOcrV6, data, &settings), ("pp_ocr_v6", Some(data.join("models").join("ppocrv6"))));
        assert_eq!(ocr_backend(OcrEngine::Indra, data, &settings), ("indra", Some(data.join("models").join("indra"))));
        assert_eq!(ocr_backend(OcrEngine::Tesseract, data, &settings), ("tesseract", None));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn photos_ocr_filters_page_past_initial_matches() {
        let dir = scratch_dir("photos-ocr-pagination");
        let engine = make_engine(&dir, None);
        let mut entries = Vec::new();
        for i in 0..105 {
            let path = format!(r"C:\photos\other{i}.png");
            entries.push(ps_index::IndexEntry {
                path: path.clone(),
                name: format!("other{i}.png"),
                ext: "png".into(),
                kind: EntryKind::File,
                size: 10,
                modified: 1,
            });
            engine
                .db
                .ocr_upsert(
                    &path,
                    &format!("hash-{i}"),
                    "pagination pagination",
                    "en",
                    10,
                    10,
                    10,
                    1,
                )
                .unwrap();
        }
        let target = r"C:\photos\target.png";
        entries.push(ps_index::IndexEntry {
            path: target.into(),
            name: "target.png".into(),
            ext: "png".into(),
            kind: EntryKind::File,
            size: 10,
            modified: 5,
        });
        engine
            .db
            .ocr_upsert(target, "target-hash", "pagination", "en", 10, 10, 10, 5)
            .unwrap();
        *engine.index.write().unwrap() = FilenameIndex::from_entries(entries);
        engine.sync_filename_entries();
        engine.settings.write().unwrap().indexing.ocr_enabled = true;
        let mut request = SearchRequest::new(41, "pagination");
        request.scope = SearchScope::Photos;
        request.filters.under_path = Some(r"C:\photos".into());
        request.filters.modified_after = Some(5);
        let results = engine.search(&request).results;
        let paths: Vec<_> = results
            .iter()
            .filter(|result| result.kind == SourceKind::OcrText)
            .filter_map(|result| result.path.as_deref())
            .collect();
        assert_eq!(paths, vec![target]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn app_scope_never_includes_recent_files() {
        let dir = scratch_dir("app_recent_scope");
        let engine = make_engine(&dir, None);
        engine
            .sources
            .write()
            .unwrap()
            .recent
            .push(ps_sources::RecentEntry {
                path: r"C:\docs\report.txt".into(),
                name: "report.txt".into(),
                last_used_ms: 1,
            });
        let mut request = SearchRequest::new(1, "report");
        request.scope = SearchScope::Apps;
        assert!(engine
            .search(&request)
            .results
            .iter()
            .all(|r| r.kind == SourceKind::Application));
        request.query = "file:report".into();
        assert!(engine
            .search(&request)
            .results
            .iter()
            .all(|r| r.kind == SourceKind::Application));
        request.query.clear();
        assert!(engine
            .search(&request)
            .results
            .iter()
            .all(|r| r.kind == SourceKind::Application));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn files_scope_is_strict_and_all_keeps_groups_separate() {
        let dir = scratch_dir("scopes");
        let engine = make_engine(&dir, None);
        *engine.index.write().unwrap() = FilenameIndex::from_entries(vec![
            ps_index::IndexEntry {
                path: r"C:\docs\report.txt".into(),
                name: "report.txt".into(),
                ext: "txt".into(),
                kind: EntryKind::File,
                size: 1,
                modified: 1,
            },
            ps_index::IndexEntry {
                path: r"C:\docs\report".into(),
                name: "report".into(),
                ext: String::new(),
                kind: EntryKind::Folder,
                size: 0,
                modified: 2,
            },
        ]);
        engine.sync_filename_entries();

        let mut apps_with_file_prefix = SearchRequest::new(29, "file:report");
        apps_with_file_prefix.scope = SearchScope::Apps;
        assert!(engine
            .search(&apps_with_file_prefix)
            .results
            .iter()
            .all(|r| r.kind == SourceKind::Application));
        let mut files = SearchRequest::new(30, "report");
        files.scope = SearchScope::Files;
        let resp = engine.search(&files);
        assert!(!resp.results.is_empty(), "FILES scope must return files");
        assert!(
            resp.results.iter().all(|r| r.kind == SourceKind::File),
            "FILES scope must not return folders: {:?}",
            resp.results
                .iter()
                .map(|r| (&r.title, r.kind))
                .collect::<Vec<_>>()
        );

        let mut folders = SearchRequest::new(31, "report");
        folders.scope = SearchScope::Folders;
        let resp = engine.search(&folders);
        assert!(!resp.results.is_empty(), "FOLDERS scope must return folders");
        assert!(
            resp.results.iter().all(|r| r.kind == SourceKind::Folder),
            "FOLDERS scope must not return files: {:?}",
            resp.results
                .iter()
                .map(|r| (&r.title, r.kind))
                .collect::<Vec<_>>()
        );

        // ALL keeps the two groups and both are present.
        let resp = engine.search(&SearchRequest::new(32, "report"));
        assert!(resp.results.iter().any(|r| r.kind == SourceKind::File));
        assert!(resp.results.iter().any(|r| r.kind == SourceKind::Folder));

        // kind: filter narrows within ALL.
        // ALL scope also surfaces tier-0 action rows, so look at filename rows.
        let is_filename = |kind: SourceKind| matches!(kind, SourceKind::File | SourceKind::Folder);
        let resp = engine.search(&SearchRequest::new(33, "report kind:folder"));
        let rows: Vec<_> = resp.results.iter().filter(|r| is_filename(r.kind)).collect();
        assert!(!rows.is_empty(), "kind:folder must return the folder");
        assert!(
            rows.iter().all(|r| r.kind == SourceKind::Folder),
            "kind:folder returned: {:?}",
            rows.iter().map(|r| (&r.title, r.kind)).collect::<Vec<_>>()
        );
        let resp = engine.search(&SearchRequest::new(34, "report kind:file"));
        let rows: Vec<_> = resp.results.iter().filter(|r| is_filename(r.kind)).collect();
        assert!(!rows.is_empty(), "kind:file must return the file");
        assert!(
            rows.iter().all(|r| r.kind == SourceKind::File),
            "kind:file returned: {:?}",
            rows.iter().map(|r| (&r.title, r.kind)).collect::<Vec<_>>()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clipboard_results_are_live_categorized_and_counted() {
        let dir = scratch_dir("clipboard-live");
        let engine = make_engine(&dir, None);
        // Clipboard history is opt-in in v2; enable it for this test.
        let mut privacy = engine.settings().privacy;
        privacy.clipboard_history = true;
        engine
            .update_settings(SettingsPatch {
                privacy: Some(privacy),
                ..Default::default()
            })
            .unwrap();
        let db = engine.clipboard_db_path();
        // Seed through the public API; record() classifies at write time.
        let seeds = [
            ("https://example.com/kall-link", "link"),
            ("kall-mail@example.com", "email"),
            (r"C:\docs\kall.txt", "path"),
            ("SELECT * FROM kall WHERE id = 1;", "code"),
            ("kall plain text", "text"),
        ];
        for (text, expected) in seeds {
            assert!(ps_sources::clipboard::record(&db, text, "test-app").unwrap());
            assert_eq!(
                ps_sources::clipboard::classify(text).as_str(),
                expected,
                "classifier mismatch for {text}"
            );
        }
        assert_eq!(ps_sources::clipboard::count(&db), 5);
        // Live status count comes from the database, not the startup snapshot.
        let status = engine.status();
        let clip = status.sources.iter().find(|s| s.id == "clipboard").unwrap();
        assert_eq!(clip.item_count, 5);
        assert!(clip.enabled);

        // Pin the link item: search must put pinned items first.
        let pinned_row = ps_sources::clipboard::list_recent(&db, 10)
            .into_iter()
            .find(|row| row.text.contains("kall-link"))
            .expect("seeded link row");
        assert_eq!(pinned_row.category.as_str(), "link");
        ps_sources::clipboard::set_pinned(&db, pinned_row.id, true).unwrap();
        let mut req = SearchRequest::new(40, "kall");
        req.scope = SearchScope::Clipboard;
        let resp = engine.search(&req);
        assert!(!resp.results.is_empty(), "live clipboard query must return rows");
        assert!(
            resp.results.iter().all(|r| r.kind == SourceKind::Clipboard),
            "clipboard scope must only return clipboard rows"
        );
        assert_eq!(
            resp.results[0].meta.label.as_deref(),
            Some("LINK"),
            "pinned item must rank first: {:?}",
            resp.results
                .iter()
                .map(|r| r.meta.label.clone())
                .collect::<Vec<_>>()
        );
        assert!(resp.results[0].meta.pinned, "the pinned row carries the flag");
        assert!(resp.results[1..].iter().all(|r| !r.meta.pinned));
        // Category labels map to the UI badge vocabulary.
        let labels: Vec<String> = resp.results.iter().filter_map(|r| r.meta.label.clone()).collect();
        assert!(labels.contains(&"LINK".to_string()));
        assert!(labels.contains(&"TEXT".to_string()));
        assert!(labels
            .iter()
            .all(|l| ["LINK", "EMAIL", "PATH", "CODE", "IMAGE", "TEXT"].contains(&l.as_str())));

        // Disabling privacy stops clipboard results immediately.
        let mut settings = engine.settings();
        settings.privacy.clipboard_history = false;
        engine
            .update_settings(SettingsPatch {
                privacy: Some(settings.privacy),
                ..Default::default()
            })
            .unwrap();
        let resp = engine.search(&req);
        assert!(
            !resp.results.iter().any(|r| r.kind == SourceKind::Clipboard),
            "disabled clipboard privacy must hide clipboard results"
        );
        let clip = engine
            .status()
            .sources
            .into_iter()
            .find(|s| s.id == "clipboard")
            .unwrap();
        assert!(!clip.enabled);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clipboard_listener_lifecycle_follows_privacy_setting() {
        let dir = scratch_dir("clipboard-listener");
        let engine = make_engine(&dir, None);
        assert!(engine.clipboard_listener.lock().unwrap().is_none());
        // Clipboard history is opt-in: nothing runs while the setting is off.
        assert!(!engine.settings().privacy.clipboard_history);
        engine.sync_clipboard_listener();
        assert!(engine.clipboard_listener.lock().unwrap().is_none());
        // Enabling starts the hook thread exactly once.
        let mut settings = engine.settings();
        settings.privacy.clipboard_history = true;
        engine
            .update_settings(SettingsPatch {
                privacy: Some(settings.privacy),
                ..Default::default()
            })
            .unwrap();
        let started = engine.clipboard_listener.lock().unwrap().is_some();
        engine.sync_clipboard_listener();
        assert_eq!(engine.clipboard_listener.lock().unwrap().is_some(), started);
        // Disabling the privacy setting stops the hook thread.
        let mut settings = engine.settings();
        settings.privacy.clipboard_history = false;
        engine
            .update_settings(SettingsPatch {
                privacy: Some(settings.privacy),
                ..Default::default()
            })
            .unwrap();
        assert!(engine.clipboard_listener.lock().unwrap().is_none());
        // Re-enabling starts it again.
        let mut settings = engine.settings();
        settings.privacy.clipboard_history = true;
        engine
            .update_settings(SettingsPatch {
                privacy: Some(settings.privacy),
                ..Default::default()
            })
            .unwrap();
        let restarted = engine.clipboard_listener.lock().unwrap().is_some();
        // Pausing capture stops the hook even though privacy stays on.
        let mut settings = engine.settings();
        settings.clipboard.pause = true;
        engine
            .update_settings(SettingsPatch {
                clipboard: Some(settings.clipboard),
                ..Default::default()
            })
            .unwrap();
        assert!(
            engine.clipboard_listener.lock().unwrap().is_none(),
            "pause must stop the clipboard hook"
        );
        // Unpausing restarts it; changing a format toggle restarts with the new
        // capture options (captureImages off).
        let mut settings = engine.settings();
        settings.clipboard.pause = false;
        engine
            .update_settings(SettingsPatch {
                clipboard: Some(settings.clipboard),
                ..Default::default()
            })
            .unwrap();
        assert!(engine.clipboard_listener.lock().unwrap().is_some());
        let mut settings = engine.settings();
        settings.clipboard.capture_images = false;
        engine
            .update_settings(SettingsPatch {
                clipboard: Some(settings.clipboard),
                ..Default::default()
            })
            .unwrap();
        let capture = engine
            .clipboard_listener
            .lock()
            .unwrap()
            .as_ref()
            .map(|(_, capture)| *capture);
        assert_eq!(
            capture,
            Some(ClipboardCapture {
                capture_text: true,
                capture_images: false
            }),
            "format toggles must reach the running hook"
        );
        engine.stop_clipboard_listener();
        assert!(engine.clipboard_listener.lock().unwrap().is_none());
        if started != restarted {
            eprintln!(
                "clipboard listener lifecycle: started={started} restarted={restarted} warnings={:?}",
                engine.status().warnings
            );
        }
        // The sandbox may forbid a clipboard hook; require lifecycle consistency.
        assert_eq!(started, restarted);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_query_clipboard_scope_lists_recent_categorized_history() {
        let dir = scratch_dir("clipboard-empty");
        let engine = make_engine(&dir, None);
        // Clipboard history is opt-in in v2.
        let mut privacy = engine.settings().privacy;
        privacy.clipboard_history = true;
        engine
            .update_settings(SettingsPatch {
                privacy: Some(privacy),
                ..Default::default()
            })
            .unwrap();
        let db = engine.clipboard_db_path();
        assert!(ps_sources::clipboard::record(&db, "https://example.com/one", "app").unwrap());
        assert!(ps_sources::clipboard::record(&db, "kall-mail@example.com", "app").unwrap());
        assert!(ps_sources::clipboard::record(&db, "kall plain text", "app").unwrap());
        let pinned = ps_sources::clipboard::list_recent(&db, 10)
            .into_iter()
            .find(|row| row.text.contains("example.com/one"))
            .expect("seeded link row");
        ps_sources::clipboard::set_pinned(&db, pinned.id, true).unwrap();

        let mut req = SearchRequest::new(50, "");
        req.scope = SearchScope::Clipboard;
        let resp = engine.search(&req);
        assert_eq!(
            resp.results.len(),
            3,
            "empty clipboard query must list recent rows: {:?}",
            resp.results
                .iter()
                .map(|r| (&r.title, r.kind))
                .collect::<Vec<_>>()
        );
        assert!(
            resp.results.iter().all(|r| r.kind == SourceKind::Clipboard),
            "clipboard scope must only return clipboard rows"
        );
        assert_eq!(
            resp.results[0].meta.label.as_deref(),
            Some("LINK"),
            "pinned row must rank first: {:?}",
            resp.results
                .iter()
                .map(|r| r.meta.label.clone())
                .collect::<Vec<_>>()
        );
        let labels: Vec<String> = resp.results.iter().filter_map(|r| r.meta.label.clone()).collect();
        assert!(labels.contains(&"EMAIL".to_string()), "labels: {labels:?}");
        assert!(labels.contains(&"TEXT".to_string()), "labels: {labels:?}");

        // Empty query in ALL scope keeps the existing recent-files behaviour.
        let resp_all = engine.search(&SearchRequest::new(51, ""));
        assert!(
            resp_all.results.iter().all(|r| r.kind != SourceKind::Clipboard),
            "ALL scope must not leak clipboard rows into the recent list"
        );

        // Privacy off => the clipboard scope is empty, never stale rows.
        let mut privacy = engine.settings().privacy;
        privacy.clipboard_history = false;
        engine
            .update_settings(SettingsPatch {
                privacy: Some(privacy),
                ..Default::default()
            })
            .unwrap();
        let resp = engine.search(&req);
        assert!(
            resp.results.is_empty(),
            "privacy off must hide clipboard history: {:?}",
            resp.results
                .iter()
                .map(|r| (&r.title, r.kind))
                .collect::<Vec<_>>()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migrated_rows_survive_restart_even_without_index_file() {
        let dir = scratch_dir("migration-restart");
        let v2 = dir.join("v2");
        std::fs::create_dir_all(&v2).unwrap();
        let legacy = dir.join("legacy.db");
        make_legacy_fixture(&legacy);
        {
            let engine = make_engine(&v2, Some(legacy.clone()));
            engine.migrate_legacy(&legacy);
            assert_eq!(engine.status().filename_entries, 3);
        }
        // Migration persists the index for the next launch...
        assert!(
            v2.join(INDEX_FILE).is_file(),
            "migration must persist the filename index"
        );
        // ...and even if that file is lost, open() recovers the migrated rows
        // from the v2 database instead of starting at "0 items".
        std::fs::remove_file(v2.join(INDEX_FILE)).unwrap();
        let reopened = make_engine(&v2, None);
        assert_eq!(
            reopened.status().filename_entries,
            3,
            "restart must report migrated rows"
        );
        assert_eq!(reopened.index.read().unwrap().len(), 3);
        let resp = reopened.search(&SearchRequest::new(3, "docs"));
        assert_eq!(indexed_files(resp.status), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // -- Gate-2 preview backend tests (task-37) -----------------------------

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("docs")
            .join("qa")
            .join("preview-audit")
            .join("fixtures")
            .join(name)
    }

    fn preview_result(id: &str, kind: SourceKind, path: Option<&str>) -> SearchResult {
        let name = path
            .and_then(|path| Path::new(path).file_name())
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| id.to_string());
        SearchResult {
            id: id.to_string(),
            kind,
            title: name,
            subtitle: String::new(),
            path: path.map(str::to_string),
            icon: IconRef::Monogram,
            score: 0.0,
            match_ranges: Vec::new(),
            meta: ResultMeta::default(),
            action: path
                .map(|path| ActionSpec::OpenPath {
                    path: path.to_string(),
                    line: None,
                    column: None,
                })
                .unwrap_or(ActionSpec::None),
            actions: Vec::new(),
        }
    }

    fn preview_for(engine: &Engine, result: &SearchResult, max_chars: usize) -> PreviewPayload {
        engine.remember_results(std::slice::from_ref(result));
        engine.preview(&PreviewRequest {
            result_id: result.id.clone(),
            max_chars: Some(max_chars),
        })
    }

    #[test]
    fn preview_text_and_structured_formats_are_real() {
        let dir = scratch_dir("preview-formats");
        let engine = make_engine(&dir, None);
        let cases = [
            ("notes.txt", "ZEBRA-TXT-2026", "txt"),
            ("readme.md", "UNICORN-MD-2026", "md"),
            ("sample.rs", "PANDA-CODE-2026", "rs"),
            ("report.pdf", "WALRUS-PDF-2026", "pdf"),
            ("report.docx", "KOALA-DOCX-2026", "docx"),
            ("sheet.xlsx", "TIGER-XLSX-2026", "xlsx"),
        ];
        for (name, token, format) in cases {
            let path = fixture(name);
            assert!(path.is_file(), "missing fixture {name}");
            let mut result = preview_result(
                &format!("file:{}", path.to_string_lossy().to_lowercase()),
                SourceKind::File,
                Some(&path.to_string_lossy()),
            );
            result.match_ranges = vec![TextRange { start: 0, end: 3 }];
            let payload = preview_for(&engine, &result, 4_000);
            assert_eq!(payload.format, format, "{name} format");
            assert_eq!(payload.status, PreviewStatus::Ok, "{name} status");
            assert!(!payload.truncated, "{name} must not be truncated at 4k chars");
            assert!(
                payload.unsupported_reason.is_none(),
                "{name} unexpectedly unsupported: {:?}",
                payload.unsupported_reason
            );
            let text = payload.text.as_deref().unwrap_or("");
            assert!(
                text.contains(token),
                "{name} preview text missing token: {text:?}"
            );
            assert!(
                payload.highlight_ranges.is_empty(),
                "filename offsets must not highlight extracted body text"
            );
            assert!(payload.fields.iter().any(|field| field.label == "Size"));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn preview_scanned_pdf_binary_and_unsupported_are_honest() {
        let dir = scratch_dir("preview-honest");
        let engine = make_engine(&dir, None);
        for name in ["scanned.pdf", "blob.bin", "unknown.xyz"] {
            let path = fixture(name);
            assert!(path.is_file(), "missing fixture {name}");
            let result = preview_result(
                &format!("file:{}", path.to_string_lossy().to_lowercase()),
                SourceKind::File,
                Some(&path.to_string_lossy()),
            );
            let payload = preview_for(&engine, &result, 4_000);
            assert!(
                payload.unsupported_reason.is_some(),
                "{name} must be an honest fallback, got text {:?}",
                payload.text
            );
            assert!(payload.text.is_none() || payload.text.as_deref() == Some(""));
            assert!(
                payload.fields.iter().any(|field| field.label == "Size"),
                "{name} keeps metadata"
            );
        }
        // Scanned PDF specifically explains why there is no body.
        let path = fixture("scanned.pdf");
        let result = preview_result("file:scanned", SourceKind::File, Some(&path.to_string_lossy()));
        let first = preview_for(&engine, &result, 4_000);
        assert!(first
            .unsupported_reason
            .as_deref()
            .unwrap_or("")
            .contains("text layer"));
        engine
            .db
            .ocr_upsert(
                &path.to_string_lossy(),
                "test-hash",
                "SCANNED-PDF-OCR-2026 recognized text",
                "en",
                0,
                0,
                0,
                0,
            )
            .unwrap();
        let payload = preview_for(&engine, &result, 4_000);
        assert!(payload
            .ocr_text
            .as_deref()
            .unwrap_or("")
            .contains("SCANNED-PDF-OCR-2026"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn preview_image_returns_thumbnail_and_indexed_ocr_text() {
        let dir = scratch_dir("preview-image");
        let engine = make_engine(&dir, None);
        let path = fixture("diagram.png");
        assert!(path.is_file());
        let path_string = path.to_string_lossy().to_string();
        engine
            .db
            .ocr_upsert(
                &path_string,
                "test-hash",
                "IMAGE-OCR-PROBE-2026 screenshot preview audit",
                "en",
                900,
                260,
                0,
                0,
            )
            .unwrap();
        let result = preview_result(
            &format!("file:{}", path_string.to_lowercase()),
            SourceKind::File,
            Some(&path_string),
        );
        let payload = preview_for(&engine, &result, 4_000);
        let data_url = payload.image_data_url.as_deref().unwrap_or("");
        assert!(data_url.starts_with("data:image/png;base64,"), "image data url");
        assert!(data_url.len() > 100, "thumbnail must have bytes");
        assert!(payload
            .ocr_text
            .as_deref()
            .unwrap_or("")
            .contains("IMAGE-OCR-PROBE-2026"));
        assert!(payload
            .fields
            .iter()
            .any(|field| field.label == "Dimensions" && field.value.contains('x')));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn preview_unknown_result_is_honest_and_path_ids_resolve() {
        let dir = scratch_dir("preview-unknown");
        let engine = make_engine(&dir, None);
        let payload = engine.preview(&PreviewRequest {
            result_id: "unknown:42".to_string(),
            max_chars: None,
        });
        assert!(payload
            .unsupported_reason
            .as_deref()
            .unwrap_or("")
            .contains("no longer available"));
        // Path-prefixed ids resolve even without a registry entry.
        let path = fixture("notes.txt");
        let id = format!("file:{}", path.to_string_lossy().to_lowercase());
        let payload = engine.preview(&PreviewRequest {
            result_id: id,
            max_chars: None,
        });
        assert!(payload.text.as_deref().unwrap_or("").contains("ZEBRA-TXT-2026"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn deleting_empty_ocr_result_removes_stale_searchable_text() {
        let dir = scratch_dir("ocr-empty-result");
        let engine = make_engine(&dir, None);
        let path = r"C:\images\changed.png";
        engine
            .db
            .ocr_upsert(path, "old-hash", "STALE-OCR-PROBE-2026", "en", 10, 10, 10, 1)
            .unwrap();
        assert_eq!(engine.db.ocr_search("STALE-OCR-PROBE-2026", 5).unwrap().len(), 1);

        engine.db.ocr_delete(path).unwrap();

        assert!(engine.db.ocr_text_for_path(path).is_none());
        assert!(engine
            .db
            .ocr_search("STALE-OCR-PROBE-2026", 5)
            .unwrap()
            .is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ocr_result_keeps_its_text_as_snippet() {
        let hit = crate::db::OcrHit {
            path: r"C:\shots\one.png".to_string(),
            text: "IMAGE-OCR-PROBE-2026 screenshot preview audit".to_string(),
            line: Some(1),
        };
        let result = ocr_result(hit, &Settings::default());
        assert_eq!(result.kind, SourceKind::OcrText);
        assert!(result
            .meta
            .snippet
            .as_deref()
            .unwrap_or("")
            .contains("IMAGE-OCR-PROBE-2026"));
        assert_eq!(result.meta.line, Some(1));
    }

    #[test]
    fn file_results_use_scaled_scores_and_tiers() {
        let index = FilenameIndex::from_entries(vec![ps_index::IndexEntry {
            path: r"C:\docs\report.pdf".into(),
            name: "report.pdf".into(),
            ext: "pdf".into(),
            kind: EntryKind::File,
            size: 1,
            modified: 1,
        }]);
        let settings = Settings::default();
        let exact = ps_index::FileHit {
            entry_index: 0,
            score: 0.94,
            ranges: Vec::new(),
        };
        let (result, tier) = file_result(&index, &exact, &settings).unwrap();
        assert_eq!(tier, 0, "prefix/exact filename matches are tier 0");
        assert!((result.score - 940.0).abs() < 0.01, "score scaled to 0..1000");
        let fuzzy = ps_index::FileHit {
            entry_index: 0,
            score: 0.55,
            ranges: Vec::new(),
        };
        let (result, tier) = file_result(&index, &fuzzy, &settings).unwrap();
        assert_eq!(tier, 1);
        assert!((result.score - 550.0).abs() < 0.01);
    }

    #[test]
    fn search_skips_content_tier_when_top_limit_is_already_full() {
        let dir = scratch_dir("perf-tier-budget");
        // Content document for the second half of the test.
        {
            let content = ps_content::ContentIndex::open(&dir.join("index.db")).unwrap();
            let fixture = fixture("report.pdf");
            content
                .index_file(&fixture, ps_content::DEFAULT_MAX_BYTES)
                .unwrap();
        }
        let engine = make_engine(&dir, None);
        let mut indexing = engine.settings().indexing;
        indexing.content_enabled = true;
        engine
            .update_settings(SettingsPatch {
                indexing: Some(indexing),
                ..Default::default()
            })
            .unwrap();
        // 40 filename entries that fill a 30-result page at tier 0.
        let entries: Vec<ps_index::IndexEntry> = (0..40)
            .map(|i| ps_index::IndexEntry {
                path: format!(r"C:\docs\readme_{i}.txt"),
                name: format!("readme_{i}.txt"),
                ext: "txt".into(),
                kind: EntryKind::File,
                size: 1,
                modified: i as i64,
            })
            .collect();
        *engine.index.write().unwrap() = FilenameIndex::from_entries(entries);
        engine.sync_filename_entries();

        let response = engine.search(&SearchRequest::new(1, "readme"));
        assert_eq!(response.results.len(), 30);
        assert!(
            response
                .results
                .iter()
                .all(|result| result.kind == SourceKind::File),
            "a full tier-0 page must not be diluted by lower tiers"
        );
        assert!(
            !response
                .timings
                .iter()
                .any(|timing| timing.label == "indexed content"),
            "content tier must be skipped when the page is already full"
        );

        // A query with no filename matches still reaches the content tier.
        let response = engine.search(&SearchRequest::new(2, "WALRUS-PDF-2026"));
        assert!(
            response
                .results
                .iter()
                .any(|result| result.kind == SourceKind::ContentMatch),
            "content results must still appear when filename results are sparse"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn app_icons_are_cached_and_reused() {
        let dir = scratch_dir("icon-cache");
        let icon_path = dir.join("icon.png");
        let fixture = fixture("diagram.png");
        std::fs::copy(&fixture, &icon_path).unwrap();
        let path_string = icon_path.to_string_lossy().to_string();
        let before = app_icon_cache_len();
        let first = app_icon(Some(&path_string));
        assert!(matches!(first, IconRef::AppIconData { .. }));
        assert_eq!(app_icon_cache_len(), before + 1, "first read caches the icon");
        let second = app_icon(Some(&path_string));
        assert_eq!(first, second, "cached icon data URL is identical");
        assert_eq!(app_icon_cache_len(), before + 1, "second read is a cache hit");
        // Missing/unsupported icons fall back to a monogram and are cached too.
        let missing = app_icon(Some("C:\\does\\not\\exist.png"));
        assert_eq!(missing, IconRef::Monogram);
        assert_eq!(app_icon(None), IconRef::Monogram);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clipboard_thumbnails_are_cached() {
        let dir = scratch_dir("thumb-cache");
        let thumb_path = dir.join("thumb.png");
        std::fs::copy(fixture("diagram.png"), &thumb_path).unwrap();
        let path_string = thumb_path.to_string_lossy().to_string();
        let before = preview::thumbnail_cache_len();
        let first = preview::thumbnail_data_url(&path_string);
        assert!(first
            .as_deref()
            .unwrap_or("")
            .starts_with("data:image/png;base64,"));
        assert_eq!(preview::thumbnail_cache_len(), before + 1);
        let second = preview::thumbnail_data_url(&path_string);
        assert_eq!(first, second);
        assert_eq!(preview::thumbnail_cache_len(), before + 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn remember_results_trims_icon_payload_and_actions_but_keeps_preview_data() {
        let dir = scratch_dir("registry-trim");
        let engine = make_engine(&dir, None);
        let mut result = preview_result("file:trim-probe", SourceKind::File, Some(r"C:\docs\trim.txt"));
        result.icon = IconRef::AppIconData {
            data: format!("data:image/png;base64,{}", "A".repeat(64 * 1024)),
        };
        result.actions = vec![ps_ipc::ActionItem {
            id: "reveal".into(),
            label: "Reveal".into(),
            action: ps_ipc::ActionSpec::RevealPath {
                path: r"C:\docs\trim.txt".into(),
            },
            destructive: false,
            shortcut: None,
        }];
        result.meta.snippet = Some("trimmed snippet".into());
        engine.remember_results(std::slice::from_ref(&result));
        let registry = engine.recent_results.lock().unwrap();
        let stored = registry.get(&result.id).expect("registered");
        assert_eq!(stored.icon, IconRef::Monogram, "icon payload is not cloned");
        assert!(stored.actions.is_empty(), "action list is not cloned");
        assert_eq!(stored.meta.snippet.as_deref(), Some("trimmed snippet"));
        drop(registry);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn browser_matcher_is_case_insensitive_without_allocating() {
        let entries = vec![
            ps_sources::BrowserEntry {
                url: "https://doc.rust-lang.org/book/".into(),
                title: "Rust Programming Language".into(),
                source: "chrome".into(),
                visit_count: 3,
                last_visit_ms: 10,
                bookmark: false,
            },
            ps_sources::BrowserEntry {
                url: "https://example.com/".into(),
                title: "Example Domain".into(),
                source: "chrome".into(),
                visit_count: 1,
                last_visit_ms: 5,
                bookmark: false,
            },
        ];
        let terms = vec!["rust".to_string(), "programming".to_string()];
        let hits = match_browser(&entries, &terms);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Rust Programming Language");
        // Matcher terms are lower-cased by the query parser; haystacks fold ASCII case.
        let terms = vec!["example".to_string()];
        let hits = match_browser(&entries, &terms);
        assert_eq!(hits.len(), 1);
        assert!(contains_ci("Rust Programming Language", "programming"));
        assert!(contains_ci("Rust Programming Language", "rust"));
        assert!(contains_ci(
            "Rust Programming Language",
            "PROGRAMMING".to_lowercase().as_str()
        ));
        assert!(!contains_ci("Example Domain", "rust"));
    }

    #[test]
    fn git_scope_lists_the_newest_repository_then_commits_for_the_card() {
        let dir = scratch_dir("git-card");
        let engine = make_engine(&dir, None);
        engine.settings.write().unwrap().privacy.git_indexing = true;
        let commit = |hash: &str, repo: &str, date_ms: i64| ps_sources::GitEntry::Commit {
            repo: repo.into(),
            hash: hash.into(),
            message: format!("message {hash}"),
            author: "Dev".into(),
            date_ms,
        };
        {
            let mut sources = engine.sources.write().unwrap();
            sources.git = vec![
                ps_sources::GitEntry::Repo { path: "C:/old".into(), name: "old".into(), branch: None, branches: Vec::new() },
                ps_sources::GitEntry::Repo {
                    path: "C:/new".into(),
                    name: "new".into(),
                    branch: Some("main".into()),
                    branches: vec!["main".into(), "dev".into()],
                },
                commit("aaaaaaaa1", "C:/old", 10),
                commit("bbbbbbbb2", "C:/new", 30),
                commit("cccccccc3", "C:/new", 20),
            ];
        }
        let search = |query: &str| {
            engine.search(&SearchRequest {
                protocol: PROTOCOL_VERSION,
                request_id: 1,
                query: query.into(),
                scope: SearchScope::Git,
                filters: SearchFilters::default(),
                limit: 10,
            })
        };
        let results = search("").results;
        let titles: Vec<&str> = results.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(titles, ["new", "old", "message bbbbbbbb2", "message cccccccc3", "message aaaaaaaa1"]);
        assert_eq!(results[0].meta.branches, ["main", "dev"]);
        engine.settings.write().unwrap().privacy.git_indexing = false;
        assert!(search("").results.is_empty(), "git indexing off lists nothing");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn browser_rows_carry_the_favicon_of_their_host() {
        let dir = scratch_dir("browser-favicon");
        let engine = make_engine(&dir, None);
        engine.settings.write().unwrap().privacy.browser_history = true;
        {
            let mut sources = engine.sources.write().unwrap();
            engine.db.store_browser(&[ps_sources::BrowserEntry {
                url: "https://www.example.com/a".into(),
                title: "Example".into(),
                source: "Chrome".into(),
                visit_count: 1,
                last_visit_ms: 10,
                bookmark: false,
            }]).unwrap();
            sources
                .favicons
                .insert("example.com".into(), "data:image/png;base64,AAAA".into());
        }
        let response = engine.search(&SearchRequest {
            protocol: PROTOCOL_VERSION,
            request_id: 1,
            query: String::new(),
            scope: SearchScope::Browser,
            filters: SearchFilters::default(),
            limit: 10,
        });
        assert_eq!(
            response.results[0].meta.favicon_data_url.as_deref(),
            Some("data:image/png;base64,AAAA")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn browser_scope_returns_empty_and_full_query_results_across_browsers() {
        let dir = scratch_dir("browser-scope");
        let engine = make_engine(&dir, None);
        {
            let mut settings = engine.settings.write().unwrap();
            settings.privacy.browser_history = true;
        }
        {
            let mut list = vec![ps_sources::BrowserEntry {
                url: "https://example.com/bm".into(),
                title: "Saved Docs".into(),
                source: "Firefox".into(),
                visit_count: 0,
                last_visit_ms: 0,
                bookmark: true,
            }];
            for (idx, browser) in ["Chrome", "Edge", "Brave", "Opera GX", "Zen", "Vivaldi"].iter().enumerate() {
                list.push(ps_sources::BrowserEntry {
                    url: format!("https://example.com/item/{idx}"),
                    title: format!("Example Visit {idx}"),
                    source: (*browser).into(),
                    visit_count: 2,
                    last_visit_ms: 10_000 + idx as i64,
                    bookmark: false,
                });
            }
            engine.db.store_browser(&list).unwrap();
        }

        // Empty query in Browser scope must return both recent history and bookmarks.
        let empty_res = engine.search(&SearchRequest {
            protocol: PROTOCOL_VERSION,
            request_id: 1,
            query: String::new(),
            scope: SearchScope::Browser,
            filters: SearchFilters::default(),
            limit: 10,
        });
        assert!(empty_res.results.iter().any(|r| r.kind == SourceKind::Bookmark));
        assert!(empty_res.results.iter().any(|r| r.kind == SourceKind::BrowserHistory));
        assert_eq!(empty_res.results.len(), 7);

        // Non-empty query in Browser scope must return more than 4 matching items up to limit.
        let query_res = engine.search(&SearchRequest {
            protocol: PROTOCOL_VERSION,
            request_id: 2,
            query: "example".into(),
            scope: SearchScope::Browser,
            filters: SearchFilters::default(),
            limit: 10,
        });
        let browser_hits = query_res
            .results
            .iter()
            .filter(|r| matches!(r.kind, SourceKind::Bookmark | SourceKind::BrowserHistory))
            .count();
        assert_eq!(browser_hits, 7, "all 7 browser items must match");

        // bookmarks: prefix must filter strictly to bookmarks.
        let bm_res = engine.search(&SearchRequest {
            protocol: PROTOCOL_VERSION,
            request_id: 3,
            query: "example bookmarks:".into(),
            scope: SearchScope::All,
            filters: SearchFilters::default(),
            limit: 10,
        });
        let bm_browser_hits: Vec<_> = bm_res
            .results
            .iter()
            .filter(|r| matches!(r.kind, SourceKind::Bookmark | SourceKind::BrowserHistory))
            .collect();
        assert_eq!(bm_browser_hits.len(), 1);
        assert_eq!(bm_browser_hits[0].kind, SourceKind::Bookmark);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn root_modes_and_content_scope_are_strict() {
        let mut settings = Settings::default();
        settings.indexing.roots = vec![RootConfig {
            path: "C:/Users/Example/Documents".to_string(),
            mode: RootMode::Content,
            enabled: true,
        }];
        let modes = root_modes(&settings);
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0].0, "c:\\users\\example\\documents\\");
        assert_eq!(
            mode_for(&modes, r"C:\Users\Example\Documents\a.txt"),
            1,
            "forward-slash roots must match native scanner paths"
        );
        assert!(!content_scope_allows(SearchScope::Files));
        assert!(content_scope_allows(SearchScope::Content));
        assert!(content_scope_allows(SearchScope::All));
        assert!(content_scope_allows(SearchScope::Ocr));
    }

    #[test]
    fn preview_clipboard_text_and_folder_are_honest() {
        let dir = scratch_dir("preview-clip");
        let engine = make_engine(&dir, None);
        let mut clipboard = preview_result("clip:1", SourceKind::Clipboard, None);
        clipboard.action = ActionSpec::CopyText {
            text: "copied preview text".to_string(),
        };
        clipboard.title = "copied preview text".to_string();
        let payload = preview_for(&engine, &clipboard, 2_000);
        assert_eq!(payload.format, "clipboard");
        assert!(payload
            .text
            .as_deref()
            .unwrap_or("")
            .contains("copied preview text"));
        let folder = preview_result("file:folder", SourceKind::Folder, Some(r"C:\Users\Example"));
        let payload = preview_for(&engine, &folder, 2_000);
        assert_eq!(payload.format, "folder");
        assert!(payload.unsupported_reason.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn preview_marks_truncation_when_text_caps_apply() {
        let dir = scratch_dir("preview-truncated");
        let engine = make_engine(&dir, None);
        let path = fixture("large.txt");
        let result = preview_result(
            "file:large-truncation",
            SourceKind::File,
            Some(&path.to_string_lossy()),
        );
        let payload = preview_for(&engine, &result, 500);
        assert_eq!(payload.status, PreviewStatus::Ok);
        assert!(payload.truncated, "a capped text body must set truncated");
        let text = payload.text.as_deref().unwrap_or("");
        assert!(text.chars().count() <= 501, "text respects maxChars");
        assert!(payload
            .fields
            .iter()
            .any(|field| field.label == "Note" && field.value.contains("truncated")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn preview_unavailable_and_unsupported_status_are_explicit() {
        let dir = scratch_dir("preview-status");
        let engine = make_engine(&dir, None);
        let missing = engine.preview(&PreviewRequest {
            result_id: "unknown:7".to_string(),
            max_chars: None,
        });
        assert_eq!(missing.status, PreviewStatus::Unsupported);
        assert!(missing.unsupported_reason.is_some());
        let path = fixture("scanned.pdf");
        let result = preview_result(
            "file:scanned-status",
            SourceKind::File,
            Some(&path.to_string_lossy()),
        );
        let payload = preview_for(&engine, &result, 4_000);
        assert_eq!(payload.status, PreviewStatus::Unsupported);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clipboard_image_result_carries_thumbnail_dimensions_and_preview() {
        let dir = scratch_dir("clipboard-image");
        let engine = make_engine(&dir, None);
        let image_path = dir.join("full.png");
        let thumbnail_path = dir.join("thumb.png");
        image::RgbaImage::from_pixel(8, 4, image::Rgba([200, 10, 10, 255]))
            .save(&image_path)
            .unwrap();
        image::RgbaImage::from_pixel(4, 2, image::Rgba([10, 10, 200, 255]))
            .save(&thumbnail_path)
            .unwrap();
        let entry = ps_sources::ClipboardEntry {
            id: 7,
            text: "[image:abcdef]".to_string(),
            timestamp_ms: 1_700_000_000_000,
            source_app: "test-app".to_string(),
            pinned: false,
            category: ps_sources::ClipCategory::Image,
            image_path: Some(image_path.to_string_lossy().to_string()),
            width: Some(8),
            height: Some(4),
            thumbnail_path: Some(thumbnail_path.to_string_lossy().to_string()),
            mime: Some("image/png".to_string()),
            format: Some("png".to_string()),
            ocr_text: None,
        };
        let result = clipboard_result(&entry);
        assert_eq!(result.kind, SourceKind::Clipboard);
        assert_eq!(result.meta.label.as_deref(), Some("IMAGE"));
        assert_eq!(result.path.as_deref(), entry.image_path.as_deref());
        assert_eq!(result.meta.image_width, Some(8));
        assert_eq!(result.meta.image_height, Some(4));
        assert_eq!(result.meta.mime.as_deref(), Some("image/png"));
        let thumbnail = result.meta.thumbnail_data_url.as_deref().unwrap_or("");
        assert!(thumbnail.starts_with("data:image/png;base64,"));
        assert!(
            thumbnail.len() <= 128 * 1024,
            "thumbnail data URL must stay within the 128 KB contract bound"
        );
        // The preview resolves the same row through the registry and renders the
        // bounded thumbnail without decoding the full image.
        let payload = preview_for(&engine, &result, 4_000);
        assert_eq!(payload.format, "image");
        assert!(payload
            .image_data_url
            .as_deref()
            .unwrap_or("")
            .starts_with("data:image/png;base64,"));
        assert!(payload
            .fields
            .iter()
            .any(|field| field.label == "Dimensions" && field.value == "8 x 4"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
