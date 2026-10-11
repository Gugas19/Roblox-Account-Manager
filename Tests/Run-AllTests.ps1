<#
.SYNOPSIS
    Builds the app and runs the tests that don't touch the app's windows.
    -WithUI also runs the UI tests, which open the app and click through it.

.EXAMPLE
    pwsh -File Tests\Run-AllTests.ps1
    pwsh -File Tests\Run-AllTests.ps1 -WithUI
#>
param(
    [switch] $SkipBuild,
    [switch] $WithUI,
    [switch] $SkipBrowser,
    [switch] $SkipNetwork
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")

$MSBuild = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -requires Microsoft.Component.MSBuild -find "MSBuild\**\Bin\amd64\MSBuild.exe" | Select-Object -First 1
if (-not $MSBuild) { throw "MSBuild wasn't found, install Visual Studio or the Visual Studio Build Tools" }

function Build([string] $Project, [string[]] $Properties) {
    & $MSBuild $Project -restore -t:Build -v:minimal -nologo "-clp:ErrorsOnly;Summary" @Properties
    if ($LASTEXITCODE) { throw "Build of $Project failed" }
}

if (-not $SkipBuild) {
    Write-Host "Building the app" -ForegroundColor Cyan
    Build (Join-Path $Root "RBX Alt Manager.sln") "-p:Configuration=Release", "-p:Platform=x64"
}

Write-Host "`nUnit tests" -ForegroundColor Cyan
Build (Join-Path $Root "Tests\Unit\RAM.UnitTests.csproj") @()
& (Join-Path $Root "Tests\Unit\bin\RAM.UnitTests.exe")
$UnitFailed = $LASTEXITCODE -ne 0

$UIFailed = $false
if ($WithUI) {
    Write-Host "`nUI tests" -ForegroundColor Cyan
    & (Join-Path $PSScriptRoot "UI\Run-UITests.ps1") -SkipBrowser:$SkipBrowser -SkipNetwork:$SkipNetwork
    $UIFailed = $LASTEXITCODE -ne 0
}

if ($UnitFailed -or $UIFailed) { Write-Host "`nSome tests failed" -ForegroundColor Red; exit 1 }
Write-Host "`nAll tests passed" -ForegroundColor Green
