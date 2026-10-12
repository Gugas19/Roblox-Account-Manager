<#
.SYNOPSIS
    Builds a release and signs it for the auto updater. Doesn't publish anything.

    pwsh -File tools\release.ps1

    Leaves target\dist\RobloxAccountManager-<version>.zip and .zip.sig: upload both to the GitHub release
    tagged v<version>. Installed apps only accept the zip with its signature, made with the key in
    %USERPROFILE%\.roblox-account-manager\release-signing-key.txt (keep a backup of it).
#>
$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $Root
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

function Step([string] $Name, [scriptblock] $Work) {
    Write-Host "`n$Name" -ForegroundColor Cyan
    & $Work
    if ($LASTEXITCODE) { throw "$Name failed" }
}

$Version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches.Groups[1].Value
Write-Host "Roblox Account Manager $Version" -ForegroundColor Green

# the tests can't run next to an open Roblox Account Manager: say so now, not after the build
$Open = Get-Process RobloxAccountManager -ErrorAction SilentlyContinue | Where-Object { $_.Path -notlike '*\target\test-run\*' }
if ($Open) { throw "Roblox Account Manager is open ($($Open[0].Path)), close it and keep it closed until the release is ready" }

Step "Translations" { python tools/strings.py | Select-Object -First 1 }
Step "Interface" { npm --prefix ui run build }
Step "Build" { cargo build --release }
Step "Unit tests" { cargo test --lib }
Step "Command line tests" { node tests/cli.mjs --release }
Step "Interface tests" { node tests/ui.mjs --release }
Step "Updater tests" { node tests/update.mjs }

$Dist = Join-Path $Root "target\dist"
$Pack = Join-Path $Dist "pack"
Remove-Item -Recurse -Force $Dist -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $Pack | Out-Null
Copy-Item target\release\RobloxAccountManager.exe, target\release\ram-cli.exe $Pack
Copy-Item LICENSE (Join-Path $Pack "LICENSE.txt")
$Zip = Join-Path $Dist "RobloxAccountManager-$Version.zip"
Compress-Archive -Path "$Pack\*" -DestinationPath $Zip
Remove-Item -Recurse -Force $Pack

# signs, then checks the signature the same way the app does
Step "Signature" { cargo run -q --release --example sign-release -- $Zip }

Write-Host "`nReady to upload:" -ForegroundColor Green
Get-ChildItem $Dist | ForEach-Object { "  $($_.FullName)" }
"  SHA-256 $((Get-FileHash $Zip -Algorithm SHA256).Hash)"
