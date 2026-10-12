# Checks a Leash installation, after the installer has run.
#
# powershell -ExecutionPolicy Bypass -File scripts\install-check.ps1
#
# It reads what the installer left behind: the program in Program Files, the
# scheduled task, the Start menu shortcut and the uninstall entry. A normal shell is
# enough, because it only reads. The task's command has to point at the installed
# program, which is the only thing only the installer's hook can reach.

$ErrorActionPreference = "Continue"

$program = Join-Path $env:ProgramFiles "Leash"
$exe = Join-Path $program "leash.exe"
$taskXml = Join-Path $program "leash-task.xml"

Write-Host "The installed program"
Write-Host ("  {0}: {1}" -f $program, $(if (Test-Path $program) { "found" } else { "MISSING" }))
Write-Host ("  leash.exe: {0}" -f $(if (Test-Path $exe) { "found" } else { "MISSING" }))
Write-Host ("  leash-task.xml: {0}" -f $(if (Test-Path $taskXml) { "found" } else { "MISSING" }))

Write-Host ""
Write-Host "The scheduled task"
$stored = schtasks /Query /TN Leash /XML 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Host "  MISSING. The installer did not register it."
} else {
    $stored | Select-String -Pattern "<Command>|<RunLevel>|<ExecutionTimeLimit>|<LogonTrigger" |
        ForEach-Object { "  " + $_.Line.Trim() }

    $match = $stored | Select-String -Pattern "<Command>([^<]+)</Command>" | Select-Object -First 1
    $command = if ($match) { $match.Matches[0].Groups[1].Value } else { "" }
    if ($command -eq $exe) {
        Write-Host "  the task starts the installed program, so the installer hook ran"
    } else {
        Write-Host "  WARNING: the task starts $command instead of the installed program."
        Write-Host "  That is the task from an earlier test, not the one the installer writes."
    }
}

Write-Host ""
Write-Host "The shortcut and the uninstall entry"
$shortcut = Join-Path $env:ProgramData "Microsoft\Windows\Start Menu\Programs\Leash.lnk"
Write-Host ("  start menu shortcut: {0}" -f $(if (Test-Path $shortcut) { "found" } else { "MISSING" }))
$uninstall = Get-ItemProperty "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Leash" -ErrorAction SilentlyContinue
if ($uninstall) {
    Write-Host ("  uninstall entry: {0} version {1}" -f $uninstall.DisplayName, $uninstall.DisplayVersion)
} else {
    Write-Host "  uninstall entry: MISSING"
}

Write-Host ""
Write-Host "What to do next"
Write-Host "  Log off and back on. Leash should appear in the tray with no window and no"
Write-Host "  prompt. Open it from the tray, and the left rail should say it runs elevated."
Write-Host "  To undo the install, remove Leash from Installed apps."
