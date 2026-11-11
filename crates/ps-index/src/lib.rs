//! Compact, in-memory filename index. OWNER: engine-core.
//!
//! Design (measured in examples/bench.rs, see report):
//! - Strings live in one arena; entries are 32-byte fixed records that store
//!   only offsets. Directory paths are de-duplicated in a small dir table, so a
//!   1M-entry index costs roughly 40 bytes/entry plus the names themselves.
//! - A per-entry 64-bit bloom over name characters is the fuzzy-safe prefilter
//!   (ASCII letters/digits get exact bits; other Unicode chars use a 2-bit hash).
//! - Search: exact/prefix hits come from a name-sorted index via binary search;
//!   everything else scans the bloom array with rayon and keeps a bounded top-k.
//! - Ranking is total and deterministic: score, shorter name, newer mtime,
//!   lexicographic path, entry index.
//!
//! The public API below is the integration contract used by ps-core and
//! ps-bench. You may add to it, but do not remove/rename these items without
//! L0 approval.

pub mod matching;
mod persist;
mod scan;
#[cfg(test)]
mod testutil;

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use rayon::prelude::*;

pub use scan::{default_excluded_paths, scan, scan_roots, walk_paths, ScanOptions, DEFAULT_JUNK_DIRS};

pub use matching::{NameMatch, NameQuery};

use matching::{
    bloom_add, cmp_ci, ends_with_ci, eq_ci, find_ci, match_name_into, starts_with_ci, to_utf16_ranges,
    typo_only_match, QueryPlan, SCORE_EXACT_NAME, SCORE_EXACT_TOKEN, SCORE_PREFIX, SCORE_SCALE,
    SCORE_SPANNING, SCORE_SUBSTRING, SCORE_TYPO,
};

/// Below this many entries the rayon wake-up cost dominates, so scans run
/// serially. Measured on this host: a 10k-entry scan is ~10x faster serial.
const SERIAL_SCAN_THRESHOLD: usize = 48_000;
const SCAN_CHUNK: usize = 16_384;

/// Dedicated scan pool. A small pool (2..=8 threads) beats the global pool
/// here: waking 20 threads costs hundreds of microseconds per query.
fn scan_pool() -> &'static rayon::ThreadPool {
    static POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .clamp(2, 8);
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|i| format!("ps-index-scan-{i}"))
            .build()
            .unwrap_or_else(|_| {
                rayon::ThreadPoolBuilder::new()
                    .num_threads(2)
                    .build()
                    .expect("ps-index scan pool")
            })
    })
}

/// Map chunk indices to results, serially for small inputs and on the
/// dedicated pool otherwise.
fn parallel_chunks<T, F>(n: usize, chunk: usize, f: F) -> Vec<T>
where
    T: Send,
    F: Fn(usize) -> T + Sync + Send,
{
    let nchunks = n.div_ceil(chunk);
    if nchunks <= 1 || n <= SERIAL_SCAN_THRESHOLD {
        (0..nchunks).map(f).collect()
    } else {
        scan_pool().install(|| (0..nchunks).into_par_iter().map(f).collect())
    }
}

/// One indexed filesystem entry, in owned form (scan output / external input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexEntry {
    pub path: String,
    pub name: String,
    pub ext: String,
    pub kind: EntryKind,
    pub size: u64,
    /// Unix milliseconds.
    pub modified: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Folder,
}

impl EntryKind {
    #[inline]
    pub(crate) fn code(self) -> u8 {
        match self {
            EntryKind::File => 0,
            EntryKind::Folder => 1,
        }
    }

    #[inline]
    pub(crate) fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(EntryKind::File),
            1 => Some(EntryKind::Folder),
            _ => None,
        }
    }
}

/// A ranked filename hit. entry_index addresses the index (see entry_path).
#[derive(Debug, Clone, PartialEq)]
pub struct FileHit {
    pub entry_index: usize,
    pub score: f32,
    /// UTF-16 ranges of the matched characters within the name, for highlighting.
    pub ranges: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, Default)]
pub struct ScanStats {
    pub entries: usize,
    pub errors: usize,
    pub elapsed_ms: f32,
}

// ---------------------------------------------------------------------------
// Compact internal representation
// ---------------------------------------------------------------------------

const NO_DIR: u32 = u32::MAX;
pub(crate) const KIND_TOMBSTONE: u8 = 2;

#[derive(Clone, Copy, Debug)]
pub(crate) struct EntryRec {
    pub(crate) name_off: u32,
    pub(crate) name_len: u16,
    pub(crate) ext_len: u8,
    pub(crate) kind: u8,
    pub(crate) dir: u32,
    pub(crate) size: u64,
    pub(crate) modified: i64,
}

impl EntryRec {
    pub(crate) const SIZE: usize = std::mem::size_of::<EntryRec>();
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DirRec {
    pub(crate) path_off: u32,
    pub(crate) path_len: u16,
    pub(crate) reserved: u16,
}

/// One candidate during ranking. score is fixed-point so ordering is exact.
#[derive(Clone, Debug)]
pub(crate) struct Candidate {
    pub(crate) idx: u32,
    pub(crate) score: i64,
    pub(crate) name_len: u16,
    pub(crate) modified: i64,
    pub(crate) ranges: Vec<(usize, usize)>,
}

/// Optional structured filter applied while collecting candidates.
#[derive(Debug, Clone, Default)]
pub struct EntryFilter<'a> {
    pub kind: Option<EntryKind>,
    /// Lower-case extensions without the dot; empty means any extension.
    pub extensions: &'a [String],
    /// Only entries under this directory prefix.
    pub under_path: Option<&'a str>,
    pub modified_after: Option<i64>,
    pub modified_before: Option<i64>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
}

impl EntryFilter<'_> {
    pub fn is_empty(&self) -> bool {
        self.kind.is_none()
            && self.extensions.is_empty()
            && self.under_path.is_none()
            && self.modified_after.is_none()
            && self.modified_before.is_none()
            && self.min_size.is_none()
            && self.max_size.is_none()
    }
}

/// In-memory filename index.
#[derive(Debug, Default)]
pub struct FilenameIndex {
    pub(crate) arena: Vec<u8>,
    pub(crate) entries: Vec<EntryRec>,
    pub(crate) blooms: Vec<u64>,
    pub(crate) dirs: Vec<DirRec>,
    /// Per-directory character bloom over the full directory path; keeps the
    /// path-match phase cheap (most directories are rejected in ~2ns).
    pub(crate) dir_blooms: Vec<u64>,
    pub(crate) sorted: Vec<u32>,
    pub(crate) roots: Vec<String>,
    pub(crate) root_counts: Vec<u64>,
    pub(crate) built_at: i64,
    pub(crate) live: usize,
    /// Lazily materialized owned view for entries(); never used by hot paths.
    cache: OnceLock<Vec<IndexEntry>>,
    /// Lazily built directory -> id map for incremental updates.
    dir_map: Option<HashMap<String, u32>>,
}

