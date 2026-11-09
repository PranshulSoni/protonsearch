//! Shell integration: open/reveal/launch, shortcuts and AppsFolder.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context};
use windows::core::{Interface, GUID, PCWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
use windows::Win32::System::Com::{
    CoCreateInstance, CoTaskMemFree, IBindCtx, IPersistFile, CLSCTX_INPROC_SERVER, STGM_READ,
};
use windows::Win32::UI::Shell::Common::{ITEMIDLIST, STRRET};
use windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;
use windows::Win32::UI::Shell::{
    IEnumIDList, ILCreateFromPathW, IShellFolder, IShellItem2, IShellLinkW, SHCreateItemWithParent,
    SHGetDesktopFolder, SHOpenFolderAndSelectItems, SHParseDisplayName, ShellExecuteW, StrRetToBufW,
    SHCONTF_FOLDERS, SHCONTF_NONFOLDERS, SHGDN_NORMAL, SIGDN_NORMALDISPLAY,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use super::util::{free_pwstr, hstr, hstr_path, pwstr_to_string, utf16_buf_to_string, ComGuard};
use crate::{AppsFolderEntry, ShortcutInfo};

/// CLSID_ShellLink — defined locally because the windows crate 0.58 does not
/// export every shell CLSID.
const CLSID_SHELLLINK: GUID = GUID::from_u128(0x00021401_0000_0000_c000_000000000046);

/// PKEY_AppUserModel_ID {9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3}, pid 5.
const PKEY_APP_USER_MODEL_ID: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3),
    pid: 5,
};

const SHELL_APPS_FOLDER: &str = "shell:AppsFolder";

pub(crate) fn shell_execute(verb: &str, file: &str, params: Option<&str>) -> anyhow::Result<()> {
    let op = hstr(verb);
    let file_h = hstr(file);
    let params_h = params.map(hstr);
    let params_pcwstr = params_h
        .as_ref()
        .map(|h| PCWSTR(h.as_ptr()))
        .unwrap_or_else(PCWSTR::null);
    let instance = unsafe {
        ShellExecuteW(
            HWND::default(),
            &op,
            &file_h,
            params_pcwstr,
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    let code = instance.0 as isize;
    if code <= 32 {
        bail!("ShellExecute failed for {verb:?} ({file}) with code {code}");
    }
    Ok(())
}

pub(crate) fn open_path(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        bail!("path does not exist: {}", path.display());
    }
    shell_execute("open", &path.to_string_lossy(), None)
}

/// Open in a known editor at a line/column when possible, else plain-open.
pub(crate) fn open_path_at(
    path: &Path,
    line: Option<u32>,
    column: Option<u32>,
) -> anyhow::Result<()> {
    if let Some(line) = line {
        if let Some(code) = find_vscode() {
            let mut goto = format!("{}:{}", path.display(), line);
            if let Some(col) = column {
                goto.push(':');
                goto.push_str(&col.to_string());
            }
            let status = std::process::Command::new(&code)
                .arg("-g")
                .arg(&goto)
                .creation_flags_no_window()
                .status();
            if let Ok(status) = status {
                if status.success() {
                    return Ok(());
                }
            }
        }
    }
    open_path(path)
}

/// VS Code exposes a per-user install and a machine-wide install.
fn find_vscode() -> Option<PathBuf> {
    let candidates = [
        std::env::var_os("LOCALAPPDATA")
            .map(|b| PathBuf::from(b).join("Programs/Microsoft VS Code/Code.exe")),
        std::env::var_os("ProgramFiles")
            .map(|b| PathBuf::from(b).join("Microsoft VS Code/Code.exe")),
        std::env::var_os("ProgramFiles(x86)")
            .map(|b| PathBuf::from(b).join("Microsoft VS Code/Code.exe")),
    ];
    candidates.into_iter().flatten().find(|p| p.is_file())
}

pub(crate) fn reveal_path(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        bail!("path does not exist: {}", path.display());
    }
    let text = hstr_path(path);
    let pidl = unsafe { ILCreateFromPathW(&text) };
    if pidl.is_null() {
        bail!("ILCreateFromPathW failed for {}", path.display());
    }
    let result = unsafe { SHOpenFolderAndSelectItems(pidl, None, 0) };
    unsafe { CoTaskMemFree(Some(pidl as *const _)) };
    result.map_err(|e| anyhow!("SHOpenFolderAndSelectItems failed: {e}"))
}

