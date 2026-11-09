//! Deterministic, realistic filename corpus generator. OWNER: perf-bench.
//!
//! Reproducibility contract: same (seed, count) always produces byte-identical
//! JSONL on any machine. "Now" for mtimes is fixed at FIXED_NOW_MS so time does
//! not leak into the corpus. Anchors are planted at every size so the fixed
//! query suite and golden relevance fixtures work at 10k, 100k and 1M.

use std::collections::HashSet;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::fnv1a64_file;
use crate::sha256_via_certutil;

/// Fixed "now" for deterministic mtimes: 2026-11-09T00:00:00Z in unix ms.
pub const FIXED_NOW_MS: i64 = 1_794_182_400_000;
pub const DEFAULT_SEED: u64 = 20_261_109;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetEntry {
    pub path: String,
    pub name: String,
    pub ext: String,
    pub kind: String,
    pub size: u64,
    pub modified: i64,
}

impl DatasetEntry {
    pub fn is_folder(&self) -> bool {
        self.kind == "folder"
    }
}

/// splitmix64: tiny, deterministic, good enough for corpus synthesis.
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed ^ 0x9E37_79B9_7F4A_7C15 }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }

    pub fn ratio(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }
}

pub struct Anchor {
    pub path: &'static str,
    pub name: &'static str,
}

pub const LONG_ANCHOR_PATH: &str = r"C:\Users\Pranshul\Documents\Archive\2026\Q3\Backups\Projects\protonsearch\apps\desktop\src\components\internal\generated\very_long_anchor_filename_for_path_length_testing_0001.json";

pub const ANCHORS: &[Anchor] = &[
    Anchor {
        path: r"C:\Users\Pranshul\Documents\Projects\ProtonSearch Phoenix\docs\architecture\Project Phoenix Budget 2026.xlsx",
        name: "Project Phoenix Budget 2026.xlsx",
    },
    Anchor {
        path: r"C:\Users\Pranshul\Documents\Finance\2026\Q3\quarterly_report_final_v2.pdf",
        name: "quarterly_report_final_v2.pdf",
    },
    Anchor {
        path: r"C:\Users\Pranshul\Pictures\Screenshots\Screenshot 2026-11-09 142233.png",
        name: "Screenshot 2026-11-09 142233.png",
    },
    Anchor {
        path: r"C:\Users\Pranshul\Documents\会議\会議議事録_2026-11-09.docx",
        name: "会議議事録_2026-11-09.docx",
    },
    Anchor {
        path: r"C:\Users\Pranshul\Documents\Notes\café_résumé_naïve.md",
        name: "café_résumé_naïve.md",
    },
    Anchor {
        path: r"C:\Users\Pranshul\Projects\web\src\index.js",
        name: "index.js",
    },
    Anchor {
        path: r"C:\Users\Pranshul\Projects\api\src\index.js",
        name: "index.js",
    },
    Anchor {
        path: r"C:\Users\Pranshul\Projects\admin\src\index.js",
        name: "index.js",
    },
    Anchor {
        path: r"C:\Users\Pranshul\Downloads\.env",
        name: ".env",
    },
    Anchor {
        path: LONG_ANCHOR_PATH,
        name: "very_long_anchor_filename_for_path_length_testing_0001.json",
    },
    Anchor {
        path: r"C:\Users\Pranshul\Documents\日本語\日本語_メモ_2026.txt",
        name: "日本語_メモ_2026.txt",
    },
    Anchor {
        path: r"C:\Users\Pranshul\Documents\Projects\ProtonSearch Phoenix",
        name: "ProtonSearch Phoenix",
    },
    Anchor {
        path: r"C:\Users\Pranshul\Projects\protonsearch\README.md",
        name: "README.md",
    },
    Anchor {
        path: r"C:\Users\Pranshul\Projects\protonsearch\apps\desktop\package.json",
        name: "package.json",
    },
];

const ROOTS: &[(&str, u32)] = &[
    (r"C:\Users\Pranshul\Documents", 26),
    (r"C:\Users\Pranshul\Downloads", 14),
    (r"C:\Users\Pranshul\Desktop", 10),
    (r"C:\Users\Pranshul\Pictures", 10),
    (r"C:\Users\Pranshul\Videos", 5),
    (r"C:\Users\Pranshul\Music", 3),
    (r"C:\Users\Pranshul\OneDrive", 6),
    (r"C:\Users\Pranshul\AppData\Local", 6),
    (r"C:\Program Files", 4),
    (r"C:\ProgramData", 2),
    (r"C:\Windows\System32", 2),
    (r"D:\Data", 2),
    (r"E:\Backup", 2),
    (r"\\fileserver\share", 1),
];

