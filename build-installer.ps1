# Build Script for Flip 3D (Vista Style) & Inno Setup Packaging
# Run: powershell -ExecutionPolicy Bypass -File .\build-installer.ps1

$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " 1. Building Release Binaries (Workspace) " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

cargo build --release --workspace

$EngineExe = "target\release\flip3d.exe"

if (-not (Test-Path $EngineExe)) {
    Write-Error "Build failed: $EngineExe not found!"
    exit 1
}

Write-Host "Single unified binary compiled successfully:" -ForegroundColor Green
Write-Host " - Standalone Executable: $EngineExe ($((Get-Item $EngineExe).Length / 1MB | ForEach-Object { '{0:N2} MB' -f $_ }))" -ForegroundColor Green
Write-Host "   (Houses both background switcher and WebView2 settings UI via --settings)" -ForegroundColor DarkGray

Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host " 2. Inno Setup Compiler Detection " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$IsccCandidatePaths = @(
    $env:ISCC_PATH,
    (Get-Command iscc.exe -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source),
    "C:\Program Files (x86)\Inno Setup 6\iscc.exe",
    "C:\Program Files\Inno Setup 6\iscc.exe",
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\iscc.exe"
) | Where-Object { $_ -and (Test-Path $_) }

$IsccPath = $IsccCandidatePaths | Select-Object -First 1

if ($IsccPath) {
    Write-Host "Found Inno Setup Compiler at: $IsccPath" -ForegroundColor Green
    New-Item -ItemType Directory -Force -Path "dist" | Out-Null
    
    Write-Host "Compiling installer via Inno Setup..." -ForegroundColor Yellow
    & $IsccPath "installer\setup.iss"
    
    if (Test-Path "dist\Setup.exe") {
        Write-Host "`nSetup package created successfully:" -ForegroundColor Green
        Write-Host " -> dist\Setup.exe ($((Get-Item "dist\Setup.exe").Length / 1MB | ForEach-Object { '{0:N2} MB' -f $_ }))" -ForegroundColor Green
    } else {
        Write-Error "Inno Setup compilation finished but dist\Setup.exe was not created."
        exit 1
    }
} else {
    Write-Host "`n[NOTE] Inno Setup 6 compiler (iscc.exe) was not found on this system." -ForegroundColor Yellow
    Write-Host "Release binaries are staged and ready in: target\release\" -ForegroundColor Yellow
    Write-Host "Once Inno Setup 6 is installed, you can compile the installer by either:" -ForegroundColor Yellow
    Write-Host "  1. Running: `$env:ISCC_PATH = 'C:\Path\To\iscc.exe'; .\build-installer.ps1" -ForegroundColor Yellow
    Write-Host "  2. Or opening 'installer\setup.iss' in Inno Setup Compiler and clicking Compile." -ForegroundColor Yellow
}