/// Only http/https URLs are accepted; everything else is rejected before any
/// shell call so a crafted search result cannot invoke a local protocol handler.
pub(crate) fn open_url(url: &str) -> anyhow::Result<()> {
    let trimmed = url.trim();
    if trimmed.len() > 8 * 1024 {
        bail!("url too long");
    }
    let lower = trimmed.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        bail!("only http(s) urls can be opened");
    }
    if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
        bail!("url contains control characters");
    }
    let rest = trimmed
        .split_once("://")
        .map(|(_, r)| r)
        .unwrap_or_default();
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if host.is_empty() || host.starts_with(':') {
        bail!("url has no host");
    }
    shell_execute("open", trimmed, None)
}

/// Launch a packaged app by AUMID / shell path, or an executable by target path.
pub(crate) fn launch_app(app_id: &str, target: Option<&str>) -> anyhow::Result<()> {
    let app_id = app_id.trim();
    let target = target.map(str::trim).filter(|t| !t.is_empty());
    if app_id.starts_with("shell:") {
        return shell_execute("open", app_id, None);
    }
    if let Some(target) = target {
        if target.starts_with("shell:") {
            return shell_execute("open", target, None);
        }
        if Path::new(target).is_file() {
            return shell_execute("open", target, None);
        }
    }
    if Path::new(app_id).is_file() {
        return shell_execute("open", app_id, None);
    }
    let as_folder = format!("{SHELL_APPS_FOLDER}\\{app_id}");
    shell_execute("open", &as_folder, None)
        .with_context(|| format!("could not launch app {app_id}"))
}

/// Resolve a Windows .lnk via COM (IShellLinkW + IPersistFile).
pub(crate) fn resolve_shortcut(lnk: &Path) -> anyhow::Result<ShortcutInfo> {
    if !lnk.is_file() {
        bail!("shortcut does not exist: {}", lnk.display());
    }
    let _com = ComGuard::new();
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&CLSID_SHELLLINK, None, CLSCTX_INPROC_SERVER)
            .context("CoCreateInstance(CLSID_ShellLink)")?;
        let persist: IPersistFile = link.cast().context("IShellLinkW -> IPersistFile")?;
        let name = hstr_path(lnk);
        persist
            .Load(&name, STGM_READ)
            .with_context(|| format!("loading shortcut {}", lnk.display()))?;

        let mut path_buf = [0u16; 1024];
        let target = link
            .GetPath(
                &mut path_buf,
                std::ptr::null_mut::<WIN32_FIND_DATAW>(),
                0,
            )
            .ok()
            .map(|_| PathBuf::from(utf16_buf_to_string(&path_buf)))
            .filter(|p| !p.as_os_str().is_empty());

        let mut args_buf = [0u16; 1024];
        let arguments = link
            .GetArguments(&mut args_buf)
            .ok()
            .map(|_| utf16_buf_to_string(&args_buf))
            .filter(|s| !s.is_empty());

        let mut icon_buf = [0u16; 1024];
        let mut icon_index = 0i32;
        let icon_location = link
            .GetIconLocation(&mut icon_buf, &mut icon_index)
            .ok()
            .map(|_| utf16_buf_to_string(&icon_buf))
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);

        Ok(ShortcutInfo {
            target,
            arguments,
            icon_location,
            icon_index,
        })
    }
}