const DIRS: &[&str] = &[
    "Documents", "Downloads", "Desktop", "Pictures", "Videos", "Music", "Projects", "Work",
    "Archive", "Backup", "Backups", "2023", "2024", "2025", "2026", "Q1", "Q2", "Q3", "Q4",
    "src", "docs", "assets", "images", "reports", "invoices", "notes", "config", "tests",
    "node_modules", "target", ".git", "tools", "scripts", "build", "dist", "temp", "screenshots",
    "receipts", "contracts", "research", "personal", "shared", "media", "exports", "generated",
    "internal", "app", "server", "client", "public", "private", "old", "new", "final", "draft",
    "vendor", "bin", "lib", "out", "logs", "cache", "data", "misc", "unsorted", "clients",
];

const PROJECTS: &[&str] = &[
    "protonsearch", "phoenix", "apollo", "atlas", "nebula", "omega", "helios", "quantum",
    "fusion", "orion", "spectra", "vertex", "nova", "zenith", "aurora", "cipher", "matrix",
    "relay", "pulse", "beacon", "prism", "solaris", "titan", "vector",
];

const WORDS: &[&str] = &[
    "project", "report", "budget", "quarterly", "annual", "meeting", "notes", "draft", "final",
    "plan", "strategy", "review", "summary", "analysis", "alpha", "beta", "design", "system",
    "architecture", "spec", "proposal", "contract", "invoice", "receipt", "tax", "payroll",
    "research", "paper", "thesis", "chapter", "presentation", "slides", "demo", "prototype",
    "roadmap", "backlog", "sprint", "retro", "release", "changelog", "readme", "guide",
    "manual", "tutorial", "example", "sample", "template", "config", "settings", "backup",
    "export", "import", "data", "dataset", "metrics", "dashboard", "screenshot", "capture",
    "photo", "image", "video", "audio", "music", "track", "album", "resume", "cv", "cover",
    "letter", "application", "form", "survey", "questionnaire", "agenda", "minutes", "memo",
    "policy", "handbook", "onboarding", "training", "course", "lesson", "homework", "assignment",
    "lab", "experiment", "results", "findings", "conclusion", "appendix", "reference",
    "benchmark", "profile", "trace", "debug", "error", "issue", "ticket", "bug", "feature",
    "client", "customer", "vendor", "supplier", "market", "sales", "revenue", "expense",
    "salary", "offer", "contract", "nda", "compliance", "audit", "security", "privacy",
    "network", "server", "database", "schema", "migration", "seed", "fixture", "mock",
    "static", "dynamic", "async", "worker", "queue", "cache", "index", "search", "query",
    "filter", "sort", "merge", "split", "parse", "render", "paint", "layout", "style", "theme",
    "color", "font", "icon", "button", "panel", "window", "dialog", "toast", "menu", "list",
    "grid", "table", "chart", "graph", "map", "route", "page", "view", "model", "controller",
    "service", "handler", "middleware", "adapter", "factory", "builder", "helper", "util",
    "common", "core", "shared", "local", "remote", "cloud", "offline", "sync", "update",
];

const CODE_NAMES: &[&str] = &[
    "index.js", "main.rs", "mod.rs", "README.md", "package.json", "Cargo.toml", "tsconfig.json",
    "Dockerfile", ".gitignore", "lib.rs", "utils.ts", "app.tsx", "style.css", "setup.py",
    "requirements.txt", "Makefile", "CMakeLists.txt", "index.ts", "types.ts", "test.rs",
    "worker.rs", "build.rs", "vite.config.ts", "eslint.config.js", ".env.example",
];

const CJK_NAMES: &[&str] = &[
    "会議議事録", "報告書", "請求書", "契約書", "議事録", "見積書", "社内報", "研究ノート",
    "プレゼン資料", "設計書", "仕様書", "テスト結果", "회의록", "보고서", "견적서", "회사소개서",
    "日本語_メモ", "中文_笔记", "会议纪要", "项目计划",
];

