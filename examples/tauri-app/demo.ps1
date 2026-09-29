<#
.SYNOPSIS
  Walks through the Tauri Todo app functions over adb: create, view, list and complete a task.

.EXAMPLE
  .\demo.ps1
  .\demo.ps1 -Title "Call mom" -Notes "Before 8 pm" -Interactive
#>
param(
  [string]$Title = "Buy milk",
  [string]$Notes = "2 liters, oat",
  # Wait for Enter between steps instead of a short pause (handy when recording).
  [switch]$Interactive,
  # No pauses at all.
  [switch]$Fast
)

$ErrorActionPreference = "Stop"
$Package = "com.tauri.dev"
$Service = "com.plugin.google_app_functions.TauriAppFunctionService"
$RemoteScript = "/data/local/tmp/tauri-todo-demo.sh"

$previousEncoding = [Console]::OutputEncoding
[Console]::OutputEncoding = [Text.Encoding]::UTF8

# ---------------------------------------------------------------- output helpers

function Write-Banner {
  Write-Host ""
  Write-Host "   Tauri " -ForegroundColor Cyan -NoNewline
  Write-Host "--( " -ForegroundColor DarkGray -NoNewline
  Write-Host "App Functions" -ForegroundColor White -NoNewline
  Write-Host " )-- " -ForegroundColor DarkGray -NoNewline
  Write-Host "Gemini" -ForegroundColor Blue
  Write-Host "   Tauri Todo demo over adb" -ForegroundColor DarkGray
  Write-Host ""
}

function Write-Step([int]$Number, [string]$Text) {
  Write-Host ""
  Write-Host (" {0} " -f $Number) -ForegroundColor Black -BackgroundColor Cyan -NoNewline
  Write-Host " $Text" -ForegroundColor White
}

function Write-Ok([string]$Text) { Write-Host "   [ok] " -ForegroundColor Green -NoNewline; Write-Host $Text }
function Write-Info([string]$Text) { Write-Host "        $Text" -ForegroundColor DarkGray }
function Write-Fail([string]$Text) { Write-Host "   [!!] " -ForegroundColor Red -NoNewline; Write-Host $Text -ForegroundColor Red }

function Stop-Demo([string]$Text, [string]$Hint) {
  Write-Fail $Text
  if ($Hint) { Write-Info $Hint }
  Write-Host ""
  [Console]::OutputEncoding = $previousEncoding
  exit 1
}

function Wait-Step {
  if ($Fast) { return }
  if ($Interactive) {
    Write-Host "        press Enter to continue" -ForegroundColor DarkGray -NoNewline
    [void](Read-Host)
  } else {
    Start-Sleep -Milliseconds 1500
  }
}

function Write-Task($Task) {
  $mark = if ($Task.done) { "(x)" } else { "( )" }
  $color = if ($Task.done) { "DarkGray" } else { "White" }
  Write-Host ("   {0} " -f $mark) -ForegroundColor Cyan -NoNewline
  Write-Host ("#{0,-3} " -f $Task.id) -ForegroundColor DarkGray -NoNewline
  Write-Host $Task.title -ForegroundColor $color -NoNewline
  if ($Task.fromAgent) { Write-Host "  * Gemini" -ForegroundColor Blue -NoNewline }
  Write-Host ""
  if ($Task.notes) { Write-Host ("             {0}" -f $Task.notes) -ForegroundColor DarkGray }
}

# ---------------------------------------------------------------- adb

function Find-Adb {
  $command = Get-Command adb -ErrorAction SilentlyContinue
  if ($command) { return $command.Source }
  foreach ($sdk in @($env:ANDROID_HOME, $env:ANDROID_SDK_ROOT, "$env:LOCALAPPDATA\Android\Sdk")) {
    if ($sdk -and (Test-Path "$sdk\platform-tools\adb.exe")) { return "$sdk\platform-tools\adb.exe" }
  }
  return $null
}

function Invoke-Adb {
  # adb reports progress on stderr; don't let that stop the script.
  $ErrorActionPreference = "Continue"
  $output = & $script:Adb @args 2>&1
  return ($output | ForEach-Object { "$_" }) -join "`n"
}

# PowerShell 5.1 mangles quotes in native arguments, so the command goes to the device as a file.
function Invoke-DeviceShell([string]$Command) {
  $local = Join-Path $env:TEMP "tauri-todo-demo.sh"
  [IO.File]::WriteAllText($local, $Command, (New-Object Text.UTF8Encoding $false))
  [void](Invoke-Adb push $local $RemoteScript)
  return Invoke-Adb shell sh $RemoteScript
}

$ErrorNames = @{
  1001 = "InvalidArgument"; 1002 = "Disabled"; 1003 = "FunctionNotFound"
  1500 = "ElementNotFound"; 1501 = "LimitExceeded"; 1502 = "ElementAlreadyExists"
  2001 = "Cancelled"; 3000 = "Unknown"; 3500 = "PermissionRequired"; 3501 = "NotSupported"
}

# Documents hold every value as an array; unwrap single values so results read like JSON.
function ConvertFrom-Document($Value, [bool]$IsList = $false) {
  if ($Value -is [System.Management.Automation.PSCustomObject]) {
    $result = [ordered]@{}
    foreach ($property in $Value.PSObject.Properties) {
      $result[$property.Name] = ConvertFrom-Document $property.Value
    }
    return [pscustomobject]$result
  }
  if ($Value -is [array] -and -not $IsList) {
    if ($Value.Count -eq 1) { return ConvertFrom-Document $Value[0] }
    return , @($Value | ForEach-Object { ConvertFrom-Document $_ })
  }
  return $Value
}

