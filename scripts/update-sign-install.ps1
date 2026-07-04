param(
    [string]$Repo = "PranshulSoni/protonsearch",
    [string]$AssetPattern = "protonsearchsetup*.exe",
    [string]$OutputDir = "$env:TEMP\ProtonSearchUpdate",
    [string]$TimestampUrl = "http://timestamp.digicert.com",
    [string]$CertThumbprint = "",
    [string]$CertStore = "Cert:\CurrentUser\My",
    [string]$PfxPath = "",
    [securestring]$PfxPassword,
    [string[]]$InstallerArgs = @()
)

$ErrorActionPreference = "Stop"

function Find-SignTool {
    $fromPath = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($fromPath) {
        return $fromPath.Source
    }

    $kitsRoot = "${env:ProgramFiles(x86)}\Windows Kits\10\bin"
    if (Test-Path $kitsRoot) {
        $tool = Get-ChildItem -Path $kitsRoot -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match "\\x64\\signtool\.exe$" } |
            Sort-Object FullName -Descending |
            Select-Object -First 1
        if ($tool) {
            return $tool.FullName
        }
    }

    throw "signtool.exe was not found. Install Windows SDK or add signtool.exe to PATH."
}

function Convert-SecureStringToPlainText {
    param([securestring]$Value)
    if (-not $Value) {
        return ""
    }

    $bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($Value)
    try {
        return [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr)
    } finally {
        [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)
    }
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$headers = @{
    "Accept" = "application/vnd.github+json"
    "User-Agent" = "ProtonSearch-Update-Signer"
}

$releaseUrl = "https://api.github.com/repos/$Repo/releases/latest"
Write-Host "Fetching latest release: $releaseUrl"
$release = Invoke-RestMethod -Uri $releaseUrl -Headers $headers

$asset = $release.assets |
    Where-Object { $_.name -like $AssetPattern } |
    Select-Object -First 1

if (-not $asset) {
    throw "No release asset matched '$AssetPattern' in $($release.tag_name)."
}

$installerPath = Join-Path $OutputDir $asset.name
Write-Host "Downloading $($asset.name) from $($release.tag_name)..."
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $installerPath -Headers @{ "User-Agent" = "ProtonSearch-Update-Signer" }

if (-not (Test-Path $installerPath)) {
    throw "Download failed: $installerPath was not created."
}

$signTool = Find-SignTool
Write-Host "Signing with: $signTool"

if ($PfxPath) {
    if (-not (Test-Path $PfxPath)) {
        throw "PFX file not found: $PfxPath"
    }

    $plainPassword = Convert-SecureStringToPlainText $PfxPassword
    $args = @("sign", "/fd", "SHA256", "/td", "SHA256", "/tr", $TimestampUrl, "/f", $PfxPath)
    if ($plainPassword) {
        $args += @("/p", $plainPassword)
    }
    $args += $installerPath
} elseif ($CertThumbprint) {
    $certPath = Join-Path $CertStore $CertThumbprint
    if (-not (Test-Path $certPath)) {
        throw "Certificate thumbprint not found: $certPath"
    }

    $args = @("sign", "/fd", "SHA256", "/td", "SHA256", "/tr", $TimestampUrl, "/sha1", $CertThumbprint, $installerPath)
} else {
    throw "Provide either -CertThumbprint or -PfxPath. The installer was downloaded but not signed."
}

& $signTool @args
if ($LASTEXITCODE -ne 0) {
    throw "Signing failed with exit code $LASTEXITCODE."
}

& $signTool verify /pa /v $installerPath
if ($LASTEXITCODE -ne 0) {
    throw "Signature verification failed with exit code $LASTEXITCODE."
}

Write-Host "Running installer: $installerPath"
$process = Start-Process -FilePath $installerPath -ArgumentList $InstallerArgs -Wait -PassThru
if ($process.ExitCode -ne 0) {
    throw "Installer exited with code $($process.ExitCode)."
}

Write-Host "Done."
