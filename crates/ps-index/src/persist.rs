//! Versioned binary persistence for the filename index. OWNER: engine-core.
//!
//! Format (little endian):
//!   magic "PSIDXv2\0" | u32 format_version | u32 flags | i64 built_at
//!   u64 entry_count | u64 dir_count | u32 root_count | u64 arena_len | u64 sorted_len
//!   roots: root_count * (u16 len + bytes)
//!   root_counts: root_count * u64
//!   dirs: dir_count * (u32 path_off, u16 path_len, u16 reserved)
//!   entries: entry_count * (u32 name_off, u16 name_len, u8 ext_len, u8 kind,
//!                           u32 dir, u64 size, i64 modified)
//!   blooms: entry_count * u64
//!   sorted: sorted_len * u32
//!   arena: arena_len bytes
//!   u64 FNV-1a checksum of every byte before it
//!
//! Writes go to a temp file and are atomically renamed. Loading validates every
//! offset, the sorted permutation and the checksum, returning Err (never
//! panicking) for corrupt or truncated files so callers can rebuild.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::sync::OnceLock;

use crate::{DirRec, EntryRec, FilenameIndex, KIND_TOMBSTONE};

const MAGIC: [u8; 8] = *b"PSIDXv2\0";
pub const FORMAT_VERSION: u32 = 2;

const MAX_ENTRIES: u64 = 200_000_000;
const MAX_DIRS: u64 = 50_000_000;
const MAX_ARENA: u64 = 8 << 30;

fn fnv1a_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

struct HashedWriter<W: Write> {
    inner: W,
    hash: u64,
}

impl<W: Write> HashedWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            hash: FNV_OFFSET,
        }
    }

    fn bytes(&mut self, b: &[u8]) -> std::io::Result<()> {
        self.hash = fnv1a_bytes(self.hash, b);
        self.inner.write_all(b)
    }

    fn u8(&mut self, v: u8) -> std::io::Result<()> {
        self.bytes(&[v])
    }

    fn u16(&mut self, v: u16) -> std::io::Result<()> {
        self.bytes(&v.to_le_bytes())
    }

    fn u32(&mut self, v: u32) -> std::io::Result<()> {
        self.bytes(&v.to_le_bytes())
    }

    fn u64(&mut self, v: u64) -> std::io::Result<()> {
        self.bytes(&v.to_le_bytes())
    }

    fn i64(&mut self, v: i64) -> std::io::Result<()> {
        self.bytes(&v.to_le_bytes())
    }

    fn finish(mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

struct HashedReader<R: Read> {
    inner: R,
    hash: u64,
}

impl<R: Read> HashedReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            hash: FNV_OFFSET,
        }
    }

    fn bytes(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.inner.read_exact(buf)?;
        self.hash = fnv1a_bytes(self.hash, buf);
        Ok(())
    }

    fn u8(&mut self) -> std::io::Result<u8> {
        let mut b = [0u8; 1];
        self.bytes(&mut b)?;
        Ok(b[0])
    }

    fn u16(&mut self) -> std::io::Result<u16> {
        let mut b = [0u8; 2];
        self.bytes(&mut b)?;
        Ok(u16::from_le_bytes(b))
    }

    fn u32(&mut self) -> std::io::Result<u32> {
        let mut b = [0u8; 4];
        self.bytes(&mut b)?;
        Ok(u32::from_le_bytes(b))
    }

    fn u64(&mut self) -> std::io::Result<u64> {
        let mut b = [0u8; 8];
        self.bytes(&mut b)?;
        Ok(u64::from_le_bytes(b))
    }

    fn i64(&mut self) -> std::io::Result<i64> {
        let mut b = [0u8; 8];
        self.bytes(&mut b)?;
        Ok(i64::from_le_bytes(b))
    }

    fn raw_u64(&mut self) -> std::io::Result<u64> {
        let mut b = [0u8; 8];
        self.inner.read_exact(&mut b)?;
        Ok(u64::from_le_bytes(b))
    }
}

fn corrupt(msg: impl Into<String>) -> anyhow::Error {
    anyhow::anyhow!("corrupt filename index: {}", msg.into())
}