function Invoke-AppFunction([string]$Name, $Parameters, [switch]$ReturnsList) {
  $json = ($Parameters | ConvertTo-Json -Compress) -replace "'", "'\''"
  Write-Info "$Name $json"
  $output = Invoke-DeviceShell "cmd app_function execute-app-function --package $Package --function '$Service#$Name' --parameters '$json'"

  if ($output -match "AppFunctionException: (.*) \(code (\d+)\)") {
    $code = [int]$Matches[2]
    $kind = $ErrorNames[$code]
    if (-not $kind) { $kind = "Error" }
    return [pscustomobject]@{ Ok = $false; Error = "$kind ($code): $($Matches[1])" }
  }
  if ($output -match "No shell command implementation") {
    Stop-Demo "This Android build has no 'cmd app_function'." "Use an Android 16 QPR or Android 17 emulator image."
  }
  try {
    $document = $output | ConvertFrom-Json
  } catch {
    return [pscustomobject]@{ Ok = $false; Error = $output }
  }
  $value = $null
  if ($document.PSObject.Properties.Name -contains "androidAppfunctionsReturnValue") {
    $raw = $document.androidAppfunctionsReturnValue
    if ($ReturnsList) {
      $value = @($raw | ForEach-Object { ConvertFrom-Document $_ })
    } else {
      $value = ConvertFrom-Document $raw
    }
  }
  return [pscustomobject]@{ Ok = $true; Value = $value }
}

# ---------------------------------------------------------------- demo

Write-Banner

Write-Step 1 "Checking the device"
$script:Adb = Find-Adb
if (-not $script:Adb) {
  Stop-Demo "adb not found." "Install Android SDK platform-tools or add adb to PATH."
}
Write-Ok "adb: $script:Adb"

$devices = (Invoke-Adb devices) -split "`n" | Where-Object { $_ -match "\tdevice$" }
if (-not $devices) { Stop-Demo "No device or emulator connected." "Start an emulator and try again." }
Write-Ok ("device: " + (($devices | Select-Object -First 1) -split "\t")[0])

$sdk = [int](Invoke-Adb shell getprop ro.build.version.sdk)
if ($sdk -lt 36) { Stop-Demo "Android API ${sdk}: App Functions need Android 16 (API 36) or newer." }
Write-Ok "Android API $sdk"

if (-not ((Invoke-Adb shell pm list packages $Package) -match "package:$Package")) {
  Stop-Demo "Tauri Todo ($Package) is not installed." "Run: npm run tauri android dev"
}
Write-Ok "Tauri Todo is installed"

if (-not ((Invoke-Adb shell dumpsys app_function) -match [regex]::Escape("$Service#createTask"))) {
  Stop-Demo "Android has not indexed the app functions." "Reinstall the app; early Android 16 builds never index them."
}
Write-Ok "app functions are indexed"
Wait-Step

Write-Step 2 "Opening Tauri Todo"
[void](Invoke-Adb shell monkey -p $Package -c android.intent.category.LAUNCHER 1)
Write-Ok "launched; the functions below need the app open"
Wait-Step

Write-Step 3 "createTask: an agent adds a task"
$created = $null
for ($attempt = 1; $attempt -le 10; $attempt++) {
  $created = Invoke-AppFunction "createTask" ([ordered]@{ title = $Title; notes = $Notes })
  if ($created.Ok -or $created.Error -notmatch "not running") { break }
  Start-Sleep -Seconds 1
}
if (-not $created.Ok) { Stop-Demo $created.Error }
$task = $created.Value
Write-Task $task
Write-Ok "look at the phone: the task appeared with a Gemini badge"
Wait-Step

Write-Step 4 "getTask: read it back by ID"
$fetched = Invoke-AppFunction "getTask" ([ordered]@{ id = $task.id })
if (-not $fetched.Ok) { Stop-Demo $fetched.Error }
Write-Task $fetched.Value
Wait-Step

Write-Step 5 "listTasks: what is still to do"
$open = Invoke-AppFunction "listTasks" ([ordered]@{ includeDone = $false }) -ReturnsList
if (-not $open.Ok) { Stop-Demo $open.Error }
$open.Value | ForEach-Object { Write-Task $_ }
Wait-Step

Write-Step 6 "completeTask: tick it off"
$completed = Invoke-AppFunction "completeTask" ([ordered]@{ id = $task.id })
if (-not $completed.Ok) { Stop-Demo $completed.Error }
Write-Task $completed.Value
Wait-Step

Write-Step 7 "getTask with a wrong ID: errors come back typed"
$missing = Invoke-AppFunction "getTask" ([ordered]@{ id = 999999 })
if ($missing.Ok) { Stop-Demo "expected an error for a missing task" }
Write-Ok $missing.Error

Write-Host ""
Write-Host "   All app functions answered. " -ForegroundColor Green -NoNewline
Write-Host "Now try Gemini: " -ForegroundColor DarkGray -NoNewline
Write-Host "`"Add buy milk to my list in Tauri Todo`"" -ForegroundColor White
Write-Host ""

[Console]::OutputEncoding = $previousEncoding
