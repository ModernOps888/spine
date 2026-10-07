[CmdletBinding()]
param(
    [switch]$Quiet
)

# SPINE Anti-Sycophancy Reality Gateway & HUD Launcher
# Starts the high-performance Rust Gateway on :8080 and React 19 HUD on :3333

if (-not $Quiet) {
    Write-Host "==========================================================" -ForegroundColor Cyan
    Write-Host " 🛡️ Starting SPINE Anti-Sycophancy Reality Gateway & HUD" -ForegroundColor Cyan
    Write-Host "==========================================================" -ForegroundColor Cyan
}

$baseDir = if ($PSScriptRoot) { $PSScriptRoot } else { "." }
$binDir = Join-Path $baseDir "bin"
$binPath = Join-Path $binDir "spine.exe"
$targetBin = Join-Path $baseDir "backend\target\debug\spine-gateway.exe"
$releaseBin = Join-Path $baseDir "backend\target\release\spine-gateway.exe"

if (-not (Test-Path $binDir)) {
    New-Item -ItemType Directory -Path $binDir -Force | Out-Null
}

# Resolve the best available binary
if (Test-Path $targetBin) {
    try {
        Copy-Item $targetBin $binPath -Force -ErrorAction SilentlyContinue
    } catch {}
} elseif (Test-Path $releaseBin) {
    try {
        Copy-Item $releaseBin $binPath -Force -ErrorAction SilentlyContinue
    } catch {}
} elseif (-not (Test-Path $binPath)) {
    if (-not $Quiet) { Write-Host "Compiling SPINE backend binary..." -ForegroundColor Yellow }
    cargo build --manifest-path (Join-Path $baseDir "backend\Cargo.toml")
    try {
        Copy-Item $targetBin $binPath -Force -ErrorAction SilentlyContinue
    } catch {}
}

$exeToRun = if (Test-Path $targetBin) { $targetBin } elseif (Test-Path $binPath) { $binPath } else { "spine.exe" }

# Ensure .env is available in root
$backendEnv = Join-Path $baseDir "backend\.env"
$rootEnv = Join-Path $baseDir ".env"
if ((Test-Path $backendEnv) -and (-not (Test-Path $rootEnv))) {
    Copy-Item $backendEnv $rootEnv -Force -ErrorAction SilentlyContinue
}

# Start Rust Gateway on port 8080 if not already listening
$port8080 = Get-NetTCPConnection -LocalPort 8080 -State Listen -ErrorAction SilentlyContinue
if (-not $port8080) {
    if (-not $Quiet) { Write-Host "`n[1/2] Launching Rust Reality Gateway on http://127.0.0.1:8080 ..." -ForegroundColor Green }
    $windowStyle = if ($Quiet) { "Hidden" } else { "Minimized" }
    Start-Process -FilePath $exeToRun -WorkingDirectory $baseDir -WindowStyle $windowStyle
    Start-Sleep -Seconds 2
} else {
    if (-not $Quiet) { Write-Host "`n[1/2] Rust Reality Gateway already listening on http://127.0.0.1:8080" -ForegroundColor Green }
}

# Start React 19 HUD on port 3333 if not already listening
$port3333 = Get-NetTCPConnection -LocalPort 3333 -State Listen -ErrorAction SilentlyContinue
if (-not $port3333) {
    if (-not $Quiet) { Write-Host "[2/2] Launching React 19 / TypeScript HUD on http://localhost:3333 ..." -ForegroundColor Green }
    $frontWindowStyle = if ($Quiet) { "Hidden" } else { "Minimized" }
    $frontDir = Join-Path $baseDir "frontend"
    Start-Process -FilePath "powershell.exe" -ArgumentList "-NoProfile", "-Command", "Set-Location '$frontDir'; npm run dev -- --host --port 3333" -WindowStyle $frontWindowStyle
    Start-Sleep -Seconds 2
} else {
    if (-not $Quiet) { Write-Host "[2/2] React 19 / TypeScript HUD already active on http://localhost:3333" -ForegroundColor Green }
}

if (-not $Quiet) {
    Start-Process "http://localhost:3333"
    Write-Host "`n[ACTIVE] SPINE HUD is open at http://localhost:3333" -ForegroundColor Cyan
    Write-Host "To stop services at any time, run: .\stop.bat`n" -ForegroundColor Yellow
}
