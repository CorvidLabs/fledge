# Fledge installer for Windows (PowerShell)
# Usage: irm https://raw.githubusercontent.com/CorvidLabs/fledge/main/install.ps1 | iex

$ErrorActionPreference = "Stop"
$Repo = "CorvidLabs/fledge"

function Get-LatestVersion {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -UseBasicParsing
    return $release.tag_name
}

function Get-Artifact {
    $arch = if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq [System.Runtime.InteropServices.Architecture]::Arm64) {
        "aarch64"
    } else {
        "x86_64"
    }
    return "fledge-windows-$arch.exe"
}

$version = Get-LatestVersion
if (-not $version) {
    Write-Error "Could not determine latest version."
    exit 1
}

$artifact = Get-Artifact
$installDir = if ($env:FLEDGE_INSTALL_DIR) { $env:FLEDGE_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "fledge" }

Write-Host "  Installing fledge $version..." -ForegroundColor Cyan

if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

$url = "https://github.com/$Repo/releases/download/$version/$artifact"
$checksumUrl = "$url.sha256"
$tmpFile = Join-Path $env:TEMP "fledge-download.exe"

Write-Host "  Downloading $url"
Invoke-WebRequest -Uri $url -OutFile $tmpFile -UseBasicParsing

try {
    $expectedHash = (Invoke-WebRequest -Uri $checksumUrl -UseBasicParsing).Content.Trim().Split(" ")[0]
    $actualHash = (Get-FileHash -Path $tmpFile -Algorithm SHA256).Hash.ToLower()
    if ($actualHash -ne $expectedHash) {
        Remove-Item $tmpFile -Force
        Write-Error "Checksum mismatch! Expected: $expectedHash, Got: $actualHash"
        exit 1
    }
    Write-Host "  Checksum verified." -ForegroundColor Green
} catch {
    Write-Host "  Warning: Could not verify checksum (continuing anyway)." -ForegroundColor Yellow
}

$dest = Join-Path $installDir "fledge.exe"
Move-Item -Path $tmpFile -Destination $dest -Force

$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$userPath;$installDir", "User")
    Write-Host "  Added $installDir to user PATH." -ForegroundColor Green
    Write-Host "  Restart your terminal for PATH changes to take effect."
}

Write-Host ""
Write-Host "  Installed fledge $version to $dest" -ForegroundColor Green
Write-Host "  Run 'fledge --help' to get started."
