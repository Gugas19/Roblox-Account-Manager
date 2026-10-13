<#
.SYNOPSIS
    Builds a release and signs it for the auto updater. Doesn't publish anything.

    pwsh -File tools\release.ps1

    Leaves target\dist\RobloxAccountManager-<version>.zip and .zip.sig: upload both to the GitHub release
    tagged v<version>. Installed apps only accept the zip with its signature, made with the key in
    %USERPROFILE%\.roblox-account-manager\release-signing-key.txt (keep a backup of it).

    The app can stay open: the tests run their own copies, which never take the focus. What doesn't depend on
    each other runs at the same time (the final build uses one core for a while, the PC has more).
#>
$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $Root
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$Logs = Join-Path $Root "target\release-logs"
New-Item -ItemType Directory -Force $Logs | Out-Null
$Started = Get-Date

function Step([string] $Name, [scriptblock] $Work) {
    Write-Host "`n$Name" -ForegroundColor Cyan
    $watch = [Diagnostics.Stopwatch]::StartNew()
    & $Work
    if ($LASTEXITCODE) { throw "$Name failed" }
    Write-Host ("  {0:N0} s" -f $watch.Elapsed.TotalSeconds) -ForegroundColor DarkGray
}

# Runs commands at the same time, each with its log; fails with the log of whatever failed
function Together([string] $Name, [hashtable] $Commands) {
    Write-Host "`n$Name" -ForegroundColor Cyan
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $running = foreach ($entry in $Commands.GetEnumerator()) {
        $log = Join-Path $Logs ("{0}.log" -f ($entry.Key -replace '\W', '-'))
        $process = Start-Process -FilePath cmd.exe -ArgumentList '/d', '/c', "$($entry.Value) > `"$log`" 2>&1" -WorkingDirectory $Root -NoNewWindow -PassThru
        [pscustomobject]@{ Name = $entry.Key; Process = $process; Log = $log }
    }
    $failed = @()
    foreach ($r in $running) {
        $r.Process.WaitForExit()
        $tail = Get-Content $r.Log -Tail 1 -ErrorAction SilentlyContinue
        if ($r.Process.ExitCode) { $failed += $r }
        Write-Host ("  {0,-22} {1,5:N0} s  {2}" -f $r.Name, ($r.Process.ExitTime - $r.Process.StartTime).TotalSeconds, $tail) -ForegroundColor $(if ($r.Process.ExitCode) { 'Red' } else { 'Gray' })
    }
    foreach ($f in $failed) { Write-Host "`n--- $($f.Name) ($($f.Log))" -ForegroundColor Red; Get-Content $f.Log -Tail 30 }
    if ($failed) { throw "$Name failed: $($failed.Name -join ', ')" }
    Write-Host ("  {0:N0} s" -f $watch.Elapsed.TotalSeconds) -ForegroundColor DarkGray
}

$Version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches.Groups[1].Value
Write-Host "Roblox Account Manager $Version" -ForegroundColor Green

Step "Translations" { python tools/strings.py | Select-Object -First 1 }
Step "Interface" { npm --prefix ui run build | Select-Object -Last 1 }

# the unit tests build in their own folder, so they don't wait for the release build's lock
Together "Build and unit tests" @{
    # the signing tool too, built alongside (the updater test and the signature use it)
    "release build" = "cargo build --release --bins --example sign-release"
    "unit tests"    = "set CARGO_TARGET_DIR=target\unit&& cargo test --lib"
}

# each suite runs its own copy of the app (own folder, lock and browser data)
Together "Tests" @{
    "command line tests" = "node tests/cli.mjs --release"
    "interface tests"    = "node tests/ui.mjs --release"
    "updater tests"      = "node tests/update.mjs"
}

# Windows code signing (Authenticode), optional: with a certificate in the Windows certificate store and its
# thumbprint in RAM_CODESIGN_THUMBPRINT, the exes are signed (fewer antivirus / SmartScreen warnings for users)
if ($env:RAM_CODESIGN_THUMBPRINT) {
    Step "Code signing" {
        $signtool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue | Sort-Object FullName -Descending | Select-Object -First 1
        if (-not $signtool) { throw "signtool.exe wasn't found, install the Windows SDK" }
        & $signtool.FullName sign /fd sha256 /tr http://timestamp.digicert.com /td sha256 /sha1 $env:RAM_CODESIGN_THUMBPRINT target\release\RobloxAccountManager.exe target\release\ram-cli.exe
    }
} else {
    Write-Host "`nCode signing: skipped (no certificate set up, see Releasing in the README)" -ForegroundColor DarkGray
}

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

Write-Host ("`nReady to upload, in {0:N1} minutes:" -f ((Get-Date) - $Started).TotalMinutes) -ForegroundColor Green
Get-ChildItem $Dist | ForEach-Object { "  $($_.FullName)" }
"  SHA-256 $((Get-FileHash $Zip -Algorithm SHA256).Hash)"
