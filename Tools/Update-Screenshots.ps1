<#
.SYNOPSIS
    Retakes every screenshot used by the README (Images\*.png) from the current build.

.DESCRIPTION
    Starts a copy of the build in Tests\.run\app with 6 made-up demo accounts (your own accounts and settings are
    never used), opens each window, captures it and builds the README images:

      Images\main.png              main window (English)
      Images\languages.png         main window in every other language, 2 x 2
      Images\language-menu.png     the flag menu over the main window
      Images\account-menu.png      right click menu of an account
      Images\search.png            searching the account list
      Images\settings.png          the 3 settings tabs side by side
      Images\theme-editor.png      theme editor
      Images\utilities.png         utilities with a server list
      Images\account-utilities.png account utilities

    The README links to these fixed names, so after a UI change running this script (and committing Images\) is
    all that's needed. The app's windows pop up while it runs, it takes about a minute.

.EXAMPLE
    pwsh -File Tools\Update-Screenshots.ps1
    pwsh -File Tools\Update-Screenshots.ps1 -SkipBuild -NoNetwork
#>
param(
    [switch] $SkipBuild,
    [switch] $NoNetwork # don't load the server list (it needs the internet)
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$BuildDir = Join-Path $Root "RBX Alt Manager\bin\x64\Release"
$WorkDir = Join-Path $Root "Tests\.run\app"
$Raw = Join-Path $Root "Tests\.run\screenshots"
$Images = Join-Path $Root "Images"

Import-Module (Join-Path $Root "Tests\UI\RamUi.psm1") -Force
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class WindowRect
{
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("dwmapi.dll")] static extern int DwmGetWindowAttribute(IntPtr h, int a, out RECT r, int s);
    public static RECT Of(IntPtr h) { RECT r; DwmGetWindowAttribute(h, 9, out r, 16); return r; } // DWMWA_EXTENDED_FRAME_BOUNDS, same area PrintWindow captures
}
'@

#region Build & helpers

if (-not $SkipBuild) {
    $MSBuild = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -requires Microsoft.Component.MSBuild -find "MSBuild\**\Bin\amd64\MSBuild.exe" | Select-Object -First 1
    & $MSBuild (Join-Path $Root "RBX Alt Manager.sln") -restore -t:Build -p:Configuration=Release -p:Platform=x64 -v:minimal -nologo "-clp:ErrorsOnly;Summary"
    if ($LASTEXITCODE) { throw "Build failed (close Roblox Account Manager if it's running)" }
}

New-Item -ItemType Directory -Force $Raw, $Images | Out-Null
Get-ChildItem $Raw -File | Remove-Item -Force

$Positions = @{}

function Capture([string] $Name, [IntPtr] $Window) {
    Start-Sleep -Milliseconds 400
    Save-RamScreenshot $Window (Join-Path $Raw "$Name.png")
    $Positions[$Name] = [WindowRect]::Of($Window)
}

function Get-OpenMenu { Get-RamWindows | Where-Object { $_.Title -eq "" -and $_.Class -match "Window.808" } | Select-Object -First 1 -ExpandProperty Handle }