// ---------------------------------------------------------------------------
// Small path helpers
// ---------------------------------------------------------------------------

pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[inline]
pub(crate) fn is_sep(b: u8) -> bool {
    b == b'\\' || b == b'/'
}

/// Normalize a configured root: forward slashes become native, trailing
/// separators are trimmed except for drive roots such as C:\.
pub fn normalize_root(path: &Path) -> String {
    let raw = path.to_string_lossy();
    let mut s: String = raw.replace('/', "\\");
    while s.len() > 3 && s.ends_with('\\') {
        s.pop();
    }
    if s.len() == 3 && s.as_bytes()[1] == b':' && s.as_bytes()[2] == b'\\' {
        return s;
    }
    if s.len() > 1 && s.ends_with('\\') {
        s.pop();
    }
    s
}

/// Parent directory of a path, or None for a drive root / empty path.
pub(crate) fn parent_dir(path: &str) -> Option<&str> {
    let bytes = path.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let mut i = bytes.len();
    while i > 0 {
        i -= 1;
        if is_sep(bytes[i]) {
            if i == 0 {
                return None;
            }
            // Drive root: parent of C:\foo is C:\, parent of C:\ is none.
            if i == 2 && bytes[1] == b':' {
                if bytes.len() == 3 {
                    return None;
                }
                return Some(&path[..3]);
            }
            return Some(&path[..i]);
        }
    }
    None
}

pub(crate) fn file_name(path: &str) -> &str {
    let bytes = path.as_bytes();
    let mut i = bytes.len();
    while i > 0 {
        i -= 1;
        if is_sep(bytes[i]) {
            return &path[i + 1..];
        }
    }
    path
}

