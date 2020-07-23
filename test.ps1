param(
  [switch]$RestartServer = $false,
  [switch]$StopServer = $false
)

$FastDevPath = Join-Path $PSScriptRoot "target\fastdev"
$ServerExePath = [System.IO.Path]::GetFullPath((Join-Path $FastDevPath "musium_server.exe"))
$CliExePath = [System.IO.Path]::GetFullPath((Join-Path $FastDevPath "musium_cli.exe"))

function Stop-Servers {
  Get-Process | Where-Object { $_.Path -like $ServerExePath } | Stop-Process
}
function Start-Server {
  Start-Process -FilePath $ServerExePath -WorkingDirectory $PSScriptRoot -NoNewWindow
}

# Start server if it is not running, or restart it if requested
$ServerIsRunning = (Get-Process | Where-Object { $_.Path -like $ServerExePath }).Count -gt 0
if($ServerIsRunning -eq $true -and $RestartServer) {
  Stop-Servers
  Start-Server
} elseif($ServerIsRunning -eq $false) {
  Start-Server
}

# Run CLI
Start-Process -FilePath $CliExePath -WorkingDirectory $PSScriptRoot -ArgumentList "sync" -NoNewWindow -Wait

# Stop server if requested
if($StopServer -eq $true) {
  Stop-Servers
}