const UNICODE_NAMES: &[&str] = &[
    "café_résumé", "naïve_implementation", "über_notes", "résumé_2026", "Straße_plan",
    "façade_review", "señor_contrato", "größe_metrics", "déjà_vu", "pièce_jointe",
];

const EXTENSIONS: &[(&str, u32)] = &[
    ("pdf", 12), ("docx", 9), ("xlsx", 8), ("pptx", 4), ("txt", 8), ("md", 8), ("json", 6),
    ("yaml", 2), ("yml", 1), ("csv", 3), ("log", 5), ("html", 2), ("css", 3), ("js", 8),
    ("ts", 7), ("tsx", 4), ("jsx", 2), ("py", 6), ("rs", 5), ("java", 3), ("cpp", 3), ("h", 2),
    ("go", 2), ("sql", 2), ("png", 9), ("jpg", 7), ("jpeg", 3), ("gif", 2), ("svg", 2),
    ("webp", 2), ("mp3", 2), ("mp4", 3), ("zip", 3), ("7z", 1), ("epub", 1), ("exe", 1),
    ("dll", 1), ("db", 2), ("ini", 3), ("toml", 3), ("lock", 2), ("heic", 1), ("avif", 1),
    ("wav", 1), ("bak", 2), ("tmp", 1), ("iso", 1), ("bin", 1),
];