/// Length of the extension part (bytes after the final dot), 0 when none.
/// Dotfiles such as .gitignore have no extension.
pub(crate) fn ext_len_of(name: &str) -> u8 {
    let bytes = name.as_bytes();
    let mut i = bytes.len();
    while i > 0 {
        i -= 1;
        if bytes[i] == b'.' {
            if i == 0 {
                return 0;
            }
            let len = name.len() - i - 1;
            return len.min(u8::MAX as usize) as u8;
        }
    }
    0
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

#[derive(Default)]
pub(crate) struct IndexBuilder {
    pub(crate) arena: Vec<u8>,
    pub(crate) entries: Vec<EntryRec>,
    pub(crate) blooms: Vec<u64>,
    pub(crate) dirs: Vec<DirRec>,
    pub(crate) dir_blooms: Vec<u64>,
    pub(crate) dir_map: HashMap<String, u32>,
    pub(crate) roots: Vec<String>,
    pub(crate) root_counts: Vec<u64>,
    pub(crate) errors: usize,
}

impl IndexBuilder {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn push_str(&mut self, s: &str) -> (u32, u16) {
        let off = self.arena.len() as u32;
        self.arena.extend_from_slice(s.as_bytes());
        (off, s.len().min(u16::MAX as usize) as u16)
    }

    /// Intern a full directory path, returning its stable id.
    pub(crate) fn intern_dir(&mut self, path: &str) -> u32 {
        if let Some(&id) = self.dir_map.get(path) {
            return id;
        }
        let id = self.dirs.len() as u32;
        let (off, len) = self.push_str(path);
        self.dirs.push(DirRec {
            path_off: off,
            path_len: len,
            reserved: 0,
        });
        self.dir_blooms.push(matching::bloom_for_str(path));
        self.dir_map.insert(path.to_string(), id);
        id
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn push_entry(
        &mut self,
        dir: u32,
        name: &str,
        kind: EntryKind,
        size: u64,
        modified: i64,
    ) -> usize {
        let (name_off, name_len) = self.push_str(name);
        let ext_len = ext_len_of(name);
        let mut bloom = 0u64;
        for c in name.chars() {
            bloom_add(&mut bloom, c);
        }
        self.entries.push(EntryRec {
            name_off,
            name_len,
            ext_len,
            kind: kind.code(),
            dir,
            size,
            modified,
        });
        self.blooms.push(bloom);
        self.entries.len() - 1
    }

    pub(crate) fn finish(mut self) -> FilenameIndex {
        let index = FilenameIndex {
            arena: std::mem::take(&mut self.arena),
            entries: std::mem::take(&mut self.entries),
            blooms: std::mem::take(&mut self.blooms),
            dirs: std::mem::take(&mut self.dirs),
            dir_blooms: std::mem::take(&mut self.dir_blooms),
            sorted: Vec::new(),
            roots: std::mem::take(&mut self.roots),
            root_counts: std::mem::take(&mut self.root_counts),
            built_at: now_ms(),
            live: 0,
            cache: OnceLock::new(),
            dir_map: None,
        };
        index.with_sorted_rebuilt()
    }

    pub(crate) fn finish_with_stats(self) -> (FilenameIndex, ScanStats) {
        let errors = self.errors;
        let index = self.finish();
        let entries = index.live;
        (
            index,
            ScanStats {
                entries,
                errors,
                elapsed_ms: 0.0,
            },
        )
    }
}

impl FilenameIndex {
    pub fn new() -> Self {
        Self::default()
    }

    fn with_sorted_rebuilt(mut self) -> Self {
        let mut sorted: Vec<u32> = (0..self.entries.len() as u32)
            .filter(|&i| self.entries[i as usize].kind != KIND_TOMBSTONE)
            .collect();
        self.sort_indices(&mut sorted);
        self.live = sorted.len();
        self.sorted = sorted;
        self
    }

    fn sort_indices(&self, sorted: &mut [u32]) {
        sorted.sort_unstable_by(|&a, &b| self.entry_sort_cmp(a, b));
    }

    /// Deterministic ordering for the name-sorted array: lower-cased name,
    /// then exact name bytes, then directory path, then index.
    pub(crate) fn entry_sort_cmp(&self, a: u32, b: u32) -> Ordering {
        sort_cmp_parts(&self.arena, &self.entries, &self.dirs, a, b)
    }

    pub fn len(&self) -> usize {
        self.live
    }

    pub fn is_empty(&self) -> bool {
        self.live == 0
    }

    pub fn roots(&self) -> &[String] {
        &self.roots
    }

    /// Number of unique directories in the dir table (diagnostic).
    pub fn dir_count(&self) -> usize {
        self.dirs.len()
    }

    pub fn built_at(&self) -> i64 {
        self.built_at
    }

    /// Approximate heap footprint of the index structures, in bytes.
    pub fn memory_bytes(&self) -> usize {
        self.arena.len()
            + self.entries.len() * EntryRec::SIZE
            + self.blooms.len() * 8
            + self.dirs.len() * std::mem::size_of::<DirRec>()
            + self.dir_blooms.len() * 8
            + self.sorted.len() * 4
            + self.roots.iter().map(|r| r.len() + 24).sum::<usize>()
            + self.dir_map.as_ref().map(|m| m.len() * 48).unwrap_or(0)
    }

    // -- internal accessors -------------------------------------------------

    #[inline]
    pub(crate) fn arena_str(&self, off: u32, len: u16) -> &str {
        let start = off as usize;
        let end = start.saturating_add(len as usize);
        match self.arena.get(start..end) {
            Some(bytes) => std::str::from_utf8(bytes).unwrap_or(""),
            None => "",
        }
    }

    #[inline]
    pub(crate) fn dir_bytes(&self, dir: u32) -> &[u8] {
        if dir == NO_DIR {
            return &[];
        }
        match self.dirs.get(dir as usize) {
            Some(d) => {
                let start = d.path_off as usize;
                let end = start.saturating_add(d.path_len as usize);
                self.arena.get(start..end).unwrap_or(&[])
            }
            None => &[],
        }
    }

    #[inline]
    pub(crate) fn dir_path_slice(&self, dir: usize) -> &str {
        match self.dirs.get(dir) {
            Some(d) => self.arena_str(d.path_off, d.path_len),
            None => "",
        }
    }

    #[inline]
    fn rec(&self, i: usize) -> Option<&EntryRec> {
        self.entries.get(i)
    }

    #[inline]
    pub(crate) fn is_live(&self, i: usize) -> bool {
        self.entries
            .get(i)
            .map(|r| r.kind != KIND_TOMBSTONE)
            .unwrap_or(false)
    }

    /// Name of an entry, original case.
    pub fn entry_name(&self, i: usize) -> &str {
        match self.rec(i) {
            Some(r) => self.arena_str(r.name_off, r.name_len),
            None => "",
        }
    }

    /// Extension without the dot (empty when none).
    pub fn entry_ext(&self, i: usize) -> &str {
        let Some(r) = self.rec(i) else { return "" };
        let name = self.arena_str(r.name_off, r.name_len);
        if r.ext_len == 0 {
            return "";
        }
        let start = name.len().saturating_sub(r.ext_len as usize);
        &name[start..]
    }

    /// Name without its extension.
    pub fn entry_stem(&self, i: usize) -> &str {
        let Some(r) = self.rec(i) else { return "" };
        let name = self.arena_str(r.name_off, r.name_len);
        if r.ext_len == 0 {
            return name;
        }
        let cut = name.len().saturating_sub(r.ext_len as usize + 1);
        &name[..cut.min(name.len())]
    }

    pub fn entry_kind(&self, i: usize) -> EntryKind {
        match self.rec(i) {
            Some(r) => EntryKind::from_code(r.kind).unwrap_or(EntryKind::File),
            None => EntryKind::File,
        }
    }

    pub fn entry_size(&self, i: usize) -> u64 {
        self.rec(i).map(|r| r.size).unwrap_or(0)
    }

    pub fn entry_modified(&self, i: usize) -> i64 {
        self.rec(i).map(|r| r.modified).unwrap_or(0)
    }

    /// Write the full path of an entry into a reusable buffer.
    fn write_path_into(&self, i: usize, out: &mut String) {
        out.clear();
        let Some(r) = self.rec(i) else { return };
        let dir = self.dir_path_slice(r.dir as usize);
        out.push_str(dir);
        if !out.is_empty() && !out.ends_with('\\') && !out.ends_with('/') {
            out.push(std::path::MAIN_SEPARATOR);
        }
        out.push_str(self.arena_str(r.name_off, r.name_len));
    }

    /// Full path of an entry. Allocates; use only for the results you render.
    pub fn entry_path(&self, i: usize) -> Cow<'_, str> {
        let Some(r) = self.rec(i) else {
            return Cow::Borrowed("");
        };
        let dir = self.dir_path_slice(r.dir as usize);
        let name = self.arena_str(r.name_off, r.name_len);
        if dir.is_empty() {
            return Cow::Borrowed(name);
        }
        let mut out = String::with_capacity(dir.len() + 1 + name.len());
        self.write_path_into(i, &mut out);
        Cow::Owned(out)
    }

    /// Metadata for one indexed path without materializing the full entry list.
    pub fn metadata_for_path(&self, path: &str) -> Option<(u64, i64)> {
        let lower_name = file_name(path).to_lowercase();
        let index = self.find_by_name_and_path(&lower_name, path)?;
        Some((self.entry_size(index), self.entry_modified(index)))
    }

    /// Materialize an owned view (compat API). This is O(n) and allocates
    /// roughly 250 bytes per entry; hot paths must use entry_path/entry_name.
    pub fn entries(&self) -> &[IndexEntry] {
        self.cache.get_or_init(|| {
            let mut out = Vec::with_capacity(self.live);
            for i in 0..self.entries.len() {
                if !self.is_live(i) {
                    continue;
                }
                out.push(IndexEntry {
                    path: self.entry_path(i).into_owned(),
                    name: self.entry_name(i).to_string(),
                    ext: self.entry_ext(i).to_string(),
                    kind: self.entry_kind(i),
                    size: self.entry_size(i),
                    modified: self.entry_modified(i),
                });
            }
            out
        })
    }

    pub fn replace_all(&mut self, entries: Vec<IndexEntry>) {
        *self = Self::from_entries(entries);
    }

    /// Build an index from owned entries (fast construction path).
    pub fn from_entries(entries: Vec<IndexEntry>) -> Self {
        let mut builder = IndexBuilder::new();
        for entry in entries {
            let dir_path = parent_dir(&entry.path).unwrap_or("");
            let dir = builder.intern_dir(dir_path);
            let name = file_name(&entry.path);
            builder.push_entry(dir, name, entry.kind, entry.size, entry.modified);
        }
        builder.finish()
    }

    // -- ranking helpers ----------------------------------------------------

    fn cand_cmp(&self, a: &Candidate, b: &Candidate) -> Ordering {
        b.score
            .cmp(&a.score)
            .then_with(|| a.name_len.cmp(&b.name_len))
            .then_with(|| b.modified.cmp(&a.modified))
            .then_with(|| self.entry_path_cmp(a.idx, b.idx))
            .then_with(|| a.idx.cmp(&b.idx))
    }

    fn entry_path_cmp(&self, a: u32, b: u32) -> Ordering {
        let ra = &self.entries[a as usize];
        let rb = &self.entries[b as usize];
        self.dir_bytes(ra.dir).cmp(self.dir_bytes(rb.dir)).then_with(|| {
            let na = self.arena_str(ra.name_off, ra.name_len);
            let nb = self.arena_str(rb.name_off, rb.name_len);
            na.as_bytes().cmp(nb.as_bytes())
        })
    }

    fn hit_from_candidate(&self, c: Candidate) -> FileHit {
        FileHit {
            entry_index: c.idx as usize,
            score: c.score as f32 / SCORE_SCALE,
            ranges: c.ranges,
        }
    }

    fn merge_candidates(&self, partials: Vec<Vec<Candidate>>, limit: usize) -> Vec<FileHit> {
        let mut all: Vec<Candidate> = Vec::new();
        for mut p in partials {
            all.append(&mut p);
        }
        if all.len() > limit {
            all.sort_unstable_by(|a, b| self.cand_cmp(a, b));
            all.truncate(limit);
        } else {
            all.sort_unstable_by(|a, b| self.cand_cmp(a, b));
        }
        all.into_iter().map(|c| self.hit_from_candidate(c)).collect()
    }

    fn merge_hits(&self, a: Vec<FileHit>, b: Vec<FileHit>, limit: usize) -> Vec<FileHit> {
        if a.is_empty() {
            return b;
        }
        if b.is_empty() {
            return a;
        }
        let mut cands: Vec<Candidate> = Vec::with_capacity(a.len() + b.len());
        for h in a.into_iter().chain(b.into_iter()) {
            cands.push(Candidate {
                idx: h.entry_index as u32,
                score: (h.score * SCORE_SCALE) as i64,
                name_len: self.rec(h.entry_index).map(|r| r.name_len).unwrap_or(u16::MAX),
                modified: self.entry_modified(h.entry_index),
                ranges: h.ranges,
            });
        }
        cands.sort_unstable_by(|x, y| self.cand_cmp(x, y));
        cands.dedup_by_key(|c| c.idx);
        cands.truncate(limit);
        cands.into_iter().map(|c| self.hit_from_candidate(c)).collect()
    }

    fn compute_under(&self, filter: &EntryFilter<'_>) -> Option<Vec<bool>> {
        let under = filter.under_path?;
        let under = normalize_root(Path::new(under));
        if under.is_empty() {
            return None;
        }
        Some(
            (0..self.dirs.len())
                .map(|d| is_under(self.dir_path_slice(d), &under))
                .collect(),
        )
    }

    #[inline]
    fn filter_pass(&self, filter: &EntryFilter<'_>, i: usize, under: Option<&[bool]>) -> bool {
        let Some(r) = self.rec(i) else { return false };
        if r.kind == KIND_TOMBSTONE {
            return false;
        }
        if let Some(kind) = filter.kind {
            if r.kind != kind.code() {
                return false;
            }
        }
        if let Some(after) = filter.modified_after {
            if r.modified < after {
                return false;
            }
        }
        if let Some(before) = filter.modified_before {
            if r.modified > before {
                return false;
            }
        }
        if let Some(min) = filter.min_size {
            if r.size < min {
                return false;
            }
        }
        if let Some(max) = filter.max_size {
            if r.size > max {
                return false;
            }
        }
        if !filter.extensions.is_empty() {
            let ext = self.entry_ext(i);
            if !filter
                .extensions
                .iter()
                .any(|wanted| ext.eq_ignore_ascii_case(wanted))
            {
                return false;
            }
        }
        if let Some(under) = under {
            match under.get(r.dir as usize) {
                Some(true) => {}
                _ => return false,
            }
        }
        true
    }

    // -- search -------------------------------------------------------------

    /// Compact exact/prefix filename search with a hard top-k bound.
    pub fn search(&self, query: &str, limit: usize) -> Vec<FileHit> {
        self.search_filtered(query, limit, &EntryFilter::default())
    }

    pub fn search_filtered(&self, query: &str, limit: usize, filter: &EntryFilter<'_>) -> Vec<FileHit> {
        if limit == 0 || self.live == 0 {
            return Vec::new();
        }
        let plan = QueryPlan::new(query);
        if plan.is_empty() {
            return Vec::new();
        }
        let prefix = self.search_prefix_plan(&plan, limit, filter);
        // The scan can still find whole-word matches, which outrank a prefix;
        // one- and two-letter queries keep the cheap prefix-only answer.
        let floor = if plan.char_len <= 2 { SCORE_PREFIX } else { SCORE_EXACT_TOKEN };
        if prefix.len() >= limit
            && prefix
                .last()
                .map(|h| h.score >= floor - 1e-4)
                .unwrap_or(false)
        {
            return prefix;
        }
        let scan = self.search_scan_plan(&plan, limit, filter);
        self.merge_hits(prefix, scan, limit)
    }

    /// Tier-0 search: exact name, exact stem and name prefix only. Uses the
    /// sorted-name index, so it is O(log n + matches).
    pub fn search_prefix(&self, query: &str, limit: usize) -> Vec<FileHit> {
        self.search_prefix_filtered(query, limit, &EntryFilter::default())
    }

    pub fn search_prefix_filtered(
        &self,
        query: &str,
        limit: usize,
        filter: &EntryFilter<'_>,
    ) -> Vec<FileHit> {
        if limit == 0 || self.live == 0 {
            return Vec::new();
        }
        let plan = QueryPlan::new(query);
        if plan.is_empty() {
            return Vec::new();
        }
        self.search_prefix_plan(&plan, limit, filter)
    }

    /// Tier-1 search: word-boundary, substring, fuzzy and typo name matches. Never returns exact/prefix-only duplicates of tier 0 as such;
    /// callers may merge and de-duplicate by entry index.
    pub fn search_scan(&self, query: &str, limit: usize) -> Vec<FileHit> {
        self.search_scan_filtered(query, limit, &EntryFilter::default())
    }

    pub fn search_scan_filtered(&self, query: &str, limit: usize, filter: &EntryFilter<'_>) -> Vec<FileHit> {
        if limit == 0 || self.live == 0 {
            return Vec::new();
        }
        let plan = QueryPlan::new(query);
        if plan.is_empty() {
            return Vec::new();
        }
        self.search_scan_plan(&plan, limit, filter)
    }

    fn search_prefix_plan(&self, plan: &QueryPlan, limit: usize, filter: &EntryFilter<'_>) -> Vec<FileHit> {
        if plan.has_sep {
            return Vec::new();
        }
        let under = self.compute_under(filter);
        let q = &plan.lower;
        // Lower bound over the name-sorted array.
        let sorted = &self.sorted;
        let (mut lo, mut hi) = (0usize, sorted.len());
        while lo < hi {
            let mid = (lo + hi) / 2;
            let idx = sorted[mid] as usize;
            if cmp_ci(self.entry_name(idx), q) == Ordering::Less {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let cap = (limit.saturating_mul(8)).max(256).min(20_000);
        let mut top = TopK::new(self, limit);
        let mut seen = 0usize;
        let mut scratch: Vec<(usize, usize)> = Vec::new();
        let prefix_term = plan.terms.first().map(String::as_str).unwrap_or(q.as_str());
        for &idx in &sorted[lo..] {
            let i = idx as usize;
            let name = self.entry_name(i);
            if !starts_with_ci(name, prefix_term) {
                break;
            }
            seen += 1;
            if seen > cap {
                break;
            }
            if !self.filter_pass(filter, i, under.as_deref()) {
                continue;
            }
            let stem = self.entry_stem(i);
            let Some(score) = match_name_into(name, stem, plan, false, &mut scratch) else {
                continue;
            };
            let ranges = to_utf16_ranges(name, &scratch);
            top.push(Candidate {
                idx,
                score: (score * SCORE_SCALE) as i64,
                name_len: self.entries[i].name_len,
                modified: self.entries[i].modified,
                ranges,
            });
        }
        top.into_hits()
    }

    fn search_scan_plan(&self, plan: &QueryPlan, limit: usize, filter: &EntryFilter<'_>) -> Vec<FileHit> {
        if plan.has_sep {
            return self.search_spanning(plan, limit, filter);
        }
        let mut name_hits = self.scan_name_phase(plan, limit, filter);
        // Typo matches are the lowest tier: only scan for them when the top-k
        // is not already filled by stronger matches.
        let need_typo = name_hits.len() < limit
            || name_hits
                .last()
                .map(|h| h.score <= SCORE_TYPO + 1e-4)
                .unwrap_or(true);
        if need_typo {
            let typo_hits = self.scan_typo_phase(plan, limit, filter);
            name_hits = self.merge_hits(name_hits, typo_hits, limit);
        }
        name_hits
    }

    fn scan_name_phase(&self, plan: &QueryPlan, limit: usize, filter: &EntryFilter<'_>) -> Vec<FileHit> {
        let under = self.compute_under(filter);
        let n = self.entries.len();
        let partials: Vec<Vec<Candidate>> = parallel_chunks(n, SCAN_CHUNK, |c| {
            let mut top = TopK::new(self, limit);
            let mut scratch: Vec<(usize, usize)> = Vec::new();
            let start = c * SCAN_CHUNK;
            let end = ((c + 1) * SCAN_CHUNK).min(n);
            for i in start..end {
                let r = &self.entries[i];
                if r.kind == KIND_TOMBSTONE {
                    continue;
                }
                if !plan.bloom_pass(self.blooms[i]) {
                    continue;
                }
                // A match needs at least one character per query char.
                if (r.name_len as usize) < plan.char_len {
                    continue;
                }
                let name = self.arena_str(r.name_off, r.name_len);
                let stem = self.entry_stem(i);
                let Some(score) = match_name_into(name, stem, plan, false, &mut scratch) else {
                    continue;
                };
                if !top.would_accept(score, r.name_len, r.modified, i as u32) {
                    continue;
                }
                if !self.filter_pass(filter, i, under.as_deref()) {
                    continue;
                }
                let ranges = to_utf16_ranges(name, &scratch);
                top.push(Candidate {
                    idx: i as u32,
                    score: (score * SCORE_SCALE) as i64,
                    name_len: r.name_len,
                    modified: r.modified,
                    ranges,
                });
            }
            top.items
        });
        self.merge_candidates(partials, limit)
    }

    /// Dedicated typo pass: relaxed bloom plus the cheap bounded edit-distance
    /// check only. Runs only when the stronger top-k is not already full.
    fn scan_typo_phase(&self, plan: &QueryPlan, limit: usize, filter: &EntryFilter<'_>) -> Vec<FileHit> {
        let relaxed = plan.relaxed();
        let under = self.compute_under(filter);
        let n = self.entries.len();
        let partials: Vec<Vec<Candidate>> = parallel_chunks(n, SCAN_CHUNK, |c| {
            let mut top = TopK::new(self, limit);
            let mut scratch: Vec<(usize, usize)> = Vec::new();
            let start = c * SCAN_CHUNK;
            let end = ((c + 1) * SCAN_CHUNK).min(n);
            for i in start..end {
                let r = &self.entries[i];
                if r.kind == KIND_TOMBSTONE {
                    continue;
                }
                if !relaxed.bloom_pass(self.blooms[i]) {
                    continue;
                }
                // Typo candidates are only plausible within a small length
                // window and with a matching first character; this prunes the
                // vast majority of the million-entry scan before the edit
                // distance work.
                let name = self.arena_str(r.name_off, r.name_len);
                let stem_len = name
                    .len()
                    .saturating_sub(r.ext_len as usize + usize::from(r.ext_len > 0));
                let qlen = plan.char_len;
                let name_in_window = r.name_len as usize + 2 >= qlen && (r.name_len as usize) <= qlen + 2;
                let stem_in_window = stem_len + 2 >= qlen && stem_len <= qlen + 2;
                if !name_in_window && !stem_in_window {
                    continue;
                }
                if let Some(first) = plan.terms_chars.first().and_then(|c| c.first()) {
                    if name.as_bytes().first().map(|b| b.to_ascii_lowercase()) != Some(*first as u8) {
                        continue;
                    }
                }
                let stem = self.entry_stem(i);
                let score = if plan.multi {
                    // Multi-term typo queries are rare; reuse the full matcher
                    // (every term may be exact/fuzzy/typo).
                    match match_name_into(name, stem, &relaxed, true, &mut scratch) {
                        Some(score) => score,
                        None => continue,
                    }
                } else if typo_only_match(name, stem, plan) {
                    SCORE_TYPO
                } else {
                    continue;
                };
                if !top.would_accept(score, r.name_len, r.modified, i as u32) {
                    continue;
                }
                if !self.filter_pass(filter, i, under.as_deref()) {
                    continue;
                }
                top.push(Candidate {
                    idx: i as u32,
                    score: (score * SCORE_SCALE) as i64,
                    name_len: r.name_len,
                    modified: r.modified,
                    ranges: Vec::new(),
                });
            }
            top.items
        });
        self.merge_candidates(partials, limit)
    }

    /// Full-path queries (containing a path separator). Anchors on the basename
    /// through the bloom-filtered name scan, verifies candidates against the
    /// whole path, then falls back to a directory-anchored pass and finally a
    /// complete full-path scan. There is no arbitrary index cap: an anchor near
    /// the end of a 1M-entry index is still found.
    fn search_spanning(&self, plan: &QueryPlan, limit: usize, filter: &EntryFilter<'_>) -> Vec<FileHit> {
        let q = normalize_seps(&plan.lower);
        let sep = std::path::MAIN_SEPARATOR;
        let trimmed = q.trim_end_matches(sep);
        let trailing_sep = trimmed.len() != q.len();
        let (_, name_q) = match trimmed.rfind(sep) {
            Some(i) => (&trimmed[..i], &trimmed[i + 1..]),
            None => ("", trimmed),
        };
        let mut out: Vec<FileHit> = Vec::new();
        if !name_q.is_empty() && !trailing_sep {
            let name_plan = QueryPlan::new(name_q);
            let generous = limit.saturating_mul(16).max(64).min(2_000);
            let name_hits = self.scan_name_phase(&name_plan, generous, filter);
            out = self.verify_full_path(&name_hits, &q, limit);
        }
        // An exact full-path hit is the strongest possible signal; filling the
        // remaining slots with directory-prefix matches would be noise (and a
        // 1M-entry dir scan). Only widen the search when it is not exact.
        let has_exact = out
            .first()
            .map(|h| h.score >= SCORE_EXACT_NAME - 1e-4)
            .unwrap_or(false);
        if !has_exact && out.len() < limit {
            let dir_hits = self.search_dir_anchor(trimmed, trailing_sep, limit, filter);
            out = self.merge_hits(out, dir_hits, limit);
        }
        if out.is_empty() {
            // Correctness backstop: complete scan of reconstructed full paths.
            out = self.scan_full_paths(&q, limit, filter);
        }
        out
    }

    /// Keep only name-scan hits whose reconstructed full path contains the
    /// full-path query; score exact matches at the exact-name tier.
    fn verify_full_path(&self, hits: &[FileHit], q: &str, limit: usize) -> Vec<FileHit> {
        if hits.is_empty() {
            return Vec::new();
        }
        let mut cands: Vec<Candidate> = Vec::with_capacity(hits.len());
        let mut buf = String::with_capacity(192);
        for hit in hits {
            let i = hit.entry_index;
            if !self.is_live(i) {
                continue;
            }
            self.write_path_into(i, &mut buf);
            let Some((start, end)) = find_ci(&buf, q) else {
                continue;
            };
            let exact = start == 0 && end == buf.len();
            let score = if exact { SCORE_EXACT_NAME } else { SCORE_SUBSTRING };
            let name = self.entry_name(i);
            let ranges = if exact {
                vec![(0, name.encode_utf16().count())]
            } else {
                Vec::new()
            };
            cands.push(Candidate {
                idx: i as u32,
                score: (score * SCORE_SCALE) as i64,
                name_len: self.entries[i].name_len,
                modified: self.entries[i].modified,
                ranges,
            });
        }
        self.finish_candidates(cands, limit)
    }

    /// Directory-anchored pass: entries whose directory path equals, starts
    /// with, or ends with the query path. Covers trailing-separator listings
    /// and queries that point at a folder path.
    fn search_dir_anchor(
        &self,
        trimmed: &str,
        exact_only: bool,
        limit: usize,
        filter: &EntryFilter<'_>,
    ) -> Vec<FileHit> {
        if trimmed.is_empty() || self.dirs.is_empty() {
            return Vec::new();
        }
        let nd = self.dirs.len();
        const DIR_CHUNK: usize = 65_536;
        let parts: Vec<Vec<u8>> = parallel_chunks(nd, DIR_CHUNK, |c| {
            let start = c * DIR_CHUNK;
            let end = ((c + 1) * DIR_CHUNK).min(nd);
            let mut out = Vec::with_capacity(end - start);
            for d in start..end {
                let path = self.dir_path_slice(d);
                let hit = if path.is_empty() {
                    false
                } else if exact_only {
                    // Trailing-separator listing: only the directory itself and
                    // its descendants (no sibling-prefix false positives).
                    is_under(path, trimmed)
                } else {
                    eq_ci(path, trimmed) || starts_with_ci(path, trimmed) || ends_with_ci(path, trimmed)
                };
                out.push(hit as u8);
            }
            out
        });
        let mut dir_hit = Vec::with_capacity(nd);
        for mut part in parts {
            dir_hit.append(&mut part);
        }
        let under = self.compute_under(filter);
        let n = self.entries.len();
        let partials: Vec<Vec<Candidate>> = parallel_chunks(n, SCAN_CHUNK, |c| {
            let mut top = TopK::new(self, limit);
            let start = c * SCAN_CHUNK;
            let end = ((c + 1) * SCAN_CHUNK).min(n);
            for i in start..end {
                let r = &self.entries[i];
                if r.kind == KIND_TOMBSTONE {
                    continue;
                }
                if dir_hit.get(r.dir as usize).copied().unwrap_or(0) == 0 {
                    continue;
                }
                if !self.filter_pass(filter, i, under.as_deref()) {
                    continue;
                }
                top.push(Candidate {
                    idx: i as u32,
                    score: (SCORE_SPANNING * SCORE_SCALE) as i64,
                    name_len: r.name_len,
                    modified: r.modified,
                    ranges: Vec::new(),
                });
            }
            top.items
        });
        self.merge_candidates(partials, limit)
    }

    /// Complete scan over reconstructed full paths (no index cap). Only used as
    /// a correctness backstop when the anchored passes find nothing.
    fn scan_full_paths(&self, q: &str, limit: usize, filter: &EntryFilter<'_>) -> Vec<FileHit> {
        let under = self.compute_under(filter);
        let n = self.entries.len();
        let partials: Vec<Vec<Candidate>> = parallel_chunks(n, SCAN_CHUNK, |c| {
            let mut top = TopK::new(self, limit);
            let mut buf = String::with_capacity(192);
            let start = c * SCAN_CHUNK;
            let end = ((c + 1) * SCAN_CHUNK).min(n);
            for i in start..end {
                let r = &self.entries[i];
                if r.kind == KIND_TOMBSTONE {
                    continue;
                }
                if !self.filter_pass(filter, i, under.as_deref()) {
                    continue;
                }
                self.write_path_into(i, &mut buf);
                let Some((start, end)) = find_ci(&buf, q) else {
                    continue;
                };
                let exact = start == 0 && end == buf.len();
                let score = if exact { SCORE_EXACT_NAME } else { SCORE_SPANNING };
                if !top.would_accept(score, r.name_len, r.modified, i as u32) {
                    continue;
                }
                top.push(Candidate {
                    idx: i as u32,
                    score: (score * SCORE_SCALE) as i64,
                    name_len: r.name_len,
                    modified: r.modified,
                    ranges: Vec::new(),
                });
            }
            top.items
        });
        self.merge_candidates(partials, limit)
    }

    fn finish_candidates(&self, mut cands: Vec<Candidate>, limit: usize) -> Vec<FileHit> {
        cands.sort_unstable_by(|a, b| self.cand_cmp(a, b));
        cands.dedup_by_key(|c| c.idx);
        cands.truncate(limit);
        cands.into_iter().map(|c| self.hit_from_candidate(c)).collect()
    }

    // -- incremental updates ------------------------------------------------

    fn ensure_dir_map(&mut self) -> &mut HashMap<String, u32> {
        if self.dir_map.is_none() {
            let mut map = HashMap::with_capacity(self.dirs.len());
            for (id, d) in self.dirs.iter().enumerate() {
                map.insert(self.arena_str(d.path_off, d.path_len).to_string(), id as u32);
            }
            self.dir_map = Some(map);
        }
        self.dir_map.as_mut().unwrap()
    }

    fn intern_dir_mut(&mut self, path: &str) -> u32 {
        if let Some(&id) = self.dir_map.as_ref().and_then(|m| m.get(path)) {
            return id;
        }
        if self.dir_map.is_none() {
            self.ensure_dir_map();
        }
        if let Some(&id) = self.dir_map.as_ref().and_then(|m| m.get(path)) {
            return id;
        }
        let off = self.arena.len() as u32;
        self.arena.extend_from_slice(path.as_bytes());
        let id = self.dirs.len() as u32;
        self.dirs.push(DirRec {
            path_off: off,
            path_len: path.len().min(u16::MAX as usize) as u16,
            reserved: 0,
        });
        self.dir_blooms.push(matching::bloom_for_str(path));
        self.dir_map.as_mut().unwrap().insert(path.to_string(), id);
        id
    }

    fn invalidate_cache(&mut self) {
        self.cache = OnceLock::new();
    }

    /// Insert or update a single entry (watcher path). Returns true when the
    /// entry was newly added.
    pub fn upsert(&mut self, entry: IndexEntry) -> bool {
        let name = file_name(&entry.path).to_string();
        let path = entry.path.clone();
        let lower = name.to_lowercase();
        if let Some(i) = self.find_by_name_and_path(&lower, &path) {
            let rec = &mut self.entries[i];
            rec.size = entry.size;
            rec.modified = entry.modified;
            let new_kind = entry.kind.code();
            if rec.kind != new_kind {
                rec.kind = new_kind;
            }
            self.invalidate_cache();
            return false;
        }
        let dir_path = parent_dir(&path).unwrap_or("").to_string();
        let dir = self.intern_dir_mut(&dir_path);
        let name_off = self.arena.len() as u32;
        self.arena.extend_from_slice(name.as_bytes());
        let ext_len = ext_len_of(&name);
        let mut bloom = 0u64;
        for c in name.chars() {
            bloom_add(&mut bloom, c);
        }
        let idx = self.entries.len() as u32;
        self.entries.push(EntryRec {
            name_off,
            name_len: name.len().min(u16::MAX as usize) as u16,
            ext_len,
            kind: entry.kind.code(),
            dir,
            size: entry.size,
            modified: entry.modified,
        });
        self.blooms.push(bloom);
        // Insert into the name-sorted array at the right position.
        let pos = self
            .sorted
            .partition_point(|&x| self.entry_sort_cmp(x, idx) == Ordering::Less);
        self.sorted.insert(pos, idx);
        self.live += 1;
        self.invalidate_cache();
        true
    }

    /// Remove one entry by full path (watcher path). Tombstones are compacted
    /// lazily; search skips them. Returns true when something was removed.
    pub fn remove_path(&mut self, path: &str) -> bool {
        let name = file_name(path);
        let lower = name.to_lowercase();
        if let Some(i) = self.find_by_name_and_path(&lower, path) {
            self.entries[i].kind = KIND_TOMBSTONE;
            self.live = self.live.saturating_sub(1);
            self.invalidate_cache();
            return true;
        }
        false
    }

    fn find_by_name_and_path(&self, lower_name: &str, path: &str) -> Option<usize> {
        let sorted = &self.sorted;
        let (mut lo, mut hi) = (0usize, sorted.len());
        while lo < hi {
            let mid = (lo + hi) / 2;
            let idx = sorted[mid] as usize;
            if cmp_ci(self.entry_name(idx), lower_name) == Ordering::Less {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        for &idx in &sorted[lo..] {
            let i = idx as usize;
            if !self.is_live(i) {
                continue;
            }
            if !eq_ci(self.entry_name(i), lower_name) {
                break;
            }
            let candidate = self.entry_path(i);
            if eq_ci(&candidate, path) {
                return Some(i);
            }
        }
        None
    }

    /// Drop tombstones and rebuild the sorted array. Cheap to call; no-op when
    /// there is nothing to compact.
    pub fn compact(&mut self) {
        let dead = self.entries.len().saturating_sub(self.live);
        if dead == 0 {
            return;
        }
        let mut new_entries = Vec::with_capacity(self.live);
        let mut new_blooms = Vec::with_capacity(self.live);
        for (i, rec) in self.entries.iter().enumerate() {
            if rec.kind == KIND_TOMBSTONE {
                continue;
            }
            new_entries.push(*rec);
            new_blooms.push(self.blooms[i]);
        }
        self.entries = new_entries;
        self.blooms = new_blooms;
        let mut sorted: Vec<u32> = (0..self.entries.len() as u32).collect();
        self.sort_indices(&mut sorted);
        self.sorted = sorted;
        self.live = self.entries.len();
        self.invalidate_cache();
    }

    /// Update the configured roots. Returns true when they changed (callers
    /// should trigger a full rebuild in that case).
    pub fn set_roots(&mut self, roots: Vec<String>) -> bool {
        let normalized: Vec<String> = roots
            .iter()
            .map(|r| normalize_root(Path::new(r)))
            .filter(|r| !r.is_empty())
            .collect();
        if normalized == self.roots {
            return false;
        }
        self.roots = normalized;
        self.recount_roots();
        true
    }

    fn recount_roots(&mut self) {
        let mut counts = vec![0u64; self.roots.len()];
        for i in 0..self.entries.len() {
            if !self.is_live(i) {
                continue;
            }
            let path = self.entry_path(i);
            for (r, root) in self.roots.iter().enumerate() {
                if is_under(&path, root) {
                    counts[r] += 1;
                    break;
                }
            }
        }
        self.root_counts = counts;
    }

    /// Per-root entry counts (aligned with roots()).
    pub fn root_counts(&self) -> &[u64] {
        &self.root_counts
    }
}

/// Normalize path separators in a query to the platform separator.
pub(crate) fn normalize_seps(s: &str) -> String {
    if std::path::MAIN_SEPARATOR == '\\' {
        s.replace('/', "\\")
    } else {
        s.replace('\\', "/")
    }
}

/// Comparator for the name-sorted array, usable before a FilenameIndex exists
/// (persistence validation avoids cloning the whole index).
pub(crate) fn sort_cmp_parts(
    arena: &[u8],
    entries: &[EntryRec],
    dirs: &[DirRec],
    a: u32,
    b: u32,
) -> Ordering {
    let str_of = |rec: &EntryRec| -> &str {
        let start = rec.name_off as usize;
        let end = start.saturating_add(rec.name_len as usize);
        arena
            .get(start..end)
            .and_then(|b| std::str::from_utf8(b).ok())
            .unwrap_or("")
    };
    let dir_of = |rec: &EntryRec| -> &[u8] {
        match dirs.get(rec.dir as usize) {
            Some(d) => {
                let start = d.path_off as usize;
                let end = start.saturating_add(d.path_len as usize);
                arena.get(start..end).unwrap_or(&[])
            }
            None => &[],
        }
    };
    let ra = &entries[a as usize];
    let rb = &entries[b as usize];
    let na = str_of(ra);
    let nb = str_of(rb);
    cmp_ci(na, nb)
        .then_with(|| na.as_bytes().cmp(nb.as_bytes()))
        .then_with(|| dir_of(ra).cmp(dir_of(rb)))
        .then_with(|| a.cmp(&b))
}

fn is_under(path: &str, root: &str) -> bool {
    if root.is_empty() {
        return true;
    }
    if eq_ci(path, root) {
        return true;
    }
    if !starts_with_ci(path, root) || path.len() <= root.len() {
        return false;
    }
    let next = path.as_bytes()[root.len()];
    is_sep(next)
}

/// Bounded top-k collector. Items are kept sorted best-first.
struct TopK<'a> {
    index: &'a FilenameIndex,
    limit: usize,
    items: Vec<Candidate>,
}

impl<'a> TopK<'a> {
    fn new(index: &'a FilenameIndex, limit: usize) -> Self {
        Self {
            index,
            limit,
            items: Vec::with_capacity(limit.min(64)),
        }
    }

    /// Cheap eligibility check before allocating highlight ranges.
    fn would_accept(&self, score: f32, name_len: u16, modified: i64, _idx: u32) -> bool {
        if self.limit == 0 {
            return false;
        }
        if self.items.len() < self.limit {
            return true;
        }
        let score = (score * SCORE_SCALE) as i64;
        match self.items.last() {
            Some(worst) => {
                score > worst.score
                    || (score == worst.score
                        && (name_len < worst.name_len
                            || (name_len == worst.name_len
                                && (modified > worst.modified || modified == worst.modified))))
            }
            None => true,
        }
    }

    fn push(&mut self, cand: Candidate) {
        if self.limit == 0 {
            return;
        }
        if self.items.len() < self.limit {
            let pos = self
                .items
                .binary_search_by(|probe| self.index.cand_cmp(probe, &cand))
                .unwrap_or_else(|e| e);
            self.items.insert(pos, cand);
            return;
        }
        // Compare against the worst kept candidate first (cheap).
        if self.index.cand_cmp(&cand, self.items.last().unwrap()) != Ordering::Less {
            return;
        }
        self.items.pop();
        let pos = self
            .items
            .binary_search_by(|probe| self.index.cand_cmp(probe, &cand))
            .unwrap_or_else(|e| e);
        self.items.insert(pos, cand);
    }

    fn into_hits(self) -> Vec<FileHit> {
        self.items
            .into_iter()
            .map(|c| self.index.hit_from_candidate(c))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, kind: EntryKind, size: u64, modified: i64) -> IndexEntry {
        IndexEntry {
            path: path.to_string(),
            name: file_name(path).to_string(),
            ext: {
                let n = file_name(path);
                let l = ext_len_of(n) as usize;
                if l == 0 {
                    String::new()
                } else {
                    n[n.len() - l..].to_string()
                }
            },
            kind,
            size,
            modified,
        }
    }

    #[test]
    fn path_helpers() {
        assert_eq!(parent_dir(r"C:\Users\X\a.txt"), Some(r"C:\Users\X"));
        assert_eq!(parent_dir(r"C:\a.txt"), Some(r"C:\"));
        assert_eq!(parent_dir(r"C:\"), None);
        assert_eq!(file_name(r"C:\Users\X\a.txt"), "a.txt");
        assert_eq!(ext_len_of("a.tar.gz"), 2);
        assert_eq!(ext_len_of(".gitignore"), 0);
        assert_eq!(ext_len_of("noext"), 0);
        assert_eq!(normalize_root(Path::new(r"C:\Users\X\")), r"C:\Users\X");
    }

    #[test]
    fn builder_roundtrip_and_search() {
        let entries = vec![
            entry(r"C:\docs\report.pdf", EntryKind::File, 100, 1000),
            entry(r"C:\docs\report-final.docx", EntryKind::File, 200, 2000),
            entry(r"C:\docs\notes.txt", EntryKind::File, 300, 3000),
            entry(r"C:\docs", EntryKind::Folder, 0, 4000),
        ];
        let index = FilenameIndex::from_entries(entries.clone());
        assert_eq!(index.len(), 4);
        let hits = index.search("report", 10);
        assert!(!hits.is_empty());
        assert_eq!(index.entry_name(hits[0].entry_index), "report.pdf");
        assert_eq!(index.entry_path(hits[0].entry_index), r"C:\docs\report.pdf");
        let all = index.entries();
        assert_eq!(all.len(), 4);
        assert!(all.iter().any(|e| e.path == r"C:\docs\report.pdf"));
    }

    #[test]
    fn incremental_upsert_remove() {
        let mut index = FilenameIndex::from_entries(vec![
            entry(r"C:\docs\a.txt", EntryKind::File, 1, 1),
            entry(r"C:\docs\b.txt", EntryKind::File, 2, 2),
        ]);
        assert_eq!(index.len(), 2);
        assert!(index.upsert(entry(r"C:\docs\c.txt", EntryKind::File, 3, 3)));
        assert_eq!(index.len(), 3);
        assert!(!index.upsert(entry(r"C:\docs\a.txt", EntryKind::File, 9, 9)));
        assert_eq!(index.len(), 3);
        assert_eq!(index.entry_size(0), 9);
        assert!(index.remove_path(r"C:\docs\b.txt"));
        assert_eq!(index.len(), 2);
        assert!(index.search("b.txt", 5).is_empty());
        index.compact();
        assert_eq!(index.entries().len(), 2);
    }
}