function Close-Window([IntPtr] $Window) { [RamWin]::PostMessage($Window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null; Start-Sleep -Milliseconds 300 }

function Load([string] $Name) { [System.Drawing.Image]::FromFile((Join-Path $Raw "$Name.png")) }

$Background = [System.Drawing.Color]::FromArgb(13, 17, 23) # GitHub dark, the images sit on it

function New-Canvas([int] $Width, [int] $Height) {
    $Bitmap = New-Object System.Drawing.Bitmap $Width, $Height
    $Graphics = [System.Drawing.Graphics]::FromImage($Bitmap)
    $Graphics.Clear($Background)
    $Graphics.InterpolationMode = "HighQualityBicubic"
    $Graphics.SmoothingMode = "AntiAlias"
    $Graphics.TextRenderingHint = "AntiAliasGridFit"
    , @($Bitmap, $Graphics)
}

function Save-Image($Canvas, [string] $Name, [string] $To = $Images) {
    $Canvas[1].Dispose()
    $Canvas[0].Save((Join-Path $To $Name), [System.Drawing.Imaging.ImageFormat]::Png)
    $Canvas[0].Dispose()
    if ($To -eq $Images) { Write-Host "  Images\$Name" }
}

# a window with a menu drawn over it at the place it opened
function Save-WithMenu([string] $Window, [string] $Menu, [string] $Name, [System.Drawing.Rectangle] $Crop) {
    $Base = Load $Window; $Popup = Load $Menu
    $Canvas = New-Canvas $Crop.Width $Crop.Height
    $Canvas[1].DrawImage($Base, (New-Object System.Drawing.Rectangle 0, 0, $Crop.Width, $Crop.Height), $Crop, "Pixel")
    $X = $Positions[$Menu].Left - $Positions[$Window].Left - $Crop.X
    $Y = $Positions[$Menu].Top - $Positions[$Window].Top - $Crop.Y
    $Canvas[1].DrawImage($Popup, $X, $Y, $Popup.Width, $Popup.Height)
    $Base.Dispose(); $Popup.Dispose()
    Save-Image $Canvas $Name
}

# images next to each other with a small gap, optional caption under each
function Save-Row([string[]] $Names, [string] $Name, [string[]] $Captions = @(), [float] $Scale = 1, [string] $To = $Images) {
    $Pictures = $Names | ForEach-Object { Load $_ }
    $Gap = 16; $Caption = if ($Captions) { 30 } else { 0 }
    $Width = [int](($Pictures | ForEach-Object { $_.Width * $Scale } | Measure-Object -Sum).Sum + $Gap * ($Pictures.Count + 1))
    $Height = [int](($Pictures | ForEach-Object { $_.Height * $Scale } | Measure-Object -Maximum).Maximum + $Gap * 2 + $Caption)
    $Canvas = New-Canvas $Width $Height
    $Font = New-Object System.Drawing.Font "Segoe UI", 11
    $X = $Gap

    for ($i = 0; $i -lt $Pictures.Count; $i++) {
        $W = [int]($Pictures[$i].Width * $Scale); $H = [int]($Pictures[$i].Height * $Scale)
        $Canvas[1].DrawImage($Pictures[$i], $X, $Gap, $W, $H)
        if ($Captions) { $Canvas[1].DrawString($Captions[$i], $Font, [System.Drawing.Brushes]::Gainsboro, $X, $Gap + $H + 6) }
        $X += $W + $Gap
    }

    $Pictures | ForEach-Object { $_.Dispose() }
    Save-Image $Canvas $Name $To
}

#endregion

Write-Host "Taking screenshots (the app's windows will pop up for about a minute)"

try {
    Start-RamApp -BuildDir $BuildDir -WorkDir $WorkDir -Language en
    $Main = Find-RamWindow "^Roblox Account Manager$"
    $List = Get-RamChildren $Main "SysListView32" | Select-Object -First 1
    $Row = Get-RamListItemPoint $Main $List "NightFalcon_07"
    Invoke-RamMouse $Main $Row.X $Row.Y
    Start-Sleep 3

    Capture "main_en" $Main

    Invoke-RamButton $Main -At 843, 33 # language button
    Wait-Until { [bool](Get-OpenMenu) }
    Capture "language_menu" (Get-OpenMenu)
    Send-RamKeys "Escape"

    $Languages = "en", "pt-PT", "es", "fr", "de" # order of the language menu
    for ($i = 1; $i -lt $Languages.Count; $i++) {
        Invoke-RamButton $Main -At 843, 33
        Wait-Until { [bool](Get-OpenMenu) }
        Send-RamKeys ("Home" + (",Down" * $i) + ",Return")
        Start-Sleep 2
        Capture "main_$($Languages[$i])" $Main
    }

    Invoke-RamButton $Main -At 843, 33
    Wait-Until { [bool](Get-OpenMenu) }
    Send-RamKeys "Home,Return"
    Start-Sleep 2

    Invoke-RamMouse $Main $Row.X $Row.Y -Button Right
    Wait-Until { [bool](Get-OpenMenu) }
    Capture "account_menu" (Get-OpenMenu)
    Capture "main_account_menu" $Main
    Send-RamKeys "Escape"

    $Box = Set-RamText $Main 150 86 "grind"
    Start-Sleep 1
    Capture "search" $Main
    Set-RamText -Window $Main -Control $Box -Text "" | Out-Null

    Invoke-RamButton $Main -At 947, 33 # settings
    $Settings = Wait-RamWindow "^Settings$"
    Capture "settings_general" $Settings
    Select-RamTab $Settings 1
    Invoke-RamButton $Settings "Enable Developer Mode"
    Invoke-RamButton $Settings "Enable Web Server"
    Close-AllRamDialogs
    Capture "settings_developer" $Settings
    Invoke-RamButton $Settings "Enable Web Server"
    Invoke-RamButton $Settings "Enable Developer Mode"
    Select-RamTab $Settings 2
    Capture "settings_misc" $Settings
    Close-Window $Settings

    Invoke-RamButton $Main -At 905, 33 # theme editor
    $Theme = Wait-RamWindow "^Theme Editor$"
    Capture "theme_editor" $Theme
    Close-Window $Theme

    Invoke-RamButton $Main "Utilities"
    $Utilities = Wait-RamWindow "^Utilities$"
    if (-not $NoNetwork) {
        Invoke-RamButton $Utilities "Refresh"
        $Servers = Get-RamChildren $Utilities "SysListView32" | Select-Object -First 1
        try { Wait-Until -Timeout 25 { (Get-RamListCount $Servers) -gt 5 } } catch { Write-Warning "The server list didn't load, the utilities screenshot will be empty" }
        Start-Sleep 2
    }
    Capture "utilities" $Utilities
    Close-Window $Utilities

    $Row = Get-RamListItemPoint $Main $List "NightFalcon_07" # the search above cleared the selection, this window needs one
    Invoke-RamMouse $Main $Row.X $Row.Y
    Invoke-RamButton $Main "Account Utilities"
    $AccountUtilities = Wait-RamWindow "^Account Utilities$"
    Capture "account_utilities" $AccountUtilities
    Close-Window $AccountUtilities
}
finally {
    Stop-RamApp
    Remove-Item -Recurse -Force $WorkDir -ErrorAction SilentlyContinue
}

Write-Host "Building the README images"

Copy-Item (Join-Path $Raw "main_en.png") (Join-Path $Images "main.png") -Force
Write-Host "  Images\main.png"

Save-Row "main_pt-PT", "main_es" "languages-1.png" -Captions "Português (Portugal)", "Español" -Scale 0.62 -To $Raw
Save-Row "main_fr", "main_de" "languages-2.png" -Captions "Français", "Deutsch" -Scale 0.62 -To $Raw
$First = Load "languages-1"; $Second = Load "languages-2"
$Canvas = New-Canvas ([Math]::Max($First.Width, $Second.Width)) ($First.Height + $Second.Height)
$Canvas[1].DrawImage($First, 0, 0, $First.Width, $First.Height)
$Canvas[1].DrawImage($Second, 0, $First.Height, $Second.Width, $Second.Height)
$First.Dispose(); $Second.Dispose()
Save-Image $Canvas "languages.png"

$MainWidth = (Load "main_en").Width
Save-WithMenu "main_en" "language_menu" "language-menu.png" (New-Object System.Drawing.Rectangle ($MainWidth - 420), 0, 420, 230)
Save-WithMenu "main_account_menu" "account_menu" "account-menu.png" (New-Object System.Drawing.Rectangle 0, 0, 640, 420)
Copy-Item (Join-Path $Raw "search.png") (Join-Path $Images "search.png") -Force; Write-Host "  Images\search.png"
Save-Row "settings_general", "settings_developer", "settings_misc" "settings.png"
foreach ($Name in "theme_editor", "utilities", "account_utilities") {
    Copy-Item (Join-Path $Raw "$Name.png") (Join-Path $Images ($Name.Replace("_", "-") + ".png")) -Force
    Write-Host "  Images\$($Name.Replace("_", "-")).png"
}

Write-Host "`nDone, commit the Images folder to update the README"
