#Requires -Version 5.1
# ProtonSearch v2 - unsigned NSIS installer build wrapper.
# Does NOT publish. Does NOT sign. Does NOT touch %APPDATA%.
#
# Policy: the actual build must only run after the Lead confirms the integration
# build is green. Therefore this script is DRY-RUN by default; pass -Execute to build.
#
# Usage:
#   powershell -NoProfile -File scripts/build-installer.ps1            # preflight + plan only
#   powershell -NoProfile -File scripts/build-installer.ps1 -Execute   # runs the build
[CmdletBinding()]
param(
    [switch]$Execute,
    [string]$OutputDir = 'dist-installer'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Root = Split-Path -Parent $PSScriptRoot
$Desktop = Join-Path $Root 'apps\desktop'
$TauriDir = Join-Path $Desktop 'src-tauri'
$OutDir = Join-Path $Root $OutputDir

function Write-Step([string]$Text) { Write-Host ('==> ' + $Text) -ForegroundColor Cyan }
function Write-Warn2([string]$Text) { Write-Host ('[warn] ' + $Text) -ForegroundColor Yellow }
function Assert-Cmd([string]$Name) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw ('Required command not found: ' + $Name)
    }
}
function Get-Sha256([string]$Path) { return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash }

Write-Step 'Preflight: workspace + toolchains'
Assert-Cmd 'cargo'
Assert-Cmd 'node'
Assert-Cmd 'npm'
foreach ($p in @('Cargo.toml', 'Cargo.lock', 'apps\desktop\package.json', 'apps\desktop\package-lock.json', 'apps\desktop\src-tauri\Cargo.toml', 'apps\desktop\src-tauri\tauri.conf.json')) {
    $full = Join-Path $Root $p
    if (-not (Test-Path -LiteralPath $full)) { throw ('Missing required file: ' + $full) }
}
$cargoVersion = ((& cargo --version) -join ' ').Trim()
$rustcVersion = ((& rustc --version) -join ' ').Trim()
$nodeVersion = ((& node --version) -join ' ').Trim()
$npmVersion = ((& npm --version) -join ' ').Trim()
$gitRev = ''
if (Get-Command git -ErrorAction SilentlyContinue) { $gitRev = ((& git -C $Root rev-parse --short HEAD) -join '').Trim() }
Write-Host ('  rustc: ' + $rustcVersion)
Write-Host ('  cargo: ' + $cargoVersion)
Write-Host ('  node:  ' + $nodeVersion + '   npm: ' + $npmVersion)
Write-Host ('  git:   ' + $gitRev)

Write-Step 'Preflight: lockfiles must be committed (reproducible builds)'
if (Get-Command git -ErrorAction SilentlyContinue) {
    $status = @(& git -C $Root status --porcelain -- Cargo.lock apps/desktop/package-lock.json 2>$null)
    if ($status.Count -gt 0) {
        Write-Warn2 'Lockfiles are modified/untracked; commit them before producing a release artifact.'
        $status | ForEach-Object { Write-Host ('    ' + $_) }
    }
}

Write-Step 'Preflight: running ProtonSearch processes'
$procs = @(Get-Process -Name 'ProtonSearch' -ErrorAction SilentlyContinue)
if ($procs.Count -gt 0) {
    $pids = (($procs | ForEach-Object { $_.Id } | Sort-Object) -join ',')
    if ($Execute) {
        Write-Warn2 ('Killing running ProtonSearch PID(s): ' + $pids)
        $procs | Stop-Process -Force
    } else {
        Write-Warn2 ('ProtonSearch is running (PID(s): ' + $pids + '); -Execute would kill it.')
    }
} else {
    Write-Host '  no running ProtonSearch process'
}

Write-Step 'Preflight: data-dir safety'
Write-Host '  This script never writes to %APPDATA%\protonsearch. E2E/migration tests use'
Write-Host '  PROTONSEARCH_DATA_DIR pointing at a temp directory (docs/release/MIGRATION.md).'

