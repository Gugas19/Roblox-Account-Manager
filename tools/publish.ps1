<#
.SYNOPSIS
    Publishes the version in Cargo.toml, built by GitHub: run it after tools\release.ps1 passed and the commit is pushed.

    pwsh -File tools\publish.ps1 -Notes notes.md

    1. Tags the current commit v<version> and pushes the tag: .github\workflows\release.yml builds the exes and the zip
       on GitHub, attests where they were built, and leaves a draft release (the updater doesn't see drafts).
    2. Downloads the draft's files and checks their attestations: built by that workflow, from this repository.
    3. Signs the zip for the updater with the key on this PC (it never goes to GitHub), uploads the .sig.
    4. Publishes the release with the notes, plus how to verify the files.
#>
param(
    [Parameter(Mandatory)] [string] $Notes
)
$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $Root
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$Repo = "Gugas19/Roblox-Account-Manager"
$Remote = "fork"

function Run([string] $What, [scriptblock] $Work) {
    Write-Host "`n$What" -ForegroundColor Cyan
    & $Work
    if ($LASTEXITCODE) { throw "$What failed" }
}

$Version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches.Groups[1].Value
$Tag = "v$Version"
$Head = (git rev-parse HEAD).Trim()
$NotesText = Get-Content $Notes -Raw
Write-Host "Roblox Account Manager $Version ($Head)" -ForegroundColor Green

if (git status --porcelain --untracked-files=no) { throw "Commit your changes first: GitHub builds the commit, not this folder" }
git fetch $Remote -q
if ((git rev-parse "$Remote/master").Trim() -ne $Head) { throw "Push the commit first (git push $Remote modernized:master)" }
if (gh release view $Tag --repo $Repo --json isDraft --jq '.isDraft' 2>$null | Where-Object { $_ -eq 'false' }) { throw "$Tag is already published" }

Run "Tag $Tag" {
    if (-not (git tag --list $Tag)) { git tag $Tag $Head }
    if ((git rev-parse "$Tag^{commit}").Trim() -ne $Head) { throw "$Tag already points at another commit" }
    git push $Remote $Tag -q
}

Run "GitHub builds it" {
    $run = $null
    for ($i = 0; $i -lt 30 -and -not $run; $i++) {
        $run = gh run list --repo $Repo --workflow release.yml --limit 10 --json databaseId,headSha --jq ".[] | select(.headSha == `"$Head`") | .databaseId" | Select-Object -First 1
        if (-not $run) { Start-Sleep 5 }
    }
    if (-not $run) { throw "The release workflow didn't start" }
    gh run watch $run --repo $Repo --exit-status --interval 15 | Select-Object -Last 3
}

$Dist = Join-Path $Root "target\published\$Version"
Remove-Item -Recurse -Force $Dist -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $Dist | Out-Null
$Zip = Join-Path $Dist "RobloxAccountManager-$Version.zip"

Run "Download the draft" { gh release download $Tag --repo $Repo --dir $Dist --clobber }

# built by this repository's release workflow, nothing else: a file from anywhere else stops the publishing here
Run "Check the attestations" {
    foreach ($file in Get-ChildItem $Dist -File) {
        gh attestation verify $file.FullName --repo $Repo --signer-workflow "$Repo/.github/workflows/release.yml" --format json --jq '.[0].verificationResult.statement.subject[0].name' | Out-Null
        if ($LASTEXITCODE) { throw "$($file.Name) has no valid attestation" }
        Write-Host "  $($file.Name): built by GitHub from $Repo"
    }
}

# signs, then checks the signature the same way the app does
Run "Sign for the updater" {
    if (-not (Test-Path target\release\examples\sign-release.exe)) { cargo build -q --release --example sign-release }
    & target\release\examples\sign-release.exe $Zip
}

Run "Upload the signature" { gh release upload $Tag "$Zip.sig" --repo $Repo --clobber }

$Hashes = (Get-ChildItem $Dist -File -Exclude *.sig | ForEach-Object { "| ``$($_.Name)`` | ``$((Get-FileHash $_.FullName -Algorithm SHA256).Hash)`` |" }) -join "`n"
$Verify = @"

## Download

- **``RobloxAccountManager.exe``**: just the app, run it from a folder of its own.
- **``RobloxAccountManager-$Version.zip``**: the app, ``ram-cli.exe`` and the license. Extract it over your current folder; your accounts and settings stay where they are.

Installed copies update by themselves (or only tell you, depending on *Settings → General → Updates*).

## Verify

These files were built by GitHub from this repository, at commit ``$Head``, not on someone's PC. Check it with the [GitHub CLI](https://cli.github.com/):

``````
gh attestation verify RobloxAccountManager.exe --repo $Repo
``````

| File | SHA-256 |
| --- | --- |
$Hashes
"@
$Body = Join-Path $Dist "notes.md"
Set-Content $Body ($NotesText.TrimEnd() + "`n" + $Verify) -NoNewline

Run "Publish" { gh release edit $Tag --repo $Repo --draft=false --latest --notes-file $Body | Out-Null }
Write-Host "`nhttps://github.com/$Repo/releases/tag/$Tag" -ForegroundColor Green
