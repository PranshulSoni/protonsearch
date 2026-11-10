#Requires -Version 5.1
# Run the release build from a normal folder instead of target\release.
#
# This repo's folder carries a Low integrity label (set by an agent sandbox), and
# Windows starts any exe stored under a Low-labelled folder as a low-integrity
# process. WebView2 then cannot create its profile in %LOCALAPPDATA%, window
# creation fails (0x8000FFFF) and Alt+Space only spins the cursor. A copy outside
# the repo runs at normal (medium) integrity.
#
# Usage (after `npm run tauri build -- --no-bundle` in apps\desktop):
#   powershell -NoProfile -File scripts/run-release.ps1
$ErrorActionPreference = 'Stop'

$exe = Join-Path (Split-Path -Parent $PSScriptRoot) 'target\release\ProtonSearch.exe'
$dir = Join-Path $env:USERPROFILE 'ProtonSearch-Release\dev'
$run = Join-Path $dir 'ProtonSearch.exe'

# The running copy locks its own file; stop it before overwriting.
Get-Process -Name 'ProtonSearch' -ErrorAction SilentlyContinue |
    Stop-Process -Force -PassThru |
    Wait-Process -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $dir -Force | Out-Null
# Windows can keep the image locked for a moment after the process is gone.
for ($try = 0; ; $try++) {
    try { Copy-Item -LiteralPath $exe -Destination $run -Force; break }
    catch { if ($try -ge 20) { throw }; Start-Sleep -Milliseconds 250 }
}
Start-Process -FilePath $run -WorkingDirectory $dir
Write-Host ('Started ' + $run + ' (built ' + (Get-Item -LiteralPath $exe).LastWriteTime + ')')

