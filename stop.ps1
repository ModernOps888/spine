Write-Host "Stopping SPINE standalone services..." -ForegroundColor Yellow

# Identify any active IDE MCP stdio worker PIDs so we NEVER terminate an active IDE MCP connection
$mcpPids = @(Get-CimInstance Win32_Process | Where-Object {
    ($_.Name -like "*spine*.exe") -and ($_.CommandLine -match "\bmcp\b")
} | Select-Object -ExpandProperty ProcessId)

# Stop standalone gateway on port 8080 only if it is not an IDE MCP stdio process
$port8080 = Get-NetTCPConnection -LocalPort 8080 -State Listen -ErrorAction SilentlyContinue
foreach ($conn in $port8080) {
    if ($mcpPids -contains $conn.OwningProcess) {
        Write-Host "Preserving IDE MCP co-hosted gateway on port 8080 (PID: $($conn.OwningProcess))" -ForegroundColor DarkCyan
    } else {
        Write-Host "Stopping SPINE Gateway on port 8080 (PID: $($conn.OwningProcess))" -ForegroundColor Cyan
        Stop-Process -Id $conn.OwningProcess -Force -ErrorAction SilentlyContinue
    }
}

# Kill standalone backend daemon processes (excluding any MCP stdio worker invoked by IDE)
Get-CimInstance Win32_Process | Where-Object { 
    ($_.Name -like "*spine-gateway*" -or $_.Name -like "*spine.exe*") -and
    ($mcpPids -notcontains $_.ProcessId) -and
    ($_.CommandLine -notmatch "\bmcp\b") -and
    ($_.CommandLine -notlike "*stop*")
} | ForEach-Object {
    Write-Host "Stopping daemon $($_.Name) (PID: $($_.ProcessId))" -ForegroundColor Cyan
    Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
}

# Kill frontend on port 3333
$port3333 = Get-NetTCPConnection -LocalPort 3333 -State Listen -ErrorAction SilentlyContinue
foreach ($conn in $port3333) {
    Write-Host "Stopping frontend on port 3333 (PID: $($conn.OwningProcess))" -ForegroundColor Cyan
    Stop-Process -Id $conn.OwningProcess -Force -ErrorAction SilentlyContinue
}

Write-Host "SPINE standalone services stopped." -ForegroundColor Green