impl FilenameIndex {
    /// Persist to a binary file using an atomic temp+rename.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let tmp = path.with_extension("tmp");
        {
            let file = File::create(&tmp)?;
            let mut w = HashedWriter::new(BufWriter::with_capacity(1 << 20, file));

            // Live entry list (skip tombstones); sorted order is preserved.
            let has_dead = self.entries.len() != self.live;
            let mut remap: Vec<u32> = Vec::with_capacity(self.entries.len());
            let live: Vec<u32> = if has_dead {
                let mut next = 0u32;
                for (i, rec) in self.entries.iter().enumerate() {
                    if rec.kind == KIND_TOMBSTONE {
                        remap.push(u32::MAX);
                    } else {
                        remap.push(next);
                        next += 1;
                        // Placeholder; the real live list is built below.
                        let _ = i;
                    }
                }
                (0..self.entries.len() as u32)
                    .filter(|&i| self.entries[i as usize].kind != KIND_TOMBSTONE)
                    .collect()
            } else {
                (0..self.entries.len() as u32).collect()
            };
            let sorted: Vec<u32> = if has_dead {
                self.sorted
                    .iter()
                    .copied()
                    .filter(|&i| self.entries[i as usize].kind != KIND_TOMBSTONE)
                    .map(|i| remap[i as usize])
                    .collect()
            } else {
                self.sorted.clone()
            };
            let entry_count = live.len() as u64;
            let arena_len = self.arena.len() as u64;

            w.bytes(&MAGIC)?;
            w.u32(FORMAT_VERSION)?;
            w.u32(0)?;
            w.i64(self.built_at)?;
            w.u64(entry_count)?;
            w.u64(self.dirs.len() as u64)?;
            w.u32(self.roots.len() as u32)?;
            w.u64(arena_len)?;
            w.u64(sorted.len() as u64)?;
            for root in &self.roots {
                let bytes = root.as_bytes();
                let len = bytes.len().min(u16::MAX as usize);
                w.u16(len as u16)?;
                w.bytes(&bytes[..len])?;
            }
            for i in 0..self.roots.len() {
                w.u64(self.root_counts.get(i).copied().unwrap_or(0))?;
            }
            for d in &self.dirs {
                w.u32(d.path_off)?;
                w.u16(d.path_len)?;
                w.u16(d.reserved)?;
            }
            for &b in &self.dir_blooms {
                w.u64(b)?;
            }
            for &i in &live {
                let r = &self.entries[i as usize];
                w.u32(r.name_off)?;
                w.u16(r.name_len)?;
                w.u8(r.ext_len)?;
                w.u8(r.kind)?;
                w.u32(r.dir)?;
                w.u64(r.size)?;
                w.i64(r.modified)?;
            }
            for &i in &live {
                w.u64(self.blooms[i as usize])?;
            }
            for &i in &sorted {
                w.u32(i)?;
            }
            w.bytes(&self.arena)?;
            let checksum = w.hash;
            w.inner.write_all(&checksum.to_le_bytes())?;
            w.finish()?;
        }
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Load a persisted index. Returns Err for corrupt/truncated/incompatible
    /// files; callers should rebuild in that case.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let file = File::open(path)?;
        let mut r = HashedReader::new(BufReader::with_capacity(1 << 20, file));

        let mut magic = [0u8; 8];
        r.bytes(&mut magic)?;
        if magic != MAGIC {
            return Err(corrupt("bad magic"));
        }
        let version = r.u32()?;
        if version != FORMAT_VERSION {
            return Err(corrupt(format!(
                "unsupported format version {version} (expected {FORMAT_VERSION})"
            )));
        }
        let flags = r.u32()?;
        if flags != 0 {
            return Err(corrupt(format!("unsupported flags {flags}")));
        }
        let built_at = r.i64()?;
        let entry_count = r.u64()?;
        let dir_count = r.u64()?;
        let root_count = r.u32()?;
        let arena_len = r.u64()?;
        let sorted_len = r.u64()?;
        if entry_count > MAX_ENTRIES {
            return Err(corrupt("entry count out of range"));
        }
        if dir_count > MAX_DIRS {
            return Err(corrupt("dir count out of range"));
        }
        if arena_len > MAX_ARENA {
            return Err(corrupt("arena length out of range"));
        }
        if sorted_len != entry_count {
            return Err(corrupt("sorted length mismatch"));
        }
        if root_count > 4096 {
            return Err(corrupt("root count out of range"));
        }
        if entry_count > 0 && dir_count == 0 {
            return Err(corrupt("entries present but no directories"));
        }

