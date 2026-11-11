//! Host/environment capture. Everything recorded in result files comes from here,
//! so every claim can be traced to the machine it was measured on.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInfo {
    pub os: String,
    pub cpu: String,
    pub physical_cores: u32,
    pub logical_cores: u32,
    pub ram_bytes: u64,
    pub hostname: String,
    pub rustc: String,
    pub captured_at_local: String,
    pub captured_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMemory {
    pub pid: u32,
    pub working_set_bytes: u64,
    pub peak_working_set_bytes: u64,
}

/// Run a Windows program without flashing a console window.
#[cfg(windows)]
pub fn hidden_command(program: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut cmd = std::process::Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(not(windows))]
pub fn hidden_command(program: &str) -> std::process::Command {
    std::process::Command::new(program)
}

fn powershell_json(script: &str) -> Option<serde_json::Value> {
    let out = hidden_command("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(text.trim()).ok()
}

fn compute_host_info() -> HostInfo {
    let script = r#"$ErrorActionPreference='Stop'
$os=Get-CimInstance Win32_OperatingSystem
$cs=Get-CimInstance Win32_ComputerSystem
$cpu=Get-CimInstance Win32_Processor | Select-Object -First 1
[pscustomobject]@{
  os=("$($os.Caption) $($os.Version) build $($os.BuildNumber)")
  cpu=$cpu.Name.Trim()
  physical_cores=[int]$cpu.NumberOfCores
  logical_cores=[int]$cpu.NumberOfLogicalProcessors
  ram_bytes=[int64]$cs.TotalPhysicalMemory
  hostname=$env:COMPUTERNAME
  captured_at_local=(Get-Date -Format o)
  captured_at_utc=((Get-Date).ToUniversalTime().ToString('o'))
} | ConvertTo-Json -Compress"#;
    let v = powershell_json(script);
    let rustc = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    match v {
        Some(v) => HostInfo {
            os: v
                .get("os")
                .and_then(|x| x.as_str())
                .unwrap_or("unknown")
                .to_string(),
            cpu: v
                .get("cpu")
                .and_then(|x| x.as_str())
                .unwrap_or("unknown")
                .to_string(),
            physical_cores: v.get("physical_cores").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
            logical_cores: v.get("logical_cores").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
            ram_bytes: v.get("ram_bytes").and_then(|x| x.as_u64()).unwrap_or(0),
            hostname: v
                .get("hostname")
                .and_then(|x| x.as_str())
                .unwrap_or("unknown")
                .to_string(),
            rustc,
            captured_at_local: v
                .get("captured_at_local")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            captured_at_utc: v
                .get("captured_at_utc")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
        },
        None => HostInfo {
            os: std::env::consts::OS.to_string(),
            cpu: "unknown".to_string(),
            physical_cores: 0,
            logical_cores: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as u32,
            ram_bytes: 0,
            hostname: std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".to_string()),
            rustc,
            captured_at_local: String::new(),
            captured_at_utc: String::new(),
        },
    }
}

static HOST: OnceLock<HostInfo> = OnceLock::new();

pub fn host_info() -> &'static HostInfo {
    HOST.get_or_init(compute_host_info)
}

pub fn process_memory(pid: u32) -> Option<ProcessMemory> {
    let script = format!(
        "$p=Get-Process -Id {pid} -ErrorAction Stop; [pscustomobject]@{{pid={pid};working_set_bytes=[int64]$p.WorkingSet64;peak_working_set_bytes=[int64]$p.PeakWorkingSet64}} | ConvertTo-Json -Compress"
    );
    let v = powershell_json(&script)?;
    Some(ProcessMemory {
        pid,
        working_set_bytes: v.get("working_set_bytes").and_then(|x| x.as_u64()).unwrap_or(0),
        peak_working_set_bytes: v
            .get("peak_working_set_bytes")
            .and_then(|x| x.as_u64())
            .unwrap_or(0),
    })
}

pub fn self_memory() -> Option<ProcessMemory> {
    process_memory(std::process::id())
}
