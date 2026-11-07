<#
.SYNOPSIS
  Plays and switches the two bundled tracks through AppFunctions, with no app Activity.
.EXAMPLE
  .\test-music.ps1
#>
param([int]$ListenSeconds = 3)
$ErrorActionPreference = 'Stop'
$adbCommand = Get-Command adb -ErrorAction SilentlyContinue
$adb = if ($adbCommand) { $adbCommand.Source } else { "$env:LOCALAPPDATA\Android\Sdk\platform-tools\adb.exe" }
if (-not (Test-Path $adb)) { throw 'Android SDK adb not found' }
$package = 'com.tauri.dev'
$service = 'com.plugin.google_app_functions.TauriAppFunctionService'
$localScript = Join-Path $env:TEMP ("music-test-" + [guid]::NewGuid().ToString('N') + '.sh')
$remoteScript = '/data/local/tmp/' + [IO.Path]::GetFileName($localScript)

function Adb-Run {
  $ErrorActionPreference = 'Continue'
  $output = & $script:adb @args 2>&1
  if ($LASTEXITCODE -ne 0) { throw ($output -join "`n") }
  return ($output -join "`n")
}
function Music([string]$Name) {
  $command = "cmd app_function execute-app-function --package $package --function '$service#$Name' --parameters '{}'"
  [IO.File]::WriteAllText($localScript, $command, (New-Object Text.UTF8Encoding $false))
  [void](Adb-Run push $localScript $remoteScript)
  $response = (Adb-Run shell sh $remoteScript) | ConvertFrom-Json
  $value = $response.androidAppfunctionsReturnValue
  if ($value -is [array]) { $value = $value[0] }
  if ($null -eq $value) { throw "$Name returned no playback state" }
  $state = [pscustomobject]@{
    trackId = [int]$value.trackId[0]
    title = [string]$value.title[0]
    playing = [bool]$value.playing[0]
  }
  Write-Host "$Name -> $($state.title), playing=$($state.playing)"
  return $state
}
function Assert-That([bool]$Condition, [string]$Message) {
  if (-not $Condition) { throw $Message }
  Write-Host "[ok] $Message" -ForegroundColor Green
}
function Wait-Playback([scriptblock]$Matches) {
  for ($attempt = 0; $attempt -lt 15; $attempt++) {
    $state = Music playbackState
    if (& $Matches $state) { return $state }
    Start-Sleep -Milliseconds 100
  }
  throw 'Media session did not reach the expected state'
}
try {
  [void](Adb-Run shell am force-stop $package)
  $first = Music playMusic
  Assert-That $first.playing 'Playback starts with the app closed'
  Start-Sleep -Seconds $ListenSeconds
  $second = Music nextTrack
  Assert-That ($second.playing -and $second.trackId -ne $first.trackId) 'Next changes the audible track'
  Start-Sleep -Seconds $ListenSeconds
  $back = Music previousTrack
  Assert-That ($back.playing -and $back.trackId -eq $first.trackId) 'Previous returns to the first track'
  Start-Sleep -Seconds $ListenSeconds
  $sessions = Adb-Run shell cmd media_session list-sessions
  Assert-That ($sessions -match 'tag=Small Hours, package=com.tauri.dev') 'System media session is registered'
  [void](Adb-Run shell cmd media_session dispatch next)
  $mediaNext = Wait-Playback { param($state) $state.trackId -ne $first.trackId -and $state.playing }
  Assert-That ($mediaNext.trackId -ne $first.trackId) 'System Next switches tracks without the UI'
  [void](Adb-Run shell cmd media_session dispatch previous)
  $mediaPrevious = Wait-Playback { param($state) $state.trackId -eq $first.trackId -and $state.playing }
  Assert-That ($mediaPrevious.trackId -eq $first.trackId) 'System Previous switches back'
  [void](Adb-Run shell cmd media_session dispatch pause)
  $mediaPaused = Wait-Playback { param($state) -not $state.playing }
  Assert-That (-not $mediaPaused.playing) 'System Pause pauses the player'
  [void](Adb-Run shell cmd media_session dispatch play)
  $mediaPlaying = Wait-Playback { param($state) $state.playing }
  Assert-That $mediaPlaying.playing 'System Play resumes the paused session'
  $paused = Music pauseMusic
  Assert-That (-not $paused.playing) 'Pause stops playback'
  $state = Music playbackState
  Assert-That ($state.trackId -eq $first.trackId -and -not $state.playing) 'State matches the player'
  $activities = Adb-Run shell dumpsys activity activities
  Assert-That (-not ($activities -match '(?m)^.*ActivityRecord\{.*com\.tauri\.dev/')) 'No app Activity was opened'
  Write-Host 'Music checks passed.' -ForegroundColor Green
} finally {
  # Also stop playback if a check fails halfway through.
  try { $null = Music pauseMusic } catch { Write-Warning $_ }
  if (Test-Path $localScript) { Remove-Item -LiteralPath $localScript }
  & $adb shell rm -f $remoteScript | Out-Null
}
