//! Engine facade. OWNER: engine-core.
//!
//! Integration contract (do not break without L0 approval):
//!   Engine::open(config) -> Engine
//!   Engine::search(&SearchRequest) -> SearchResponse
//!   Engine::status() -> EngineStatus
//!   Engine::settings() -> Settings
//!   Engine::update_settings(SettingsPatch) -> anyhow::Result<Settings>
//!   Engine::activity(ActivityQuery) -> Vec<ActivityItem>
//!   Engine::onboarding() -> OnboardingState
//!   Engine::complete_onboarding(OnboardingInput) -> anyhow::Result<Settings>
//!   Engine::execute(ActionSpec) -> OpenOutcome
//!   Engine::index_command(IndexCommand) -> anyhow::Result<()>
//!   Engine::spawn_background()  (starts scanners; never blocks UI)

use std::path::PathBuf;
use std::sync::Mutex;

use ps_ipc::{
    ActionSpec, ActivityItem, ActivityQuery, EngineStatus, IndexCommand, IndexerState,
    OnboardingInput, OnboardingState, OpenOutcome, SearchRequest, SearchResponse, SearchStatus,
    Settings, SettingsPatch,
};

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub data_dir: PathBuf,
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

pub struct Engine {
    config: EngineConfig,
    settings: Mutex<Settings>,
}

impl Engine {
    pub fn open(config: EngineConfig) -> anyhow::Result<Self> {
        crate::paths::ensure_dirs()?;
        let settings = crate::settings::load(&crate::paths::settings_path());
        Ok(Self { config, settings: Mutex::new(settings) })
    }

    pub fn version(&self) -> &str {
        &self.config.version
    }

    pub fn search(&self, req: &SearchRequest) -> SearchResponse {
        SearchResponse {
            protocol: ps_ipc::PROTOCOL_VERSION,
            request_id: req.request_id,
            query: req.query.clone(),
            results: Vec::new(),
            total: 0,
            took_ms: 0.0,
            timings: Vec::new(),
            status: SearchStatus::Ready { indexed_files: 0, content_docs: 0 },
        }
    }

    pub fn status(&self) -> EngineStatus {
        EngineStatus {
            protocol: ps_ipc::PROTOCOL_VERSION,
            version: self.version().to_string(),
            state: IndexerState::Starting,
            filename_entries: 0,
            content_documents: 0,
            ocr_documents: 0,
            roots: Vec::new(),
            sources: Vec::new(),
            agent: None,
            last_updated: 0,
            warnings: Vec::new(),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    pub fn update_settings(&self, patch: SettingsPatch) -> anyhow::Result<Settings> {
        let mut guard = self.settings.lock().unwrap();
        let changed = guard.apply(&patch);
        let next = guard.clone();
        drop(guard);
        if changed {
            crate::settings::save(&crate::paths::settings_path(), &next)?;
        }
        Ok(next)
    }

    pub fn activity(&self, _query: ActivityQuery) -> Vec<ActivityItem> {
        Vec::new()
    }

    pub fn onboarding(&self) -> OnboardingState {
        OnboardingState {
            complete: self.settings().onboarding_complete,
            legacy: Default::default(),
            suggested_roots: Vec::new(),
            defaults: self.settings(),
        }
    }

    pub fn complete_onboarding(&self, _input: OnboardingInput) -> anyhow::Result<Settings> {
        anyhow::bail!("not implemented yet")
    }

    pub fn execute_action(&self, action: ActionSpec) -> OpenOutcome {
        ps_sources::actions::execute(&action)
    }

    pub fn index_command(&self, _cmd: IndexCommand) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn spawn_background(&self) {}
}
