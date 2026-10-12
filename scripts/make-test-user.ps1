# Creates the temporary standard account used by the standard-user installer test
# in docs/install-tests.md. Run elevated. Delete the account afterwards with
# scripts\remove-test-user.ps1.

$name = "leashtest"
$password = ConvertTo-SecureString "Leash-Test-2026!" -AsPlainText -Force

if (Get-LocalUser -Name $name -ErrorAction SilentlyContinue) {
    Write-Host "$name already exists, leaving it alone"
} else {
    New-LocalUser -Name $name -Password $password -FullName "Leash test account" `
        -Description "Temporary account for the Leash standard-user install test" | Out-Null
    Write-Host "$name created"
}

Write-Host ""
Write-Host "What the account is now"
Get-LocalUser -Name $name | Format-List Name, Enabled, FullName, Description

$administrators = Get-LocalGroupMember -Group "Administrators" -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -like "*$name" }
if ($administrators) {
    Write-Host "WARNING: $name is in Administrators. It must not be, or the test proves nothing."
} else {
    Write-Host "$name is not in Administrators, so it is a standard user"
}
