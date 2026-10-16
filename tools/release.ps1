<#
.SYNOPSIS
    Checks a release before it's published: builds it and runs every test. Doesn't publish anything.

    pwsh -File tools\release.ps1

    The files people download are then built by GitHub from the pushed commit (.github\workflows\release.yml), with an
    attestation of where they were built, and tools\publish.ps1 signs the zip for the updater with the key in
    %USERPROFILE%\.roblox-account-manager\release-signing-key.txt (keep a backup of it) and publishes the release.

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

# what gets tested must be what gets shipped: the tests use these exes and never run cargo themselves
$env:RAM_RELEASE_BUILT = '1'
$Exes = "target\release\RobloxAccountManager.exe", "target\release\ram-cli.exe"
$Built = $Exes | ForEach-Object { (Get-FileHash $_).Hash }

# each suite runs its own copy of the app (own folder, lock and browser data)
Together "Tests" @{
    "command line tests" = "node tests/cli.mjs --release"
    "interface tests"    = "node tests/ui.mjs --release"
    "updater tests"      = "node tests/update.mjs"
}

if ((Compare-Object $Built ($Exes | ForEach-Object { (Get-FileHash $_).Hash }))) {
    throw "The exes were built again while the tests ran, so the tested ones aren't the ones that would ship. Run the release again."
}

Write-Host ("`nAll tests passed in {0:N1} minutes. Commit, push, then publish (GitHub builds the release):" -f ((Get-Date) - $Started).TotalMinutes) -ForegroundColor Green
"  git push fork modernized:master"
"  pwsh -File tools\publish.ps1 -Notes <notes.md>"