        let mut roots: Vec<String> = Vec::with_capacity(root_count as usize);
        for _ in 0..root_count {
            let len = r.u16()? as usize;
            let mut buf = vec![0u8; len];
            r.bytes(&mut buf)?;
            let s = String::from_utf8(buf).map_err(|_| corrupt("root not utf-8"))?;
            roots.push(s);
        }
        let mut root_counts: Vec<u64> = Vec::with_capacity(root_count as usize);
        for _ in 0..root_count {
            root_counts.push(r.u64()?);
        }

        let mut dirs: Vec<DirRec> = Vec::with_capacity(dir_count as usize);
        for _ in 0..dir_count {
            let path_off = r.u32()?;
            let path_len = r.u16()?;
            let reserved = r.u16()?;
            if (path_off as u64 + path_len as u64) > arena_len {
                return Err(corrupt("dir path out of arena"));
            }
            dirs.push(DirRec {
                path_off,
                path_len,
                reserved,
            });
        }

        let mut dir_blooms: Vec<u64> = Vec::with_capacity(dir_count as usize);
        for _ in 0..dir_count {
            dir_blooms.push(r.u64()?);
        }

        let mut entries: Vec<EntryRec> = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            let name_off = r.u32()?;
            let name_len = r.u16()?;
            let ext_len = r.u8()?;
            let kind = r.u8()?;
            let dir = r.u32()?;
            let size = r.u64()?;
            let modified = r.i64()?;
            if (name_off as u64 + name_len as u64) > arena_len {
                return Err(corrupt("entry name out of arena"));
            }
            if ext_len as u16 >= name_len && name_len > 0 {
                return Err(corrupt("entry extension longer than name"));
            }
            if kind > 1 {
                return Err(corrupt("invalid entry kind"));
            }
            if dir as u64 >= dir_count {
                return Err(corrupt("entry dir out of range"));
            }
            entries.push(EntryRec {
                name_off,
                name_len,
                ext_len,
                kind,
                dir,
                size,
                modified,
            });
        }

        let mut blooms: Vec<u64> = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            blooms.push(r.u64()?);
        }

        let mut sorted: Vec<u32> = Vec::with_capacity(sorted_len as usize);
        {
            let mut seen = vec![false; entry_count as usize];
            for _ in 0..sorted_len {
                let idx = r.u32()?;
                if idx as u64 >= entry_count {
                    return Err(corrupt("sorted index out of range"));
                }
                if seen[idx as usize] {
                    return Err(corrupt("duplicate index in sorted array"));
                }
                seen[idx as usize] = true;
                sorted.push(idx);
            }
        }

        let mut arena = vec![0u8; arena_len as usize];
        r.bytes(&mut arena)?;

        let computed = r.hash;
        let stored = r.raw_u64()?;
        if computed != stored {
            return Err(corrupt("checksum mismatch"));
        }
        // Reject trailing garbage.
        let mut trailing = [0u8; 1];
        if r.inner.read(&mut trailing)? != 0 {
            return Err(corrupt("trailing bytes"));
        }

        // Validate every string is UTF-8 (the runtime relies on it).
        for d in &dirs {
            let start = d.path_off as usize;
            let end = start + d.path_len as usize;
            if std::str::from_utf8(&arena[start..end]).is_err() {
                return Err(corrupt("dir path not utf-8"));
            }
        }
        for e in &entries {
            let start = e.name_off as usize;
            let end = start + e.name_len as usize;
            if std::str::from_utf8(&arena[start..end]).is_err() {
                return Err(corrupt("entry name not utf-8"));
            }
        }

        Ok(FilenameIndex {
            arena,
            entries,
            blooms,
            dirs,
            dir_blooms,
            sorted,
            roots,
            root_counts,
            built_at,
            live: entry_count as usize,
            cache: OnceLock::new(),
            dir_map: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EntryKind, IndexEntry};
    use std::io::Write;

    fn temp_path(tag: &str) -> std::path::PathBuf {
        crate::testutil::unique(tag).with_extension("psidx")
    }

    fn sample_entries() -> Vec<IndexEntry> {
        vec![
            IndexEntry {
                path: r"C:\Users\X\Documents\report.pdf".into(),
                name: "report.pdf".into(),
                ext: "pdf".into(),
                kind: EntryKind::File,
                size: 1024,
                modified: 1_700_000_000_000,
            },
            IndexEntry {
                path: r"C:\Users\X\Documents\notes.txt".into(),
                name: "notes.txt".into(),
                ext: "txt".into(),
                kind: EntryKind::File,
                size: 42,
                modified: 1_700_000_100_000,
            },
            IndexEntry {
                path: r"C:\Users\X\Documents".into(),
                name: "Documents".into(),
                ext: "".into(),
                kind: EntryKind::Folder,
                size: 0,
                modified: 1_700_000_200_000,
            },
            IndexEntry {
                path: "C:\\Users\\X\\Documents\\报告-最终版.docx".into(),
                name: "报告-最终版.docx".into(),
                ext: "docx".into(),
                kind: EntryKind::File,
                size: 7,
                modified: 1_700_000_300_000,
            },
        ]
    }

    #[test]
    fn roundtrip_preserves_entries_roots_and_search() {
        let path = temp_path("roundtrip");
        let mut index = FilenameIndex::from_entries(sample_entries());
        index.roots.push(r"C:\Users\X".to_string());
        index.root_counts.push(4);
        index.save(&path).unwrap();
        let loaded = FilenameIndex::load(&path).unwrap();
        assert_eq!(loaded.len(), index.len());
        assert_eq!(loaded.roots(), index.roots());
        assert_eq!(loaded.built_at(), index.built_at());
        assert_eq!(loaded.entry_path(0), index.entry_path(0));
        let a = index.search("report", 10);
        let b = loaded.search("report", 10);
        assert_eq!(a, b);
        let cjk = loaded.search("最终版", 10);
        assert!(!cjk.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn corrupt_and_truncated_files_are_rejected() {
        let path = temp_path("corrupt");
        // Random garbage.
        std::fs::write(&path, b"not an index at all, definitely not").unwrap();
        assert!(FilenameIndex::load(&path).is_err());
        // Valid file, truncated at every interesting boundary.
        let index = FilenameIndex::from_entries(sample_entries());
        index.save(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        for cut in [0usize, 1, 7, 8, 12, 20, 40, bytes.len() / 2, bytes.len() - 1] {
            let mut f = File::create(&path).unwrap();
            f.write_all(&bytes[..cut.min(bytes.len())]).unwrap();
            drop(f);
            assert!(FilenameIndex::load(&path).is_err(), "truncated at {cut} must fail");
        }
        // Flip a byte in the arena: checksum must catch it.
        let mut flipped = bytes.clone();
        let n = flipped.len();
        flipped[n - 20] ^= 0xFF;
        std::fs::write(&path, &flipped).unwrap();
        assert!(FilenameIndex::load(&path).is_err());
        // Wrong magic.
        let mut wrong = bytes.clone();
        wrong[0] = b'X';
        std::fs::write(&path, &wrong).unwrap();
        assert!(FilenameIndex::load(&path).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_skips_tombstones_and_reload_is_consistent() {
        let path = temp_path("tombstone");
        let mut index = FilenameIndex::from_entries(sample_entries());
        assert!(index.remove_path(r"C:\Users\X\Documents\notes.txt"));
        assert_eq!(index.len(), 3);
        index.save(&path).unwrap();
        let loaded = FilenameIndex::load(&path).unwrap();
        assert_eq!(loaded.len(), 3);
        assert!(loaded.search("notes", 10).is_empty());
        let _ = std::fs::remove_file(&path);
    }
}