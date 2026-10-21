<#
.SYNOPSIS
    Checks every Roblox request behind the app's pages against the real Roblox, with your accounts. Read only:
    nothing is joined, bought, worn, sent, changed or removed.

    pwsh -File tools\live-check.ps1 [-App <the app's folder>]

    Your accounts and settings are copied to a temporary folder (the app never opens the real ones here), the
    password is asked when they have one (hidden, never written anywhere), and the copy is deleted at the end.
#>
param(
    [string] $App = (Join-Path (Resolve-Path (Join-Path $PSScriptRoot "..")) "app")
)
$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

$Copy = Join-Path ([IO.Path]::GetTempPath()) "ram-live-$([guid]::NewGuid().ToString('N').Substring(0, 8))"
New-Item -ItemType Directory $Copy | Out-Null
try {
    foreach ($file in "AccountData.json", "RAMSettings.ini") {
        if (Test-Path (Join-Path $App $file)) { Copy-Item (Join-Path $App $file) $Copy }
    }
    New-Item -ItemType File (Join-Path $Copy "ram-test-copy") | Out-Null
    $env:RAM_LIVE_DIR = $Copy

    $data = Get-Content (Join-Path $Copy "AccountData.json") -Raw -ErrorAction SilentlyContinue
    if ($data -and -not $data.TrimStart().StartsWith("[") -and $data.StartsWith("Roblox Account Manager created by ic3w0lf22")) {
        $secure = Read-Host "Accounts password" -AsSecureString
        $env:RAM_PASSWORD = [Runtime.InteropServices.Marshal]::PtrToStringBSTR([Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure))
    }
    Push-Location $Root
    try {
        cargo test -q --test live_check -- --ignored --nocapture
    } finally {
        Pop-Location
    }
} finally {
    Remove-Item Env:\RAM_PASSWORD -ErrorAction SilentlyContinue
    Remove-Item Env:\RAM_LIVE_DIR -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force $Copy -ErrorAction SilentlyContinue
}
