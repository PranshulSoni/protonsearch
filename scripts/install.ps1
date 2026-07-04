param(
    [string]$Repo = "PranshulSoni/protonsearch",
    [string]$AssetPattern = "protonsearchsetup*.exe",
    [string]$OutputDir = "$env:TEMP\ProtonSearchInstaller",
    [string[]]$InstallerArgs = @()
)

$ErrorActionPreference = "Stop"

$headers = @{
    "Accept" = "application/vnd.github+json"
    "User-Agent" = "ProtonSearch-Installer"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$releaseUrl = "https://api.github.com/repos/$Repo/releases/latest"
Write-Host "Fetching latest ProtonSearch release..."
$release = Invoke-RestMethod -Uri $releaseUrl -Headers $headers

$asset = $release.assets |
    Where-Object { $_.name -like $AssetPattern } |
    Select-Object -First 1

if (-not $asset) {
    throw "No installer asset matching '$AssetPattern' was found in $($release.tag_name)."
}

$installerPath = Join-Path $OutputDir $asset.name
Write-Host "Downloading $($asset.name) from $($release.tag_name)..."
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $installerPath -Headers @{ "User-Agent" = "ProtonSearch-Installer" }

if (-not (Test-Path $installerPath)) {
    throw "Download failed: $installerPath was not created."
}

Write-Host "Running ProtonSearch installer..."
$startArgs = @{
    FilePath = $installerPath
    Wait = $true
    PassThru = $true
}
if ($InstallerArgs -and $InstallerArgs.Count -gt 0) {
    $startArgs.ArgumentList = $InstallerArgs
}

$process = Start-Process @startArgs
if ($process.ExitCode -ne 0) {
    throw "Installer exited with code $($process.ExitCode)."
}

Write-Host "ProtonSearch install complete."
