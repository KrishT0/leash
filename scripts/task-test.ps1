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
    "LogonTrigger",
    "ExecutionTimeLimit",
    "MultipleInstancesPolicy",
    "DisallowStartIfOnBatteries",
    "StopIfGoingOnBatteries",
    "Command",
    "ScheduledTaskState"
)) {
    $line = $stored | Select-String -Pattern "<$field>" | Select-Object -First 1
    if ($line) {
        Write-Host ("  " + $line.Line.Trim())
    } else {
        Write-Host "  $field : not in the stored task"
    }
}

Write-Host ""
Write-Host "Starting it through the task. The window should appear with no prompt,"
Write-Host "and the left rail should say the app is running elevated."
schtasks /Run /TN $taskName | Out-Host

Write-Host ""
Write-Host "Remove it when you are finished with: schtasks /Delete /TN $taskName /F"
