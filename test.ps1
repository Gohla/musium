param(
  [parameter(Position = 0, Mandatory = $true)][String]$Command,
  [switch]$DebugProfile,
  [switch]$NoStopServerBefore,
  [switch]$StopServerAfter,
  [switch]$NoBuild,
  [Parameter(Position = 1, ValueFromRemainingArguments)][String]$RemainingArgs
)

if($NoStopServerBefore.IsPresent) {
  $StopServerBefore = $false
} else {
  $StopServerBefore = $true
}

if($NoBuild.IsPresent) {
  $Build = $false
} else {
  $Build = $true
}

$env:SQLITE_MAX_VARIABLE_NUMBER = 1000000

$FastDevPath = Join-Path $PSScriptRoot "target\fastdev"
$DebugPath = Join-Path $PSScriptRoot "target\debug"
if($DebugProfile -eq $true) {
  $ExeDir = $DebugPath
} else {
  $ExeDir = $FastDevPath
}
$ServerExePath = [System.IO.Path]::GetFullPath((Join-Path $ExeDir "musium_server.exe"))
$CliExePath = [System.IO.Path]::GetFullPath((Join-Path $ExeDir "musium_cli.exe"))

function Start-Diesel-Cli {
  diesel $args
}

function Start-Build {
  if($Build -eq $true) {
    Stop-Servers # Need to stop the servers before building, otherwise we cannot write to the server executable.
    if($DebugProfile -eq $true) {
      cargo build --package musium_server --package musium_cli
    } else {
      cargo build --package musium_server --package musium_cli -Z unstable-options --profile fastdev
    }
  }
}

function Stop-Servers {
  Get-Process | Where-Object { $_.Path -like $ServerExePath } | Stop-Process
}
function Stop-Servers-Before-If-Requested {
  if($StopServerBefore -eq $true) {
    Stop-Servers
  }
}
function Stop-Servers-After-If-Requested {
  if($StopServerAfter -eq $true) {
    Stop-Servers
  }
}
function Start-Server {
  Start-Process -FilePath $ServerExePath -WorkingDirectory $PSScriptRoot -NoNewWindow
}
function Start-Server-If-Not-Running {
  $ServerIsRunning = (Get-Process | Where-Object { $_.Path -like $ServerExePath }).Count -gt 0
  if($ServerIsRunning -eq $false) {
    Start-Server
  }
}
function Start-Cli {
  Start-Process -FilePath $CliExePath -WorkingDirectory $PSScriptRoot -ArgumentList $args -NoNewWindow -Wait
}


function Before {
  Start-Build
  Stop-Servers-Before-If-Requested
  Start-Server-If-Not-Running
}
function After {
  Stop-Servers-After-If-Requested
}

# Start server if it is not running, or restart it if requested
Function Test-Start {
  Before
  Start-Server-If-Not-Running
  After
}
function Test-Stop {
  Before
  Stop-Servers
  After
}


function Test-Reset {
  Start-Diesel-Cli migration redo
  Before
  Start-Cli create-spotify-source
  After
}

function Test-Sync {
  Before
  Start-Cli sync
  # No After, should not stop server when syncing, as syncing does not wait for syncing to be finished
}

function Test-ListLocalSources {
  Before
  Start-Cli "list-local-sources"
  After
}

function Test-CreateSpotifySource {
  Before
  Start-Cli "create-spotify-source"
  After
}

function Test-Play {
  Before
  Start-Cli "play-track" $RemainingArgs
  After
}

Invoke-Expression "Test-$Command"
