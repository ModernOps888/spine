Write-Host "Stopping SPINE services..." -ForegroundColor Yellow

# Kill backend processes
Get-CimInstance Win32_Process | Where-Object { $_.Name -like "*spine*" -and $_.CommandLine -notlike "*stop*" } | ForEach-Object {
    Write-Host "Stopping $($_.Name) (PID: $($_.ProcessId))" -ForegroundColor Cyan
    Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
}

# Kill frontend on port 3333
$port3333 = Get-NetTCPConnection -LocalPort 3333 -State Listen -ErrorAction SilentlyContinue
if ($port3333) {
    Write-Host "Stopping frontend on port 3333 (PID: $($port3333.OwningProcess))" -ForegroundColor Cyan
    Stop-Process -Id $port3333.OwningProcess -Force -ErrorAction SilentlyContinue
}

Write-Host "SPINE services stopped." -ForegroundColor Green