/// Enumerate the shell AppsFolder namespace (packaged apps + shell apps).
///
/// Only entries that expose an AppUserModelID are returned; classic .lnk apps
/// are discovered from the Start Menu directories instead.
pub(crate) fn list_appsfolder() -> anyhow::Result<Vec<AppsFolderEntry>> {
    let _com = ComGuard::new();
    unsafe {
        let desktop = SHGetDesktopFolder().context("SHGetDesktopFolder")?;
        let mut folder_pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        let mut attrs = 0u32;
        SHParseDisplayName(
            &hstr(SHELL_APPS_FOLDER),
            None,
            &mut folder_pidl,
            0,
            Some(&mut attrs),
        )
        .context("SHParseDisplayName(shell:AppsFolder)")?;
        if folder_pidl.is_null() {
            bail!("shell:AppsFolder did not resolve to a PIDL");
        }
        let folder: IShellFolder = desktop
            .BindToObject(folder_pidl, None::<&IBindCtx>)
            .context("BindToObject(shell:AppsFolder)")?;

        let mut enumerator: Option<IEnumIDList> = None;
        let hr = folder.EnumObjects(
            HWND::default(),
            (SHCONTF_FOLDERS.0 | SHCONTF_NONFOLDERS.0) as u32,
            &mut enumerator,
        );
        hr.ok().context("EnumObjects(shell:AppsFolder)")?;
        let enumerator = enumerator.ok_or_else(|| anyhow!("AppsFolder enumerator missing"))?;

        let mut entries = Vec::new();
        loop {
            let child: *mut ITEMIDLIST = std::ptr::null_mut();
            let mut fetched = 0u32;
            let hr = enumerator.Next(&mut [child], Some(&mut fetched));
            if hr.is_err() || fetched == 0 || child.is_null() {
                if !child.is_null() {
                    CoTaskMemFree(Some(child as *const _));
                }
                break;
            }
            let name = display_name(&folder, child).unwrap_or_default();
            let aumid = aumid_of(&folder, folder_pidl, child).unwrap_or_default();
            if !aumid.is_empty() && !name.is_empty() {
                entries.push(AppsFolderEntry { name, aumid });
            }
            CoTaskMemFree(Some(child as *const _));
        }
        CoTaskMemFree(Some(folder_pidl as *const _));
        Ok(entries)
    }
}

fn display_name(folder: &IShellFolder, child: *const ITEMIDLIST) -> anyhow::Result<String> {
    unsafe {
        if let Ok(item) =
            SHCreateItemWithParent::<&IShellFolder, IShellItem2>(None, folder, child)
        {
            if let Ok(name) = item.GetDisplayName(SIGDN_NORMALDISPLAY) {
                let s = pwstr_to_string(name);
                free_pwstr(name);
                if !s.is_empty() {
                    return Ok(s);
                }
            }
        }
        let mut strret = STRRET::default();
        folder.GetDisplayNameOf(child, SHGDN_NORMAL, &mut strret)?;
        let mut buf = [0u16; 512];
        StrRetToBufW(&mut strret, Some(child), &mut buf)?;
        Ok(utf16_buf_to_string(&buf))
    }
}

fn aumid_of(
    folder: &IShellFolder,
    folder_pidl: *const ITEMIDLIST,
    child: *const ITEMIDLIST,
) -> anyhow::Result<String> {
    unsafe {
        let item: IShellItem2 =
            SHCreateItemWithParent(Some(folder_pidl), folder, child).context("IShellItem2")?;
        let value = item.GetString(&PKEY_APP_USER_MODEL_ID)?;
        let s = pwstr_to_string(value);
        free_pwstr(value);
        Ok(s)
    }
}

// Small extension to keep the no-window rule explicit for child processes.
trait NoWindow {
    fn creation_flags_no_window(&mut self) -> &mut Self;
}

impl NoWindow for std::process::Command {
    fn creation_flags_no_window(&mut self) -> &mut Self {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            self.creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        }
        #[cfg(not(windows))]
        {
            self
        }
    }
}
