Write-Host "Stopping SPINE services..." -ForegroundColor Yellow

# Kill gateway on port 8080
$port8080 = Get-NetTCPConnection -LocalPort 8080 -State Listen -ErrorAction SilentlyContinue
if ($port8080) {
    Write-Host "Stopping SPINE Gateway on port 8080 (PID: $($port8080.OwningProcess))" -ForegroundColor Cyan
    Stop-Process -Id $port8080.OwningProcess -Force -ErrorAction SilentlyContinue
}

# Kill backend daemon processes (excluding any MCP stdio worker invoked by IDE)
Get-CimInstance Win32_Process | Where-Object { 
    $_.Name -like "*spine-gateway*" -or 
    ($_.Name -like "*spine.exe*" -and $_.CommandLine -notmatch "\bmcp\b" -and $_.CommandLine -notlike "*stop*")
} | ForEach-Object {
    Write-Host "Stopping daemon $($_.Name) (PID: $($_.ProcessId))" -ForegroundColor Cyan
    Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
}

# Kill frontend on port 3333
$port3333 = Get-NetTCPConnection -LocalPort 3333 -State Listen -ErrorAction SilentlyContinue
if ($port3333) {
    Write-Host "Stopping frontend on port 3333 (PID: $($port3333.OwningProcess))" -ForegroundColor Cyan
    Stop-Process -Id $port3333.OwningProcess -Force -ErrorAction SilentlyContinue
}

Write-Host "SPINE services stopped." -ForegroundColor Green
