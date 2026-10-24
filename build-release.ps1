# Build and Release Packaging Script for Flip 3D (Rust)
# Creates both PORTABLE ZIP and INSTALLER distributions
# Usage: powershell -ExecutionPolicy Bypass -File .\build-release.ps1 [-Target <triple>] [-Version <v>] [-Clean] [-Sign]

[CmdletBinding()]
param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [string]$Version,
    [switch]$Clean,
    [switch]$Sign
)

$ErrorActionPreference = "Stop"
$RepoRoot = $PSScriptRoot

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "  Flip 3D Release Packaging Pipeline                      " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Resolve Version
if (-not $Version) {
    $CargoEngineToml = Get-Content (Join-Path $RepoRoot "crates\engine\Cargo.toml") -Raw
    if ($CargoEngineToml -match 'version\s*=\s*"([^"]+)"') {
        $Version = $Matches[1]
    } else {
        $Version = "1.7.1"
    }
}
Write-Host "Release Version: v$Version" -ForegroundColor Green
Write-Host "Target Architecture: $Target" -ForegroundColor Green

$DistDir = Join-Path $RepoRoot "dist"
$StagingRoot = Join-Path $RepoRoot "target\staging"
$StagingFlip3D = Join-Path $StagingRoot "Flip3D"

# 2. Clean if requested
if ($Clean) {
    Write-Host "`nCleaning previous build artifacts..." -ForegroundColor Yellow
    if (Test-Path $StagingRoot) { Remove-Item -Recurse -Force $StagingRoot }
    if (Test-Path $DistDir) { Remove-Item -Recurse -Force $DistDir }
}

New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

# 3. Quality Gates: Tests & Clippy
Write-Host "`n[Gate 1/2] Running workspace test suite..." -ForegroundColor Cyan
& cargo test --workspace
if ($LASTEXITCODE -ne 0) {
    Write-Error "Test gate failed! Aborting release packaging."
    exit 1
}

Write-Host "`n[Gate 2/2] Running workspace clippy check (-D warnings)..." -ForegroundColor Cyan
& cargo clippy --workspace -- -D warnings
if ($LASTEXITCODE -ne 0) {
    Write-Error "Clippy gate failed! Aborting release packaging."
    exit 1
}

# 4. Build Release Binaries
Write-Host "`nBuilding release binaries (LTO + Static CRT + Path Remapping)..." -ForegroundColor Cyan
$UserHome = $env:USERPROFILE
$WorkDir = $RepoRoot
$env:RUSTFLAGS = "-C target-feature=+crt-static --remap-path-prefix $WorkDir=. --remap-path-prefix $UserHome=~"
& cargo build --release --workspace
if ($LASTEXITCODE -ne 0) {
    Write-Error "Cargo release build failed! Aborting release packaging."
    exit 1
}

$EngineExe = Join-Path $RepoRoot "target\release\flip3d.exe"
$SettingsExe = Join-Path $RepoRoot "target\release\settings-ui.exe"

if (-not (Test-Path $EngineExe) -or -not (Test-Path $SettingsExe)) {
    Write-Error "Release executables not found in target\release!"
    exit 1
}

# Optional code signing
if ($Sign) {
    Write-Host "`nChecking code signing credentials..." -ForegroundColor Cyan
    $PfxPath = $env:FLIP3D_SIGN_PFX
    $PfxPassword = $env:FLIP3D_SIGN_PW
    $Signtool = (Get-ChildItem -Path "C:\Program Files (x86)\Windows Kits\10\bin" -Filter signtool.exe -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName)

    if ($PfxPath -and (Test-Path $PfxPath) -and $Signtool) {
        Write-Host "Signing binaries using signtool.exe..." -ForegroundColor Green
        & $Signtool sign /f $PfxPath /p $PfxPassword /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 $EngineExe $SettingsExe
    } else {
        Write-Warning "Signing requested but FLIP3D_SIGN_PFX not set or signtool not found. Skipping code signing."
    }
}

# 5. Stage and Package Portable ZIP
Write-Host "`nPackaging Portable Distribution (ZIP)..." -ForegroundColor Cyan
if (Test-Path $StagingRoot) {
    Remove-Item -Recurse -Force $StagingRoot
}
New-Item -ItemType Directory -Force -Path $StagingFlip3D | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $StagingFlip3D "redist") | Out-Null

Copy-Item $EngineExe -Destination $StagingFlip3D
Copy-Item $SettingsExe -Destination $StagingFlip3D
Copy-Item (Join-Path $RepoRoot "LICENSE") -Destination $StagingFlip3D
Copy-Item (Join-Path $RepoRoot "README-portable.txt") -Destination $StagingFlip3D

# Marker file for portable mode
New-Item -ItemType File -Force -Path (Join-Path $StagingFlip3D "flip3d.portable") | Out-Null

