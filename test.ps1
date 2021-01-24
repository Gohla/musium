#!/usr/bin/env pwsh

param(
  [parameter(Position = 0, Mandatory = $true)][String]$Command,
  [switch]$DebugProfile,
  [switch]$NoStopServerBefore,
  [switch]$StopServerAfter,
  [switch]$NoBuild,
  [Parameter(Position = 1, ValueFromRemainingArguments)][String]$RemainingArgs
)

$StopServerBefore = $NoStopServerBefore
$Build = !$NoBuild

$Env:SQLITE_MAX_VARIABLE_NUMBER = 1000000

$FastDevPath = Join-Path $PSScriptRoot "target\fastdev"
$DebugPath = Join-Path $PSScriptRoot "target\debug"
if($DebugProfile) {
  $ExeDir = $DebugPath
} else {
  $ExeDir = $FastDevPath
}
if($IsWindows) {
  $ExeSuffix = ".exe"
} else {
  $ExeSuffix = ""
}
$ServerExePath = [System.IO.Path]::GetFullPath((Join-Path $ExeDir "musium_server$ExeSuffix"))
$CliExePath = [System.IO.Path]::GetFullPath((Join-Path $ExeDir "musium_cli$ExeSuffix"))
$GuiExePath = [System.IO.Path]::GetFullPath((Join-Path $ExeDir "musium_gui$ExeSuffix"))

function Start-Diesel-Cli {
  param(
    [parameter(mandatory = $false, position = 0, ValueFromRemainingArguments = $true)]$Args
  )
  diesel $Args
}

function Start-Build {
  param(
    [switch]$Server,
    [switch]$Cli,
    [switch]$Gui
  )
  if($Build) {
    $Params = @("build", "--quiet")
    if($Server) {
      Stop-Servers # Need to stop the servers before building, otherwise we cannot write to the server executable.
      $Params += "--package"
      $Params += "musium_server"
    }
    if($Cli) {
      $Params += "--package"
      $Params += "musium_cli"
    }
    if($Gui) {
      $Params += "--package"
      $Params += "musium_gui"
    }
    if(!$DebugProfile) {
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
  if($StopServerBefore) {
    Stop-Servers
  }
}
function Stop-Servers-After-If-Requested {
  if($StopServerAfter) {
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
  if((Get-Process | Where-Object { $_.Path -like $ServerExePath }).Count -eq 0) {
    Start-Server
  }
}
function Start-Cli {
  Start-Process -FilePath $CliExePath -WorkingDirectory $PSScriptRoot -ArgumentList $args -NoNewWindow -Wait
}
function Start-Gui {
  Start-Process -FilePath $GuiExePath -WorkingDirectory $PSScriptRoot -NoNewWindow -Wait
}


function Before {
  param(
    [parameter(mandatory = $false, position = 0, ValueFromRemainingArguments = $true)]$Args
  )
  Start-Build $Args
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

function Test-Gui {
  Before -Server -Gui
  Start-Gui
  After
}

Invoke-Expression "Test-$Command"
