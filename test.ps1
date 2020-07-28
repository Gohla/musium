param(
  [parameter(Position=0, Mandatory=$true)][String]$Command,
  [switch]$RestartServer = $false,
  [switch]$StopServer = $false,
  [Parameter(Position=1, ValueFromRemainingArguments)][String]$RemainingArgs
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
function Start-Cli {
  Start-Process -FilePath $CliExePath -WorkingDirectory $PSScriptRoot -ArgumentList $args -NoNewWindow -Wait
}

# Start server if it is not running, or restart it if requested
Function Test-Start {
  $ServerIsRunning = (Get-Process | Where-Object { $_.Path -like $ServerExePath }).Count -gt 0
  if($ServerIsRunning -eq $true -and $RestartServer) {
    Stop-Servers
    Start-Server
  } elseif($ServerIsRunning -eq $false) {
    Start-Server
  }
}

function Test-Sync {
  Test-Start
  Start-Cli sync
  if($StopServer -eq $true) {
    Stop-Servers
  }
}

function Test-CreateSpotifySource {
  Test-Start
  Start-Cli "create-spotify-source"
}

function Test-Play {
  Test-Start
  Start-Cli "play-track" $RemainingArgs
}

function Test-Stop {
  Stop-Servers
}

Invoke-Expression "Test-$Command"