# Copy WebView2 bootstrapper to redist folder
$WebviewInstaller = Join-Path $RepoRoot "installer\MicrosoftEdgeWebview2Setup.exe"
if (-not (Test-Path $WebviewInstaller)) {
    $WebviewInstaller = Join-Path $RepoRoot "MicrosoftEdgeWebview2Setup.exe"
}
if (Test-Path $WebviewInstaller) {
    Copy-Item $WebviewInstaller -Destination (Join-Path $StagingFlip3D "redist\MicrosoftEdgeWebview2Setup.exe")
}

$PortableZipName = "Flip3D-v$Version-portable-x64.zip"
$PortableZipPath = Join-Path $DistDir $PortableZipName

if (Test-Path $PortableZipPath) {
    Remove-Item -Force $PortableZipPath
}

# Compress folder Flip3D/ into zip
Compress-Archive -Path $StagingFlip3D -DestinationPath $PortableZipPath -CompressionLevel Optimal
Write-Host "Created Portable ZIP: $PortableZipPath" -ForegroundColor Green

# 6. Build Inno Setup Installer
Write-Host "`nPackaging Installer Distribution (Inno Setup 6)..." -ForegroundColor Cyan

function Find-Iscc {
    if ($env:ISCC_PATH -and (Test-Path $env:ISCC_PATH)) { return $env:ISCC_PATH }
    $cmd = Get-Command iscc.exe -ErrorAction SilentlyContinue
    if ($cmd -and $cmd.Source) { return $cmd.Source }
    $candidates = @(
        "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
        "$env:LOCALAPPDATA\Programs\Inno Setup 6\iscc.exe",
        "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
        "C:\Program Files (x86)\Inno Setup 6\iscc.exe",
        "C:\Program Files\Inno Setup 6\ISCC.exe",
        "C:\Program Files\Inno Setup 6\iscc.exe"
    )
    foreach ($path in $candidates) {
        if (Test-Path $path) { return $path }
    }
    return $null
}

$IsccPath = Find-Iscc

if (-not $IsccPath) {
    Write-Error "Inno Setup compiler (ISCC.exe) not found! Install Inno Setup 6 via: winget install --id JRSoftware.InnoSetup -e"
    exit 1
}

Write-Host "Using Inno Setup compiler: $IsccPath" -ForegroundColor Green
$SetupIssPath = Join-Path $RepoRoot "installer\setup.iss"
& $IsccPath "/DMyAppVersion=$Version" $SetupIssPath
if ($LASTEXITCODE -ne 0) {
    Write-Error "Inno Setup compilation failed!"
    exit 1
}

$InstallerExeName = "Flip3D-Setup-v$Version-x64.exe"
$InstallerExePath = Join-Path $DistDir $InstallerExeName

if (-not (Test-Path $InstallerExePath)) {
    Write-Error "Installer executable not found at: $InstallerExePath"
    exit 1
}
Write-Host "Created Installer EXE: $InstallerExePath" -ForegroundColor Green

# Sign installer if requested
if ($Sign -and $PfxPath -and (Test-Path $PfxPath) -and $Signtool) {
    Write-Host "Signing installer executable..." -ForegroundColor Green
    & $Signtool sign /f $PfxPath /p $PfxPassword /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 $InstallerExePath
}

# 7. Generate Checksums (SHA256SUMS.txt)
Write-Host "`nComputing SHA256 Checksums..." -ForegroundColor Cyan
$ZipHash = (Get-FileHash -Path $PortableZipPath -Algorithm SHA256).Hash.ToLower()
$InstallerHash = (Get-FileHash -Path $InstallerExePath -Algorithm SHA256).Hash.ToLower()

$ChecksumFile = Join-Path $DistDir "SHA256SUMS.txt"
$ChecksumContent = "$ZipHash  $PortableZipName`n$InstallerHash  $InstallerExeName`n"
[System.IO.File]::WriteAllText($ChecksumFile, $ChecksumContent, [System.Text.Encoding]::UTF8)
Write-Host "Created SHA256SUMS.txt: $ChecksumFile" -ForegroundColor Green

# 8. Summary
Write-Host "`n==========================================================" -ForegroundColor Green
Write-Host "  BUILD & PACKAGING COMPLETE                              " -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green

$ZipItem = Get-Item $PortableZipPath
$InstallerItem = Get-Item $InstallerExePath
$SumsItem = Get-Item $ChecksumFile

$Summary = @(
    [PSCustomObject]@{
        "Artifact" = $ZipItem.Name
        "Size (MB)" = [math]::Round($ZipItem.Length / 1MB, 2)
        "SHA-256" = $ZipHash
    },
    [PSCustomObject]@{
        "Artifact" = $InstallerItem.Name
        "Size (MB)" = [math]::Round($InstallerItem.Length / 1MB, 2)
        "SHA-256" = $InstallerHash
    },
    [PSCustomObject]@{
        "Artifact" = $SumsItem.Name
        "Size (MB)" = [math]::Round($SumsItem.Length / 1KB, 2).ToString() + " KB"
        "SHA-256" = "-"
    }
)

$Summary | Format-Table -AutoSize
