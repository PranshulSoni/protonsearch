//! Secure, package-aware Linux update support.
//!
//! The settings process only discovers and verifies releases. Installation is
//! delegated to a detached helper so the running application never overwrites
//! its own executable and user data remains in XDG state/config directories.

use crate::platform::{self, DistributionFamily};
use crate::system;
use crate::xdg::XdgPaths;
use anyhow::{Context, Result};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const RELEASES_API: &str = "https://api.github.com/repos/PranshulSoni/protonsearch/releases/latest";
const USER_AGENT: &str = "ProtonSearch-Linux/1.0.0";
const CHECK_INTERVAL: u64 = 24 * 60 * 60;
const MAX_RELEASE_NOTES: usize = 64 * 1024;
const MAX_DOWNLOAD_BYTES: u64 = 2 * 1024 * 1024 * 1024;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallationKind {
    Pacman,
    Apt,
    Dnf,
    AppImage,
    Flatpak,
    Manual,
    Development,
    Unknown,
}

impl InstallationKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pacman => "Arch package (pacman)",
            Self::Apt => "Debian package (APT)",
            Self::Dnf => "Fedora package (DNF)",
            Self::AppImage => "AppImage",
            Self::Flatpak => "Flatpak",
            Self::Manual => "Manual installation",
            Self::Development => "Development build",
            Self::Unknown => "Unknown installation",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformTarget {
    pub distribution: String,
    pub family: String,
    pub architecture: String,
    pub installation: InstallationKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateSnapshot {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub release_name: Option<String>,
    pub release_notes: Option<String>,
    pub release_date: Option<String>,
    pub release_url: Option<String>,
    pub asset_name: Option<String>,
    pub asset_url: Option<String>,
    pub asset_size: Option<u64>,
    pub sha256: Option<String>,
    pub package: Option<String>,
    pub target: Option<PlatformTarget>,
    pub checked_at: Option<u64>,
    pub update_available: bool,
    pub installable: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateJob {
    pub downloaded_path: PathBuf,
    pub expected_version: String,
    pub asset_name: String,
    pub installation: InstallationKind,
    pub service_was_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallStage {
    Preparing,
    Stopping,
    Installing,
    Validating,
    Restarting,
    Complete,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallState {
    pub stage: InstallStage,
    pub message: String,
    pub updated_at: u64,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    published_at: Option<String>,
    html_url: Option<String>,
    draft: bool,
    prerelease: bool,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[derive(Debug, Clone)]
pub struct DownloadedUpdate {
    pub path: PathBuf,
    pub snapshot: UpdateSnapshot,
}

pub fn current_version() -> &'static str {
    CURRENT_VERSION
}

pub fn state_path(paths: &XdgPaths) -> PathBuf {
    paths.state_dir().join("updates/state.json")
}

pub fn job_path(paths: &XdgPaths) -> PathBuf {
    paths.state_dir().join("updates/job.json")
}

pub fn install_state_path(paths: &XdgPaths) -> PathBuf {
    paths.state_dir().join("updates/install-state.json")
}

pub fn target() -> PlatformTarget {
    let info = platform::detect();
    PlatformTarget {
        distribution: info.distribution.id,
        family: match info.distribution.family {
            DistributionFamily::Arch => "arch",
            DistributionFamily::Debian => "debian",
            DistributionFamily::Fedora => "fedora",
            DistributionFamily::Other => "other",
        }
        .to_string(),
        architecture: normalized_architecture(),
        installation: detect_installation(),
    }
}

pub fn load(paths: &XdgPaths) -> UpdateSnapshot {
    let mut snapshot = fs::read(state_path(paths))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<UpdateSnapshot>(&bytes).ok())
        .unwrap_or_default();
    if !snapshot.current_version.is_empty() && snapshot.current_version != CURRENT_VERSION {
        snapshot.latest_version = None;
        snapshot.release_name = None;
        snapshot.release_notes = None;
        snapshot.release_date = None;
        snapshot.release_url = None;
        snapshot.asset_name = None;
        snapshot.asset_url = None;
        snapshot.asset_size = None;
        snapshot.sha256 = None;
        snapshot.package = None;
        snapshot.checked_at = None;
        snapshot.update_available = false;
        snapshot.installable = false;
        snapshot.message.clear();
    }
    snapshot.current_version = CURRENT_VERSION.to_string();
    if snapshot.target.is_none() {
        snapshot.target = Some(target());
    }
    snapshot
}

pub fn load_install_state(paths: &XdgPaths) -> Option<InstallState> {
    fs::read(install_state_path(paths))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

pub fn check(paths: &XdgPaths, force: bool) -> Result<UpdateSnapshot> {
    let cached = load(paths);
    let now = now_seconds();
    if !force
        && cached.latest_version.is_some()
        && cached
            .checked_at
            .is_some_and(|checked| now.saturating_sub(checked) < CHECK_INTERVAL)
    {
        return Ok(cached);
    }

    let target = target();
    let release = fetch_release()?;
    if release.draft || release.prerelease {
        anyhow::bail!("the latest GitHub release is not a stable release")
    }
    let latest = parse_version(&release.tag_name)
        .with_context(|| format!("invalid GitHub release version {}", release.tag_name))?;
    let current = Version::parse(CURRENT_VERSION)?;
    let selected = select_asset(&release.assets, &target)
        .filter(|asset| is_official_github_url(&asset.browser_download_url));
    let checksum = selected
        .as_ref()
        .and_then(|asset| checksum_for_asset(&release.assets, asset).ok().flatten());
    let update_available = latest > current;
    let installable =
        update_available && selected.is_some() && checksum.as_deref().is_some_and(is_sha256);
    let message = if !update_available {
        format!("ProtonSearch is up to date ({CURRENT_VERSION})")
    } else if selected.is_none() {
        "A newer release exists, but no compatible Linux package was published.".to_string()
    } else if checksum.is_none() {
        "A compatible package exists, but its SHA-256 checksum is missing.".to_string()
    } else {
        format!("ProtonSearch {latest} is available")
    };
    let mut snapshot = UpdateSnapshot {
        current_version: CURRENT_VERSION.to_string(),
        latest_version: Some(latest.to_string()),
        release_name: release.name,
        release_notes: Some(
            release
                .body
                .unwrap_or_default()
                .chars()
                .take(MAX_RELEASE_NOTES)
                .collect(),
        ),
        release_date: release.published_at,
        release_url: release.html_url.filter(|url| is_official_github_url(url)),
        asset_name: selected.as_ref().map(|asset| asset.name.clone()),
        asset_url: selected
            .as_ref()
            .map(|asset| asset.browser_download_url.clone()),
        asset_size: selected.as_ref().map(|asset| asset.size),
        sha256: checksum,
        package: selected.as_ref().map(|asset| asset.kind.clone()),
        target: Some(target),
        checked_at: Some(now),
        update_available,
        installable,
        message,
    };
    write_snapshot(paths, &snapshot)?;
    snapshot.current_version = CURRENT_VERSION.to_string();
    Ok(snapshot)
}

pub fn download_and_verify<F>(
    paths: &XdgPaths,
    snapshot: &UpdateSnapshot,
    mut progress: F,
) -> Result<DownloadedUpdate>
where
    F: FnMut(u64, Option<u64>),
{
    if !snapshot.installable {
        anyhow::bail!(
            "this release is not safely installable: {}",
            snapshot.message
        );
    }
    let url = snapshot
        .asset_url
        .as_deref()
        .context("release asset URL missing")?;
    if !is_official_github_url(url) {
        anyhow::bail!("release asset URL is outside the official ProtonSearch repository");
    }
    let expected_hash = snapshot
        .sha256
        .as_deref()
        .context("release SHA-256 missing")?;
    let expected_size = snapshot.asset_size.context("release asset size missing")?;
    if expected_size > MAX_DOWNLOAD_BYTES {
        anyhow::bail!("release asset is larger than the safety limit");
    }
    let name = snapshot
        .asset_name
        .as_deref()
        .context("release asset name missing")?;
    let downloads = paths.state_dir().join("updates/downloads");
    fs::create_dir_all(&downloads)?;
    let safe_name = Path::new(name)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty() && *value != "." && *value != "..")
        .context("release asset name is unsafe")?;
    let partial = downloads.join(format!("{safe_name}.partial"));
    let final_path = downloads.join(safe_name);
    let _ = fs::remove_file(&partial);

    let agent = http_agent()?;
    let mut response = agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/octet-stream")
        .call()
        .context("downloading release asset")?;
    let remote_size = response.body().content_length();
    if remote_size.is_some_and(|size| size != expected_size) {
        anyhow::bail!("downloaded file size does not match release metadata");
    }
    let mut reader = response.body_mut().as_reader();
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&partial)
        .with_context(|| format!("creating {}", partial.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    let mut total = 0_u64;
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total = total.saturating_add(count as u64);
        if total > expected_size || total > MAX_DOWNLOAD_BYTES {
            let _ = fs::remove_file(&partial);
            anyhow::bail!("download exceeded the expected size");
        }
        hasher.update(&buffer[..count]);
        file.write_all(&buffer[..count])?;
        progress(total, Some(expected_size));
    }
    file.sync_all()?;
    if total != expected_size {
        let _ = fs::remove_file(&partial);
        anyhow::bail!("download ended before the expected file size");
    }
    let actual = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if !actual.eq_ignore_ascii_case(expected_hash) {
        let _ = fs::remove_file(&partial);
        anyhow::bail!("SHA-256 verification failed");
    }
    fs::rename(&partial, &final_path)?;
    Ok(DownloadedUpdate {
        path: final_path,
        snapshot: snapshot.clone(),
    })
}

pub fn launch_install_helper(paths: &XdgPaths, downloaded: &DownloadedUpdate) -> Result<()> {
    let service_was_active = system::run(
        "systemctl",
        &["--user", "is-active", "--quiet", "protonsearch.service"],
    )
    .map(|result| result.status == Some(0))
    .unwrap_or(false);
    let job = UpdateJob {
        downloaded_path: downloaded.path.clone(),
        expected_version: downloaded
            .snapshot
            .latest_version
            .clone()
            .context("latest version missing")?,
        asset_name: downloaded
            .snapshot
            .asset_name
            .clone()
            .context("asset name missing")?,
        installation: downloaded
            .snapshot
            .target
            .as_ref()
            .map(|target| target.installation.clone())
            .unwrap_or(InstallationKind::Unknown),
        service_was_active,
    };
    write_json_atomic(&job_path(paths), &job)?;
    let executable = std::env::current_exe()?;
    Command::new(&executable)
        .args(["update-helper", job_path(paths).to_string_lossy().as_ref()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("starting detached ProtonSearch update helper")?;
    Ok(())
}

pub fn run_helper(paths: &XdgPaths, job_file: &Path) -> Result<()> {
    let job: UpdateJob = serde_json::from_slice(&fs::read(job_file)?)?;
    write_install_state(
        paths,
        InstallStage::Preparing,
        "Preparing the verified update",
    )?;
    let old_executable = std::env::current_exe()?;
    write_install_state(
        paths,
        InstallStage::Stopping,
        "Stopping ProtonSearch components",
    )?;
    if job.service_was_active {
        let _ = system::run("systemctl", &["--user", "stop", "protonsearch.service"]);
    }
    stop_owned_processes(&old_executable);
    write_install_state(
        paths,
        InstallStage::Installing,
        "Installing the verified package",
    )?;
    let install_result = match job.installation {
        InstallationKind::Pacman => {
            install_package("pacman", &["-U", "--noconfirm"], &job.downloaded_path)
        }
        InstallationKind::Apt => {
            install_package("apt-get", &["install", "-y"], &job.downloaded_path)
        }
        InstallationKind::Dnf => install_package("dnf", &["install", "-y"], &job.downloaded_path),
        InstallationKind::AppImage | InstallationKind::Manual => {
            atomic_replace(&old_executable, &job.downloaded_path)
        }
        InstallationKind::Flatpak => anyhow::bail!("Flatpak updates must be performed by Flatpak"),
        InstallationKind::Development | InstallationKind::Unknown => {
            anyhow::bail!("this installation is not managed by ProtonSearch")
        }
    };
    if let Err(error) = install_result {
        if matches!(
            job.installation,
            InstallationKind::AppImage | InstallationKind::Manual
        ) {
            let _ = rollback_replace(&old_executable);
        }
        restart_service(&job);
        write_install_state(
            paths,
            InstallStage::Failed,
            &format!("Installation failed: {error}"),
        )?;
        return Err(error);
    }
    write_install_state(
        paths,
        InstallStage::Validating,
        "Validating the installed executable",
    )?;
    let installed = std::env::current_exe()?;
    let validation = match system::run_with_timeout(
        installed.to_string_lossy().as_ref(),
        &["version"],
        Duration::from_secs(10),
    ) {
        Ok(validation) => validation,
        Err(error) => {
            return fail_install(
                paths,
                &job,
                &old_executable,
                error,
                "The installed executable could not be started",
            )
        }
    };
    let reported_version = validation.stdout.trim().trim_start_matches('v');
    if validation.status != Some(0)
        || validation.timed_out
        || reported_version != job.expected_version
    {
        return fail_install(
            paths,
            &job,
            &old_executable,
            anyhow::anyhow!(
                "installed ProtonSearch failed its health check (reported version: {})",
                if reported_version.is_empty() {
                    "none"
                } else {
                    reported_version
                }
            ),
            "The installed executable failed validation",
        );
    }
    write_install_state(paths, InstallStage::Restarting, "Restarting ProtonSearch")?;
    let _ = system::run("systemctl", &["--user", "daemon-reload"]);
    if job.service_was_active {
        let result = match system::run("systemctl", &["--user", "start", "protonsearch.service"]) {
            Ok(result) => result,
            Err(error) => {
                return fail_install(
                    paths,
                    &job,
                    &old_executable,
                    error,
                    "The ProtonSearch service could not be restarted",
                )
            }
        };
        if result.status != Some(0) {
            return fail_install(
                paths,
                &job,
                &old_executable,
                anyhow::anyhow!("ProtonSearch service failed to restart"),
                "The ProtonSearch service failed to restart",
            );
        }
    }
    if matches!(
        job.installation,
        InstallationKind::AppImage | InstallationKind::Manual
    ) {
        let _ = remove_backup(&old_executable);
    }
    let _ = fs::remove_file(job_file);
    write_install_state(
        paths,
        InstallStage::Complete,
        &format!("Updated to ProtonSearch {}", job.expected_version),
    )?;
    Ok(())
}

fn install_package(manager: &str, prefix: &[&str], path: &Path) -> Result<()> {
    let mut args = prefix
        .iter()
        .map(|arg| (*arg).to_string())
        .collect::<Vec<_>>();
    args.push(path.to_string_lossy().into_owned());
    let mut command = vec![manager.to_string()];
    command.extend(args);
    let (program, args) = if system::command_available("pkexec") {
        let mut elevated = vec![manager.to_string()];
        elevated.extend(command.iter().skip(1).cloned());
        ("pkexec".to_string(), elevated)
    } else if system::command_available("sudo") {
        ("sudo".to_string(), command)
    } else {
        anyhow::bail!("pkexec or sudo is required for package installation")
    };
    let references = args.iter().map(String::as_str).collect::<Vec<_>>();
    let result = system::run_with_timeout(&program, &references, Duration::from_secs(300))?;
    if result.timed_out || result.status != Some(0) {
        anyhow::bail!("{manager} failed to install the package")
    }
    Ok(())
}

fn atomic_replace(target: &Path, source: &Path) -> Result<()> {
    if target.is_symlink() {
        anyhow::bail!("refusing to replace a symlink installation")
    }
    let parent = target
        .parent()
        .context("installed executable has no parent")?;
    let temporary = parent.join(format!(".protonsearch-update-{}", std::process::id()));
    let backup = parent.join(format!(".protonsearch-backup-{}", std::process::id()));
    fs::copy(source, &temporary)?;
    let permissions = fs::metadata(target)?.permissions();
    fs::set_permissions(&temporary, permissions)?;
    fs::rename(target, &backup)?;
    if let Err(error) = fs::rename(&temporary, target) {
        let _ = fs::rename(&backup, target);
        return Err(error.into());
    }
    Ok(())
}

fn backup_path(target: &Path) -> Result<PathBuf> {
    let parent = target
        .parent()
        .context("installed executable has no parent")?;
    Ok(parent.join(format!(".protonsearch-backup-{}", std::process::id())))
}

fn remove_backup(target: &Path) -> Result<()> {
    let backup = backup_path(target)?;
    if backup.exists() {
        fs::remove_file(backup)?;
    }
    Ok(())
}

fn rollback_replace(target: &Path) -> Result<()> {
    let backup = backup_path(target)?;
    if !backup.exists() {
        return Ok(());
    }
    let failed = target.with_extension("failed-update");
    let _ = fs::remove_file(&failed);
    fs::rename(target, &failed)?;
    if let Err(error) = fs::rename(&backup, target) {
        let _ = fs::rename(&failed, target);
        return Err(error.into());
    }
    let _ = fs::remove_file(failed);
    Ok(())
}

fn restart_service(job: &UpdateJob) {
    if job.service_was_active {
        let _ = system::run("systemctl", &["--user", "daemon-reload"]);
        let _ = system::run("systemctl", &["--user", "start", "protonsearch.service"]);
    }
}

fn fail_install(
    paths: &XdgPaths,
    job: &UpdateJob,
    old_executable: &Path,
    error: anyhow::Error,
    message: &str,
) -> Result<()> {
    if matches!(
        job.installation,
        InstallationKind::AppImage | InstallationKind::Manual
    ) {
        let _ = rollback_replace(old_executable);
    }
    restart_service(job);
    write_install_state(paths, InstallStage::Failed, message)?;
    Err(error)
}

fn stop_owned_processes(executable: &Path) {
    let Ok(executable) = fs::canonicalize(executable) else {
        return;
    };
    let self_pid = std::process::id() as i32;
    let mut pids = Vec::new();
    let Ok(entries) = fs::read_dir("/proc") else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Ok(pid) = name.to_string_lossy().parse::<i32>() else {
            continue;
        };
        if pid == self_pid {
            continue;
        }
        let Ok(path) = fs::canonicalize(entry.path().join("exe")) else {
            continue;
        };
        if path == executable {
            unsafe {
                libc::kill(pid, libc::SIGTERM);
            }
            pids.push(pid);
        }
    }
    for _ in 0..50 {
        if pids
            .iter()
            .all(|pid| !PathBuf::from(format!("/proc/{pid}")).exists())
        {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    for pid in pids {
        if PathBuf::from(format!("/proc/{pid}")).exists() {
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }
    }
}

fn fetch_release() -> Result<GitHubRelease> {
    let agent = http_agent()?;
    let mut response = agent
        .get(RELEASES_API)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .call()
        .context("requesting GitHub release metadata")?;
    Ok(response.body_mut().read_json()?)
}

fn checksum_for_asset(assets: &[GitHubAsset], asset: &SelectedAsset) -> Result<Option<String>> {
    let Some(manifest) = assets.iter().find(|candidate| {
        let name = candidate.name.to_ascii_lowercase();
        name.contains("sha256") || name.contains("checksum")
    }) else {
        return Ok(None);
    };
    if !is_official_github_url(&manifest.browser_download_url) {
        return Ok(None);
    }
    let agent = http_agent()?;
    let mut response = agent
        .get(&manifest.browser_download_url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "text/plain")
        .call()?;
    let contents = response.body_mut().read_to_string()?;
    for line in contents.lines() {
        if !line.contains(&asset.name) {
            continue;
        }
        if let Some(hash) = line.split_whitespace().find(|part| is_sha256(part)) {
            return Ok(Some(hash.to_string()));
        }
    }
    Ok(None)
}

fn select_asset<'a>(assets: &'a [GitHubAsset], target: &PlatformTarget) -> Option<SelectedAsset> {
    let arch = target.architecture.to_ascii_lowercase();
    let names = assets.iter().filter(|asset| {
        let name = asset.name.to_ascii_lowercase();
        !name.contains("sha256")
            && !name.contains("checksum")
            && !name.ends_with(".sig")
            && is_official_github_url(&asset.browser_download_url)
    });
    let wanted = match target.installation {
        InstallationKind::Pacman => ("pkg.tar", package_arches(&arch, "x86_64")),
        InstallationKind::Apt => {
            let package_arch = debian_arch(&arch);
            (".deb", package_arches(package_arch, "amd64"))
        }
        InstallationKind::Dnf => {
            let package_arch = rpm_arch(&arch);
            (".rpm", package_arches(package_arch, "x86_64"))
        }
        InstallationKind::AppImage | InstallationKind::Manual => {
            ("appimage", package_arches(&arch, "x86_64"))
        }
        InstallationKind::Flatpak | InstallationKind::Development | InstallationKind::Unknown => {
            return None
        }
    };
    let selected = names
        .filter(|asset| {
            let name = asset.name.to_ascii_lowercase();
            name.contains(wanted.0) && wanted.1.iter().any(|arch| name.contains(arch))
        })
        .min_by_key(|asset| asset.name.len())
        .map(|asset| SelectedAsset {
            name: asset.name.clone(),
            browser_download_url: asset.browser_download_url.clone(),
            size: asset.size,
            kind: wanted.0.to_string(),
        });
    if selected.is_some() || target.installation != InstallationKind::Manual {
        return selected;
    }
    assets
        .iter()
        .filter(|asset| {
            let name = asset.name.to_ascii_lowercase();
            !name.contains("sha256")
                && !name.contains("checksum")
                && !name.ends_with(".sig")
                && !name.ends_with(".deb")
                && !name.ends_with(".rpm")
                && !name.contains("pkg.tar")
                && name.contains("linux")
                && (name.contains(&arch) || (arch == "x86_64" && name.contains("x86_64")))
                && is_official_github_url(&asset.browser_download_url)
        })
        .min_by_key(|asset| asset.name.len())
        .map(|asset| SelectedAsset {
            name: asset.name.clone(),
            browser_download_url: asset.browser_download_url.clone(),
            size: asset.size,
            kind: "binary".to_string(),
        })
}

#[derive(Debug, Clone)]
struct SelectedAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    kind: String,
}

fn detect_installation() -> InstallationKind {
    if std::env::var_os("FLATPAK_ID").is_some() || Path::new("/app/bin").exists() {
        return InstallationKind::Flatpak;
    }
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        if PathBuf::from(appimage).exists() {
            return InstallationKind::AppImage;
        }
    }
    let Ok(executable) = std::env::current_exe() else {
        return InstallationKind::Unknown;
    };
    let executable_text = executable.to_string_lossy();
    if executable_text.contains("/target/") {
        return InstallationKind::Development;
    }
    let path = executable.to_string_lossy().into_owned();
    if system::run("pacman", &["-Qo", path.as_str()]).is_ok_and(|result| result.status == Some(0)) {
        return InstallationKind::Pacman;
    }
    if system::run("dpkg-query", &["-S", path.as_str()])
        .is_ok_and(|result| result.status == Some(0))
    {
        return InstallationKind::Apt;
    }
    if system::run("rpm", &["-qf", path.as_str()]).is_ok_and(|result| result.status == Some(0)) {
        return InstallationKind::Dnf;
    }
    if executable.extension().is_some_and(|ext| ext == "AppImage") {
        InstallationKind::AppImage
    } else {
        InstallationKind::Manual
    }
}

fn normalized_architecture() -> String {
    match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        other => other,
    }
    .to_string()
}

fn debian_arch(arch: &str) -> &str {
    match arch {
        "aarch64" => "arm64",
        "x86_64" => "amd64",
        other => other,
    }
}

fn rpm_arch(arch: &str) -> &str {
    match arch {
        "aarch64" => "aarch64",
        "x86_64" => "x86_64",
        other => other,
    }
}

fn package_arches<'a>(arch: &'a str, x86_alias: &'a str) -> Vec<&'a str> {
    if arch == "x86_64" {
        vec![arch, x86_alias]
    } else {
        vec![arch]
    }
}

fn parse_version(tag: &str) -> Result<Version> {
    Version::parse(tag.trim().trim_start_matches('v')).context("invalid semantic version")
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_official_github_url(url: &str) -> bool {
    url.starts_with("https://github.com/PranshulSoni/protonsearch/")
        || url.starts_with("https://objects.githubusercontent.com/")
}

fn http_agent() -> Result<ureq::Agent> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .build();
    Ok(config.into())
}

fn write_snapshot(paths: &XdgPaths, snapshot: &UpdateSnapshot) -> Result<()> {
    write_json_atomic(&state_path(paths), snapshot)
}

fn write_install_state(paths: &XdgPaths, stage: InstallStage, message: &str) -> Result<()> {
    write_json_atomic(
        &install_state_path(paths),
        &InstallState {
            stage,
            message: message.to_string(),
            updated_at: now_seconds(),
        },
    )
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("json.new");
    let contents = serde_json::to_vec_pretty(value)?;
    let mut file = File::create(&temporary)?;
    file.write_all(&contents)?;
    file.sync_all()?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_versions_compare_numerically() {
        assert!(Version::parse("1.10.0").unwrap() > Version::parse("1.9.9").unwrap());
        assert_eq!(parse_version("v1.4.2").unwrap(), Version::new(1, 4, 2));
    }

    #[test]
    fn checksum_validation_is_strict() {
        assert!(is_sha256(&"a".repeat(64)));
        assert!(!is_sha256("not-a-checksum"));
        assert!(!is_sha256(&"a".repeat(63)));
    }

    #[test]
    fn architecture_names_map_to_package_conventions() {
        assert_eq!(debian_arch("x86_64"), "amd64");
        assert_eq!(debian_arch("aarch64"), "arm64");
        assert_eq!(rpm_arch("aarch64"), "aarch64");
    }

    #[test]
    fn asset_selection_never_crosses_architectures() {
        let assets = vec![
            GitHubAsset {
                name: "protonsearch-linux_1.1.0_amd64.deb".to_string(),
                browser_download_url:
                    "https://github.com/PranshulSoni/protonsearch/releases/download/v1.1.0/a.deb"
                        .to_string(),
                size: 1,
            },
            GitHubAsset {
                name: "protonsearch-linux_1.1.0_arm64.deb".to_string(),
                browser_download_url:
                    "https://github.com/PranshulSoni/protonsearch/releases/download/v1.1.0/b.deb"
                        .to_string(),
                size: 1,
            },
        ];
        let target = PlatformTarget {
            distribution: "ubuntu".to_string(),
            family: "debian".to_string(),
            architecture: "aarch64".to_string(),
            installation: InstallationKind::Apt,
        };
        assert_eq!(select_asset(&assets, &target).unwrap().name, assets[1].name);
    }

    #[test]
    fn official_url_check_rejects_lookalike_hosts() {
        assert!(is_official_github_url(
            "https://github.com/PranshulSoni/protonsearch/releases/download/v1.1.0/a.deb"
        ));
        assert!(!is_official_github_url(
            "https://github.com.evil.example/PranshulSoni/protonsearch/a.deb"
        ));
        assert!(!is_official_github_url(
            "http://github.com/PranshulSoni/protonsearch/a"
        ));
    }
}
