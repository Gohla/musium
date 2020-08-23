#!/usr/bin/env pwsh

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

$Env:SQLITE_MAX_VARIABLE_NUMBER = 1000000

$FastDevPath = Join-Path $PSScriptRoot "target\fastdev"
$DebugPath = Join-Path $PSScriptRoot "target\debug"
if($DebugProfile -eq $true) {
  $ExeDir = $DebugPath
} else {
  $ExeDir = $FastDevPath
}
if($IsWindows -eq $true) {
  $ExeSuffix = ".exe"
} else {
  $ExeSuffix = ""
}
$ServerExePath = [System.IO.Path]::GetFullPath((Join-Path $ExeDir "musium_server$ExeSuffix"))
$CliExePath = [System.IO.Path]::GetFullPath((Join-Path $ExeDir "musium_cli$ExeSuffix"))
$TuiExePath = [System.IO.Path]::GetFullPath((Join-Path $ExeDir "musium_tui$ExeSuffix"))

function Start-Diesel-Cli {
  diesel $args
}

function Start-Build {
  param(
    [switch]$Server,
    [switch]$Cli,
    [switch]$Tui
  )
  if($Build -eq $true) {
    $Params = @("build")
    if($Server.IsPresent) {
      Stop-Servers # Need to stop the servers before building, otherwise we cannot write to the server executable.
      $Params += "--package"
      $Params += "musium_server"
    }
    if($Cli.IsPresent) {
      $Params += "--package"
      $Params += "musium_cli"
    }
    if($Tui.IsPresent) {
      $Params += "--package"
      $Params += "musium_tui"
    }
    if($DebugProfile -eq $false) {
      $Params += "-Z"
      $Params += "unstable-options"
      $Params += "--profile"
      $Params += "fastdev"
    }
    cargo @Params
  }
}

function Stop-Servers {
  Get-Process | Where-Object {
    $_.Path -like $ServerExePath
  } | Stop-Process
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
function Start-ServerWait {
  Start-Process -FilePath $ServerExePath -WorkingDirectory $PSScriptRoot -NoNewWindow -Wait
}
function Start-Server-If-Not-Running {
  $ServerIsRunning = (Get-Process | Where-Object {
    $_.Path -like $ServerExePath
  }).Count -gt 0
  if($ServerIsRunning -eq $false) {
    Start-Server
  }
}
function Start-Cli {
  Start-Process -FilePath $CliExePath -WorkingDirectory $PSScriptRoot -ArgumentList $args -NoNewWindow -Wait
}
function Start-Tui {
  Start-Process -FilePath $TuiExePath -WorkingDirectory $PSScriptRoot -NoNewWindow -Wait
}


function Before {
  Start-Build @args
  Stop-Servers-Before-If-Requested
  Start-Server-If-Not-Running
  Start-Sleep -m 100 # Sleep to give the server some time to start up
}
function After {
  Stop-Servers-After-If-Requested
}

# Start server if it is not running, or restart it if requested
Function Test-StartServer {
  Before -Server
  Start-Server-If-Not-Running
  After
}
function Test-StopServer {
  # No Before, as building is not required (and may fail because the server could be running)
  Stop-Servers
  After
}

Function Test-Server {
  Before -Server
  Start-ServerWait
  After
}

function Test-Reset {
  Start-Diesel-Cli migration redo --migration-dir backend/migrations/
  Before -Server -Cli
  Start-Cli create-spotify-source
  After
}

function Test-Sync {
  Before -Server -Cli
  Start-Cli sync
  # No After, should not stop server when syncing, as syncing does not wait for syncing to be finished
}
function Test-ListLocalSources {
  Before -Server -Cli
  Start-Cli "list-local-sources"
  After
}
function Test-CreateSpotifySource {
  Before -Server -Cli
  Start-Cli "create-spotify-source"
  After
}
function Test-ListTracks {
  Before -Server -Cli
  Start-Cli "list-tracks"
  After
}
function Test-Play {
  Before -Server -Cli
  Start-Cli "play-track" $RemainingArgs
  After
}

function Test-Tui {
  Start-Build -Tui
  Start-Tui
}

Invoke-Expression "Test-$Command"
