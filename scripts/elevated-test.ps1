# Proves the stop path against a dummy service.
#
# Run this in an elevated shell:
#     powershell -ExecutionPolicy Bypass -File scripts\elevated-test.ps1
#
# It creates LeashTestSvc, points it at the SNMP trap binary, a real service host that
# accepts a stop command, runs the ignored tests, and removes the service again. It
# changes nothing else, and its service is separate from the real SNMPTRAP one.

$ErrorActionPreference = 'Stop'

$service = 'LeashTestSvc'
$hostExe = Join-Path $env:SystemRoot 'System32\snmptrap.exe'
$probeSeconds = 30

function Fail([string] $message) {
    Write-Host "FAIL: $message" -ForegroundColor Red
    exit 1
}

# Ask the service manager to start the service and follow it to Running.
function Start-Dummy {
    & sc.exe start $service | Out-Null
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $last = $null
    while ($watch.Elapsed.TotalSeconds -lt $probeSeconds) {
        $state = (Get-Service -Name $service -ErrorAction SilentlyContinue).Status
        if ("$state" -ne "$last") {
            Write-Host ("  {0,6} ms: {1}" -f [int]$watch.Elapsed.TotalMilliseconds, $state)
            $last = "$state"
        }
        if ("$state" -eq 'Running') { return $true }
        Start-Sleep -Milliseconds 250
    }
    Write-Host ("  still {0} after {1} seconds" -f $last, $probeSeconds) -ForegroundColor Yellow
    return $false
}

# Ask the service manager to stop the service and follow it to Stopped.
function Stop-Dummy {
    & sc.exe stop $service | Out-Null
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $last = $null
    while ($watch.Elapsed.TotalSeconds -lt $probeSeconds) {
        $state = (Get-Service -Name $service -ErrorAction SilentlyContinue).Status
        if ("$state" -ne "$last") {
            Write-Host ("  {0,6} ms: {1}" -f [int]$watch.Elapsed.TotalMilliseconds, $state)
            $last = "$state"
        }
        if ("$state" -eq 'Stopped') { return $true }
        Start-Sleep -Milliseconds 250
    }
    Write-Host ("  still {0} after {1} seconds" -f $last, $probeSeconds) -ForegroundColor Yellow
    return $false
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

    # The failure of the last run left one question open: does this binary reach
    # Running a second time? The probe answers it before any test runs.
    Write-Host "`n--- host probe: start and stop twice ---" -ForegroundColor Cyan
    foreach ($round in 1..2) {
        Write-Host "round $round, start:"
        if (-not (Start-Dummy)) {
            Fail "$service did not reach Running on start $round. The host binary cannot host a service here. Write the fixture service crate instead."
        }
        Write-Host "round $round, stop:"
        if (-not (Stop-Dummy)) {
            Fail "$service did not reach Stopped on stop $round. The host binary cannot host a service here."
        }
    }

    Write-Host 'start again and leave it running for the stop test:'
    if (-not (Start-Dummy)) {
        Fail "$service did not reach Running before the tests. The host binary cannot host a service here."
    }

    # The stop test needs the service running, so it is left alone here.
    $results = @()
    foreach ($filter in @(
        'should_stop_the_test_service',
        'should_start_the_test_service',
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