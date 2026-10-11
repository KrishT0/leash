# Proves the stop path against a dummy service.
#
# Run this in an elevated shell:
#     powershell -ExecutionPolicy Bypass -File scripts\elevated-test.ps1
#
# The script creates LeashTestSvc, points it at the SNMP trap binary, which is a
# real service host that accepts a stop command, starts it, runs the ignored
# tests, and removes the service again.
#
# It never starts, stops, or changes any other service. The service it creates
# is separate from the SNMPTRAP service, even though both use the same binary.

$ErrorActionPreference = 'Stop'

$service = 'LeashTestSvc'
$hostExe = Join-Path $env:SystemRoot 'System32\snmptrap.exe'

function Fail([string] $message) {
    Write-Host "FAIL: $message" -ForegroundColor Red
    exit 1
}

$identity = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $identity.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host 'This script needs an elevated shell.' -ForegroundColor Yellow
    Write-Host 'Open PowerShell as administrator, then run:'
    Write-Host "  powershell -ExecutionPolicy Bypass -File `"$PSCommandPath`""
    exit 2
}

if (-not (Test-Path $hostExe)) {
    Fail "the service host $hostExe is missing. Write the fixture service crate instead."
}

# A service left over from an earlier run would block the create below.
if (Get-Service -Name $service -ErrorAction SilentlyContinue) {
    & sc.exe delete $service | Out-Null
    Start-Sleep -Milliseconds 500
}

try {
    & sc.exe create $service binPath= $hostExe start= demand type= own | Out-Null
    if ($LASTEXITCODE -ne 0) { Fail "cannot create $service" }
    Write-Host "created $service" -ForegroundColor Cyan

    & sc.exe start $service | Out-Null
    $state = $null
    for ($try = 0; $try -lt 40; $try++) {
        Start-Sleep -Milliseconds 250
        $state = (Get-Service -Name $service -ErrorAction SilentlyContinue).Status
        if ($state -eq 'Running') { break }
    }
    if ($state -ne 'Running') {
        Fail "$service did not reach Running, so the host binary cannot host a service here. Write the fixture service crate instead."
    }
    Write-Host "$service is Running" -ForegroundColor Green

    # The stop test needs the service running, so it is left alone here.
    $results = @()
    foreach ($filter in @(
        'should_stop_the_test_service',
        'should_give_up_after_the_cap_on_a_service_that_keeps_coming_back'
    )) {
        Write-Host "`n--- $filter ---" -ForegroundColor Cyan
        & cargo test -p leash-engine $filter -- --ignored --nocapture
        $results += [pscustomobject]@{ Test = $filter; Passed = ($LASTEXITCODE -eq 0) }
    }

    Write-Host ''
    $results | Format-Table -AutoSize

    if ($results.Passed -contains $false) {
        Fail 'at least one elevated test failed'
    }
    Write-Host 'All elevated tests passed.' -ForegroundColor Green
}
finally {
    & sc.exe stop $service 2>$null | Out-Null
    Start-Sleep -Milliseconds 500
    & sc.exe delete $service 2>$null | Out-Null
    Write-Host "removed $service" -ForegroundColor Yellow
}
