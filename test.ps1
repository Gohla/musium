param(
  [parameter(Position=0, Mandatory=$true)][String]$Command,
  [switch]$DebugExe = $false,
  [switch]$RestartServer = $false,
  [switch]$StopServer = $false,
  [Parameter(Position=1, ValueFromRemainingArguments)][String]$RemainingArgs
)

$FastDevPath = Join-Path $PSScriptRoot "target\fastdev"
$DebugPath = Join-Path $PSScriptRoot "target\debug"
if($DebugExe -eq $true) {
  $ExeDir = $DebugPath
} else {
  $ExeDir = $FastDevPath
}
$ServerExePath = [System.IO.Path]::GetFullPath((Join-Path $ExeDir "musium_server.exe"))
$CliExePath = [System.IO.Path]::GetFullPath((Join-Path $ExeDir "musium_cli.exe"))

function Stop-Servers {
  Get-Process | Where-Object { $_.Path -like $ServerExePath } | Stop-Process
}
function Stop-Servers-If-Requested {
  if($StopServer -eq $true) {
    Stop-Servers
  }
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
}

function Test-ListLocalSources {
  Test-Start
  Start-Cli "list-local-sources"
  Stop-Servers-If-Requested
}

function Test-CreateSpotifySource {
  Test-Start
  Start-Cli "create-spotify-source"
  Stop-Servers-If-Requested
}

function Test-Play {
  Test-Start
  Start-Cli "play-track" $RemainingArgs
  Stop-Servers-If-Requested
}

function Test-Stop {
  Stop-Servers
}

Invoke-Expression "Test-$Command"
