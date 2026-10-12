# Registers the Leash scheduled task from the XML in this repository, shows what
# Task Scheduler made of it, and starts Leash through the task to prove that it
# runs elevated without a prompt.
#
# Run it from an administrator shell. A task with the highest available rights
# cannot be registered without the rights, so this cannot be tested from a
# normal one.
#
#   powershell -ExecutionPolicy Bypass -File scripts\task-test.ps1
#
# The task points at the release build, so build first:
#
#   pnpm tauri build
#
# The task is left in place at the end, which is the point of it: Leash will
# start at the next logon. To remove it:
#
#   schtasks /Delete /TN Leash /F

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$source = Join-Path $root "src-tauri\installer\leash-task.xml"
$exe = Join-Path $root "target\release\leash.exe"
$written = Join-Path $env:TEMP "leash-task.xml"
$taskName = "Leash"

if (-not (Test-Path $exe)) {
    throw "No release build at $exe. Run 'pnpm tauri build' first."
}

# The installer does the same substitution with the folder the user chose, so the
# task here names the build in this repository.
$xml = Get-Content -Raw $source
$xml = $xml.Replace("__LEASH_EXE__", $exe).Replace("__LEASH_DIR__", (Split-Path -Parent $exe))

# Written without a byte order mark. Task Scheduler refuses a UTF-8 mark
# outright, and it refuses an XML declaration that disagrees with the bytes, so
# the file carries no declaration either.
[System.IO.File]::WriteAllText($written, $xml, (New-Object System.Text.UTF8Encoding($false)))

Write-Host "Registering $taskName from $written"
schtasks /Create /TN $taskName /XML $written /F | Out-Host
if ($LASTEXITCODE -ne 0) {
    throw "schtasks refused the XML. The message above says why."
}

# Read the task back. This is the part that matters: it shows what Task
# Scheduler stored, rather than what the file asked for.
$stored = schtasks /Query /TN $taskName /XML
Write-Host ""
Write-Host "What Task Scheduler stored:"
foreach ($field in @(
    "RunLevel",
    "LogonType",
    "ExecutionTimeLimit",
    "MultipleInstancesPolicy",
    "DisallowStartIfOnBatteries",
    "StopIfGoingOnBatteries",
    "Command"
)) {
    $line = $stored | Select-String -Pattern "<$field>" | Select-Object -First 1
    if ($line) {
        Write-Host ("  " + $line.Line.Trim())
    } else {
        Write-Host "  $field : not in the stored task"
    }
}

# The logon trigger comes back as an empty element, so it is matched without a
# closing bracket. An empty element is a trigger that fires.
$trigger = $stored | Select-String -Pattern "<LogonTrigger" | Select-Object -First 1
if ($trigger) {
    Write-Host "  <LogonTrigger /> at logon"
} else {
    Write-Host "  LogonTrigger : not in the stored task"
}

# The account is stored as a SID, which is not much use to read.
$sid = $stored | Select-String -Pattern "<UserId>([^<]+)</UserId>" | Select-Object -First 1
if ($sid) {
    $value = $sid.Matches[0].Groups[1].Value
    try {
        $name = (New-Object System.Security.Principal.SecurityIdentifier($value)).Translate([System.Security.Principal.NTAccount]).Value
    } catch {
        $name = "$value (could not be resolved)"
    }
    Write-Host "  <UserId> $name"
}

Write-Host ""
Write-Host "The same task in the words Task Scheduler uses:"
schtasks /Query /TN $taskName /V /FO LIST |
    Select-String -Pattern "Status|Logon Mode|Run As User|Schedule Type|Scheduled Task State|Idle Time" |
    ForEach-Object { "  " + ($_.Line -replace "\s+", " ").Trim() }

Write-Host ""
Write-Host "Starting it through the task. There should be no prompt."
schtasks /Run /TN $taskName | Out-Host

# What appears on screen depends on one setting, so say which one to expect.
$cfg = Join-Path $env:APPDATA "Leash\config.toml"
$minimized = $true
if (Test-Path $cfg) {
    $line = Get-Content $cfg | Select-String -Pattern "^\s*start_minimized\s*=\s*(\w+)" | Select-Object -First 1
    if ($line) { $minimized = $line.Matches[0].Groups[1].Value -eq "true" }
}

Write-Host ""
if ($minimized) {
    Write-Host "Start minimized to tray is on, so no window will appear. Look for the"
    Write-Host "tray icon, choose Open, and check that the left rail says the app runs"
    Write-Host "elevated. That is the whole check."
} else {
    Write-Host "Start minimized to tray is off, so the window should appear by itself,"
    Write-Host "with no prompt, and the left rail should say the app runs elevated."
}

Write-Host ""
Write-Host "Remove it when you are finished with: schtasks /Delete /TN $taskName /F"
