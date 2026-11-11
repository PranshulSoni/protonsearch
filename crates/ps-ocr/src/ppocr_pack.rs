//! Explicit, verified installation of the PP-OCRv6 tiny model pack.
//! This feature is opt-in; no model or runtime binary is downloaded implicitly.
use anyhow::{anyhow, bail, Context};
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

const RELEASE: &str = "https://github.com/GreatV/oar-ocr/releases/download/v0.7.0/";
const MAX_REDIRECTS: usize = 5;
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
struct Asset {
    name: &'static str,
    sha256: &'static str,
    max_bytes: u64,
}

const ASSETS: &[Asset] = &[
    Asset {
        name: "pp-ocrv6_tiny_det.onnx",
        sha256: "193bab7a04fca699a6c82e6abb5b81bdb28177f0abd4062552b04908dafb19f8",
        max_bytes: 3 * 1024 * 1024,
    },
    Asset {
        name: "pp-ocrv6_tiny_rec.onnx",
        sha256: "9ef676d6ed3c88256a2d92c640c44f25b0c40947e111b14b8be8f594091563e6",
        max_bytes: 6 * 1024 * 1024,
    },
    Asset {
        name: "ppocrv6_tiny_dict.txt",
        sha256: "c5cbe34ef40c29c4df07ed012bf96569cb69a2d2a01a07027e9f13cb832bd9cd",
        max_bytes: 1024 * 1024,
    },
];

/// Downloads and installs the pinned model pack after the caller explicitly requests it.
/// `model_dir` must name the final directory and its parent must already exist.
/// The backup-and-rename activation handles ordinary errors, but is not crash-safe.
pub fn install_pp_ocrv6_tiny(model_dir: impl AsRef<Path>) -> anyhow::Result<()> {
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= MAX_REDIRECTS || !safe_url(attempt.url()) {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }))
        .build()
        .context("creating PP-OCRv6 download client")?;
    install_with(model_dir.as_ref(), ASSETS, |asset, out| {
        let url = format!("{RELEASE}{}", asset.name);
        let mut response = client.get(url).send()?.error_for_status()?;
        if !safe_url(response.url()) {
            bail!("model download redirected to an untrusted URL");
        }
        if response
            .content_length()
            .is_some_and(|size| size > asset.max_bytes)
        {
            bail!("model asset exceeds its size limit");
        }
        let mut limited = (&mut response).take(asset.max_bytes + 1);
        std::io::copy(&mut limited, out)?;
        Ok(())
    })
}

fn safe_url(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && matches!(
            url.host_str(),
            Some("github.com" | "release-assets.githubusercontent.com")
        )
}

/// Recovers any model pack left in an inconsistent state by an interrupted process.
/// If `model_dir` is missing and a `.backup` directory exists in the parent, restores it.
/// Also cleans up any leftover staging directories.
pub fn recover_model_dir(model_dir: &Path) -> anyhow::Result<()> {
    let name = match model_dir.file_name() {
        Some(n) => n,
        None => return Ok(()),
    };
    let parent = model_dir.parent().unwrap_or(Path::new("."));
    if !parent.is_dir() {
        return Ok(());
    }
    let prefix = format!(".{}.", name.to_string_lossy());
    if !model_dir.exists() {
        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();
                if file_name_str.starts_with(&prefix) && file_name_str.ends_with(".backup") {
                    let _ = fs::rename(entry.path(), model_dir);
                    break;
                }
            }
        }
    }
    if let Ok(entries) = fs::read_dir(parent) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            if file_name_str.starts_with(&prefix) && file_name_str.contains(".ppocr-staging.") {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
    Ok(())
}

fn install_with(
    model_dir: &Path,
    assets: &[Asset],
    mut download: impl FnMut(&Asset, &mut File) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let _ = recover_model_dir(model_dir);
    let name = model_dir
        .file_name()
        .ok_or_else(|| anyhow!("model directory must have a final path component"))?;
    let parent = model_dir
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = fs::canonicalize(parent).context("resolving model directory parent")?;
    let target = parent.join(name);
    match fs::symlink_metadata(&target) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
            bail!("model directory cannot be a symlink or non-directory")
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let suffix = format!("{}.{}.{id}", std::process::id(), "ppocr-staging");
    let stage = parent.join(format!(".{}.{}", name.to_string_lossy(), suffix));
    let backup = parent.join(format!(".{}.{}.backup", name.to_string_lossy(), id));
    fs::create_dir(&stage).context("creating model staging directory")?;
    let result = (|| {
        for asset in assets {
            let path = stage.join(asset.name);
            let mut file = File::create(&path)?;
            download(asset, &mut file).with_context(|| format!("downloading {}", asset.name))?;
            file.flush()?;
            let size = file.metadata()?.len();
            if size == 0 || size > asset.max_bytes {
                bail!("model asset {} has invalid size {size}", asset.name);
            }
            drop(file);
            let digest = hash_file(&path)?;
            if digest != asset.sha256 {
                bail!("SHA-256 mismatch for {}", asset.name);
            }
        }
        let had_target = target.exists();
        if had_target {
            fs::rename(&target, &backup).context("preserving existing model pack")?;
        }
        if let Err(error) = fs::rename(&stage, &target) {
            if had_target {
                fs::rename(&backup, &target).context("restoring previous model pack")?;
            }
            return Err(error).context("activating downloaded model pack");
        }
        if had_target {
            let _ = fs::remove_dir_all(backup);
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(stage);
    }
    result
}

fn hash_file(path: &Path) -> anyhow::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_bad_hash_and_preserves_existing_pack() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("models");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("keep"), b"old").unwrap();
        let assets = [Asset {
            name: "model.bin",
            sha256: "00",
            max_bytes: 8,
        }];
        let result = install_with(&target, &assets, |_, out| {
            out.write_all(b"new")?;
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(fs::read(target.join("keep")).unwrap(), b"old");
        assert!(!target.join("model.bin").exists());
    }

    #[test]
    fn installs_only_hash_verified_files() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("models");
        let assets = [Asset {
            name: "model.bin",
            sha256: "11507a0e2f5e69d5dfa40a62a1bd7b6ee57e6bcd85c67c9b8431b36fff21c437",
            max_bytes: 8,
        }];
        install_with(&target, &assets, |_, out| {
            out.write_all(b"new")?;
            Ok(())
        })
        .unwrap();
        assert_eq!(fs::read(target.join("model.bin")).unwrap(), b"new");
    }

    #[test]
    fn crash_recovery_restores_orphaned_backup() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("models");
        let backup = root.path().join(".models.999.backup");
        fs::create_dir_all(&backup).unwrap();
        fs::write(backup.join("restored.bin"), b"persisted").unwrap();

        recover_model_dir(&target).unwrap();
        assert!(target.exists());
        assert_eq!(fs::read(target.join("restored.bin")).unwrap(), b"persisted");
        assert!(!backup.exists());
    }

    #[test]
    fn accepts_only_https_github_asset_hosts() {
        assert!(safe_url(&reqwest::Url::parse("https://github.com/file").unwrap()));
        assert!(safe_url(
            &reqwest::Url::parse("https://release-assets.githubusercontent.com/file").unwrap()
        ));
        assert!(!safe_url(&reqwest::Url::parse("http://github.com/file").unwrap()));
        assert!(!safe_url(
            &reqwest::Url::parse("https://example.com/file").unwrap()
        ));
    }
}