fn pick_weighted<T: Copy>(rng: &mut Rng, items: &[(T, u32)]) -> T {
    let total: u32 = items.iter().map(|(_, w)| *w).sum();
    let mut roll = rng.below(total as u64) as u32;
    for (item, w) in items {
        if roll < *w {
            return *item;
        }
        roll -= *w;
    }
    items[0].0
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn ext_of(name: &str) -> String {
    if name.starts_with('.') && name[1..].find('.').is_none() {
        return String::new();
    }
    match name.rfind('.') {
        Some(i) if i + 1 < name.len() => name[i + 1..].to_ascii_lowercase(),
        _ => String::new(),
    }
}

fn make_name(rng: &mut Rng) -> String {
    let roll = rng.below(100);
    let ext = pick_weighted(rng, EXTENSIONS).to_string();
    if roll < 30 {
        format!(
            "{} {} {}.{}",
            capitalize(rng.pick(WORDS)),
            capitalize(rng.pick(WORDS)),
            capitalize(rng.pick(WORDS)),
            ext
        )
    } else if roll < 42 {
        format!("{}_{}_{}.{}", *rng.pick(WORDS), *rng.pick(WORDS), *rng.pick(WORDS), ext)
    } else if roll < 50 {
        let day = 1 + rng.below(28);
        let year = 2019 + rng.below(8);
        let hour = rng.below(24);
        let minute = rng.below(60);
        match rng.below(3) {
            0 => format!("Screenshot {year}-11-{day:02} {hour:02}{minute:02}{:02}.png", rng.below(60)),
            1 => format!("IMG_{year}11{day:02}_{hour:02}{minute:02}{:02}.{}", rng.below(60), ext),
            _ => format!("{year}-11-{day:02}_meeting_notes.{}", ext),
        }
    } else if roll < 56 {
        let project = capitalize(rng.pick(PROJECTS));
        format!(
            "{} {} v{}.{}.{}.{}",
            project,
            capitalize(rng.pick(WORDS)),
            1 + rng.below(4),
            rng.below(20),
            rng.below(20),
            ext
        )
    } else if roll < 62 {
        rng.pick(CODE_NAMES).to_string()
    } else if roll < 68 {
        let cjk = *rng.pick(CJK_NAMES);
        match rng.below(3) {
            0 => format!("{cjk}_{}.docx", 2020 + rng.below(7)),
            1 => format!("{cjk}_{:02}.xlsx", 1 + rng.below(12)),
            _ => format!("{cjk}.pdf"),
        }
    } else if roll < 72 {
        format!("{}_{}.{}", *rng.pick(UNICODE_NAMES), rng.below(1000), ext)
    } else if roll < 76 {
        format!(".{}", *rng.pick(WORDS))
    } else if roll < 80 {
        let n = 1000 + rng.below(9000);
        format!("invoice-{n}-{:02}.pdf", 1 + rng.below(12))
    } else if roll < 83 {
        format!("receipt_{}_{}.{}", 2020 + rng.below(7), 100 + rng.below(900), ext)
    } else if roll < 86 {
        let uuid = (0..3).map(|_| format!("{:08x}", rng.next_u64() as u32)).collect::<Vec<_>>().join("-");
        format!("{uuid}.{}", ext)
    } else if roll < 90 {
        let base = format!(
            "{} {} — {}",
            capitalize(rng.pick(WORDS)),
            capitalize(rng.pick(WORDS)),
            capitalize(rng.pick(WORDS))
        );
        format!("{base}.{}", ext)
    } else {
        format!("{}.{}", *rng.pick(WORDS), ext)
    }
}

fn make_long_name(rng: &mut Rng) -> String {
    let mut stem = String::new();
    for _ in 0..6 + rng.below(6) {
        if !stem.is_empty() {
            stem.push('_');
        }
        stem.push_str(rng.pick(WORDS));
    }
    if stem.len() > 150 {
        stem.truncate(150);
    }
    format!("{stem}_{}.json", rng.below(1000))
}

fn make_path(rng: &mut Rng, is_folder: bool) -> (String, String) {
    let root = pick_weighted(rng, ROOTS);
    let mut path = String::from(root);
    let long = rng.below(100) < 2;
    let depth = 1 + rng.below(6) + if long { 4 + rng.below(5) } else { 0 };
    for _ in 0..depth {
        path.push('\\');
        path.push_str(rng.pick(DIRS));
    }
    if long {
        path.push('\\');
        path.push_str(rng.pick(DIRS));
        path.push('\\');
        path.push_str(rng.pick(DIRS));
    }
    let name = if is_folder {
        rng.pick(DIRS).to_string()
    } else if long {
        make_long_name(rng)
    } else {
        make_name(rng)
    };
    path.push('\\');
    path.push_str(&name);
    (path, name)
}

/// Core deterministic generator; sink receives every entry (anchors last).
fn generate_with(seed: u64, count: usize, sink: &mut dyn FnMut(&DatasetEntry)) -> GenStats {
    let start = Instant::now();
    let mut rng = Rng::new(seed);
    let mut seen: HashSet<u64> = HashSet::with_capacity(count + 16);
    let anchor_count = ANCHORS.len();
    let filler = count.saturating_sub(anchor_count);

    let mut files = 0u64;
    let mut folders = 0u64;
    let mut hidden = 0u64;
    let mut cjk = 0u64;
    let mut non_ascii = 0u64;
    let mut long_paths = 0u64;
    let mut ext_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

    for i in 0..filler.max(1) {
        let is_folder = rng.below(100) < 8;
        let (mut path, mut name) = make_path(&mut rng, is_folder);
        let mut hash = crate::fnv1a64(path.as_bytes());
        if !seen.insert(hash) {
            // Collision: deterministically perturb the leaf name.
            let suffix = format!("_{i}");
            if let Some(dot) = name.rfind('.') {
                let (stem, ext) = name.split_at(dot);
                name = format!("{stem}{suffix}{ext}");
            } else {
                name = format!("{name}{suffix}");
            }
            path = format!("{}\\{name}", path.rsplit_once('\\').map(|(p, _)| p).unwrap_or(""));
            hash = crate::fnv1a64(path.as_bytes());
            seen.insert(hash);
        }
        let ext = if is_folder { String::new() } else { ext_of(&name) };
        let size = if is_folder {
            0
        } else {
            let scales: &[(u64, u32)] = &[
                (1, 20), (10, 15), (100, 12), (1_000, 12), (10_000, 10), (100_000, 10),
                (1_000_000, 8), (10_000_000, 6), (100_000_000, 4), (1_000_000_000, 3),
            ];
            let scale = pick_weighted(&mut rng, &scales);
            scale * (1 + rng.below(1000))
        };
        let age_days = rng.below(800) as i64;
        let modified = FIXED_NOW_MS - age_days * 86_400_000 - rng.below(86_400_000) as i64;
        let entry = DatasetEntry {
            path,
            name,
            ext,
            kind: if is_folder { "folder".to_string() } else { "file".to_string() },
            size,
            modified,
        };
        if entry.is_folder() {
            folders += 1;
        } else {
            files += 1;
        }
        if entry.name.starts_with('.') {
            hidden += 1;
        }
        if entry.path.chars().any(is_cjk) {
            cjk += 1;
        }
        if !entry.path.is_ascii() {
            non_ascii += 1;
        }
        if entry.path.chars().count() > 200 {
            long_paths += 1;
        }
        if !entry.ext.is_empty() {
            *ext_counts.entry(entry.ext.clone()).or_insert(0) += 1;
        }
        sink(&entry);
    }

    let mut anchors = 0u64;
    for anchor in ANCHORS {
        let ext = if anchor.path.ends_with("ProtonSearch Phoenix") {
            String::new()
        } else {
            ext_of(anchor.name)
        };
        let entry = DatasetEntry {
            path: anchor.path.to_string(),
            name: anchor.name.to_string(),
            ext: ext.clone(),
            kind: if ext.is_empty() && !anchor.name.contains('.') { "folder".to_string() } else { "file".to_string() },
            size: if anchor.name.contains('.') { 4096 * (1 + (anchor.path.len() as u64 % 128)) } else { 0 },
            modified: FIXED_NOW_MS - (anchor.path.len() as i64 * 3600_000),
        };
        if entry.name.starts_with('.') {
            hidden += 1;
        }
        if entry.path.chars().any(is_cjk) {
            cjk += 1;
        }
        if !entry.path.is_ascii() {
            non_ascii += 1;
        }
        if entry.path.chars().count() > 200 {
            long_paths += 1;
        }
        if !entry.ext.is_empty() {
            *ext_counts.entry(entry.ext.clone()).or_insert(0) += 1;
        }
        anchors += 1;
        sink(&entry);
    }

    let mut top_exts: Vec<(String, u64)> = ext_counts.into_iter().collect();
    top_exts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    top_exts.truncate(20);

    GenStats {
        seed,
        requested: count as u64,
        entries: filler.max(1) as u64 + anchors,
        files,
        folders,
        hidden,
        cjk,
        non_ascii,
        long_paths,
        anchors,
        top_exts,
        generated_ms: start.elapsed().as_secs_f64() * 1000.0,
        bytes: 0,
        fnv1a64: String::new(),
        sha256: None,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenStats {
    pub seed: u64,
    pub requested: u64,
    pub entries: u64,
    pub files: u64,
    pub folders: u64,
    pub hidden: u64,
    pub cjk: u64,
    pub non_ascii: u64,
    pub long_paths: u64,
    pub anchors: u64,
    pub top_exts: Vec<(String, u64)>,
    pub generated_ms: f64,
    pub bytes: u64,
    pub fnv1a64: String,
    pub sha256: Option<String>,
}

/// In-memory generation (used by tests and the relevance harness).
pub fn generate_entries(seed: u64, count: usize) -> Vec<DatasetEntry> {
    let mut out = Vec::with_capacity(count);
    generate_with(seed, count, &mut |e| out.push(e.clone()));
    out
}

/// Streaming generation to JSONL; returns corpus facts + hashes.
pub fn generate_to_file(seed: u64, count: usize, out: &Path) -> anyhow::Result<GenStats> {
    if let Some(dir) = out.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }
    let file = std::fs::File::create(out)?;
    let mut writer = BufWriter::with_capacity(1 << 20, file);
    let mut stats = generate_with(seed, count, &mut |entry| {
        serde_json::to_writer(&mut writer, entry).expect("serialize dataset entry");
        writer.write_all(b"\n").expect("write dataset entry");
    });
    writer.flush()?;
    drop(writer);
    let (bytes, hash) = fnv1a64_file(out)?;
    stats.bytes = bytes;
    stats.fnv1a64 = format!("{hash:016x}");
    stats.sha256 = sha256_via_certutil(out);
    // Persist the meta next to the dataset so results can cite it.
    let meta_path = out.with_extension("meta.json");
    crate::write_json(&meta_path, &stats)?;
    Ok(stats)
}

/// Load a dataset JSONL into memory (for the filename harness).
pub fn load_entries(path: &Path) -> anyhow::Result<Vec<DatasetEntry>> {
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let e: DatasetEntry = serde_json::from_str(line)
            .map_err(|err| anyhow::anyhow!("{}:{}: {err}", path.display(), i + 1))?;
        out.push(e);
    }
    Ok(out)
}

pub fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF)
}
