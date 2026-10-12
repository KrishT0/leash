# Creates the temporary standard account used by the standard-user installer test
# in docs/install-tests.md. Run elevated, and remove the account afterwards with
# `Remove-LocalUser -Name leashtest`.

$name = "leashtest"

# A fresh password each run, printed below and stored nowhere. This repository is
# public, so it holds no credential, not even for an account as short lived as this.
$alphabet = "abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789".ToCharArray()
$plain = -join ($alphabet | Get-Random -Count 20)
$password = ConvertTo-SecureString $plain -AsPlainText -Force

if (Get-LocalUser -Name $name -ErrorAction SilentlyContinue) {
    Write-Host "$name already exists, leaving it alone"
    Write-Host "Its password is not recorded here. Remove the account and run this again if you need a known one."
} else {
    New-LocalUser -Name $name -Password $password -FullName "Leash test account" `
        -Description "Temporary account for the Leash standard-user install test" | Out-Null
    Write-Host "$name created"
    Write-Host ""
    Write-Host "Sign in as $name with this password: $plain"
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