if (-not $Execute) {
    Write-Host ''
    Write-Host 'DRY RUN complete. No files were built or modified.' -ForegroundColor Green
    Write-Host 'Re-run with -Execute ONLY after the Lead confirms the integration build is green.'
    exit 0
}

Write-Step 'Frontend build (tsc --noEmit + vite build)'
Push-Location $Desktop
try {
    & npm run build
    if ($LASTEXITCODE -ne 0) { throw ('npm run build failed with exit code ' + $LASTEXITCODE) }
} finally { Pop-Location }

Write-Step 'Rust release build (workspace)'
Push-Location $Root
try {
    & cargo build --workspace --release --locked
    if ($LASTEXITCODE -ne 0) { throw ('cargo build failed with exit code ' + $LASTEXITCODE) }
} finally { Pop-Location }

Write-Step 'Tauri NSIS bundle (unsigned)'
Push-Location $Desktop
try {
    & npx tauri build --bundles nsis
    if ($LASTEXITCODE -ne 0) { throw ('tauri build failed with exit code ' + $LASTEXITCODE) }
} finally { Pop-Location }

Write-Step 'Collect artifacts'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
# Tauri/Cargo may place artifacts under the workspace target dir or the
# per-app target dir depending on how the CLI resolves the workspace root.
$bundleCandidates = @(
    (Join-Path $Root 'target\release\bundle\nsis'),
    (Join-Path $TauriDir 'target\release\bundle\nsis')
)
$bundleDir = $bundleCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $bundleDir) {
    throw ('No NSIS bundle dir found; checked: ' + ($bundleCandidates -join ', '))
}
Write-Host ('  bundle dir: ' + $bundleDir)
$installer = Get-ChildItem -LiteralPath $bundleDir -Filter '*.exe' -File -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $installer) { throw ('No NSIS installer found under ' + $bundleDir) }
$exeCandidates = @(
    (Join-Path $Root 'target\release\ProtonSearch.exe'),
    (Join-Path $TauriDir 'target\release\ProtonSearch.exe')
)
$exe = $exeCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
$copied = @()
Copy-Item -LiteralPath $installer.FullName -Destination (Join-Path $OutDir $installer.Name) -Force
$copied += (Join-Path $OutDir $installer.Name)
if ($exe) {
    Copy-Item -LiteralPath $exe -Destination (Join-Path $OutDir 'ProtonSearch.exe') -Force
    $copied += (Join-Path $OutDir 'ProtonSearch.exe')
} else {
    Write-Warn2 'ProtonSearch.exe not found in a known target dir; installer copied without bare exe.'
}

Write-Step 'Checksums + build info'
$lines = foreach ($f in $copied) { (Get-Sha256 $f) + '  ' + (Split-Path -Leaf $f) }
$lines | Set-Content -LiteralPath (Join-Path $OutDir 'SHA256SUMS.txt') -Encoding ASCII
$info = @(
    'ProtonSearch v2 build metadata',
    ('built_utc=' + [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')),
    ('git_rev=' + $gitRev),
    ('rustc=' + $rustcVersion),
    ('cargo=' + $cargoVersion),
    ('node=' + $nodeVersion),
    ('npm=' + $npmVersion),
    'bundle_target=nsis',
    'installer_mode=currentUser',
    'SIGNED=false',
    '',
    'UNSIGNED BUILD: these artifacts are not code-signed. Do not distribute publicly.',
    'Before release: Authenticode-sign the installer, verify with signtool /pa, and',
    'publish SHA256SUMS + a signed update manifest (docs/security/SECURITY_RISK_REGISTER.md S-02).'
)
$info | Set-Content -LiteralPath (Join-Path $OutDir 'BUILD_INFO.txt') -Encoding UTF8
$copied | ForEach-Object { Write-Host ('  ' + $_) }
Write-Host ('  checksums: ' + (Join-Path $OutDir 'SHA256SUMS.txt'))
Write-Warn2 'UNSIGNED ARTIFACT - signing is a G7 release gate; do not publish.'
Write-Host 'Installer build complete (local only).' -ForegroundColor Green
