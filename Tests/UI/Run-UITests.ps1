<#
.SYNOPSIS
    UI tests for Roblox Account Manager: every button, menu, option and window, run against the built app with fake accounts.

.DESCRIPTION
    The app is copied to Tests\.run\app and started with 6 fake accounts, so real data is never touched.
    All input goes to the app's own windows (window messages), never to screen coordinates.
    Things that are left alone on purpose:
      - the Discord button (it only opens discord.gg in your default browser)
      - turning on the Roblox Watcher when its EULA was already accepted on this PC with close options on (it could close a running Roblox)
      - the "Run on Windows Startup" option when an entry with the same name already exists (it would replace it)
    Your clipboard is saved before the tests and restored afterwards.

.EXAMPLE
    pwsh -File Tests\UI\Run-UITests.ps1
#>
param(
    [string] $BuildDir = (Join-Path $PSScriptRoot "..\..\RBX Alt Manager\bin\x64\Release"),
    [string] $RunDir = (Join-Path $PSScriptRoot "..\.run"),
    [switch] $SkipBrowser,
    [switch] $SkipNetwork
)

$ErrorActionPreference = "Stop"
Import-Module (Join-Path $PSScriptRoot "RamUi.psm1") -Force

$BuildDir = (Resolve-Path $BuildDir).Path
$RunDir = [IO.Path]::GetFullPath($RunDir)
$WorkDir = Join-Path $RunDir "app"
$OutDir = Join-Path $RunDir "results"
New-Item -ItemType Directory -Force $OutDir | Out-Null
Get-ChildItem $OutDir -File | Remove-Item -Force

$Results = [System.Collections.Generic.List[object]]::new()
$SavedClipboard = try { Get-Clipboard -Raw } catch { $null }
$script:Main = [IntPtr]::Zero

#region Framework

function Assert([bool] $Condition, [string] $Message) { if (-not $Condition) { throw $Message } }
function AssertEq($Expected, $Actual, [string] $What) { if ($Expected -ne $Actual) { throw "${What}: expected [$Expected] got [$Actual]" } }
function AssertMatch([string] $Text, [string] $Pattern, [string] $What) { if ($Text -notmatch $Pattern) { throw "${What}: [$Text] doesn't match /$Pattern/" } }

function Test([string] $Group, [string] $Name, [scriptblock] $Body) {
    $Started = Get-Date
    try {
        & $Body
        $Results.Add([pscustomobject]@{ Group = $Group; Test = $Name; Result = "PASS"; Detail = ""; Seconds = [math]::Round(((Get-Date) - $Started).TotalSeconds, 1) })
        Write-Host "  PASS  $Group / $Name" -ForegroundColor Green
    }
    catch {
        $Shot = Join-Path $OutDir ("fail_{0:000}.png" -f $Results.Count)
        try { if ($script:Main) { Save-RamScreenshot $script:Main $Shot } } catch { }
        $Results.Add([pscustomobject]@{ Group = $Group; Test = $Name; Result = "FAIL"; Detail = "$($_.Exception.Message) ($Shot)"; Seconds = [math]::Round(((Get-Date) - $Started).TotalSeconds, 1) })
        Write-Host "  FAIL  $Group / $Name : $($_.Exception.Message)" -ForegroundColor Red
        Close-AllRamDialogs
        while (Test-RamMenuOpen) { Send-RamKeys "Escape" }
    }
}

function Skip([string] $Group, [string] $Name, [string] $Why) {
    $Results.Add([pscustomobject]@{ Group = $Group; Test = $Name; Result = "SKIP"; Detail = $Why; Seconds = 0 })
    Write-Host "  SKIP  $Group / $Name : $Why" -ForegroundColor Yellow
}

function Start-App([string] $Language = "pt-PT", [string[]] $Extra = @(), [switch] $NoAccounts) {
    Start-RamApp -BuildDir $BuildDir -WorkDir $WorkDir -Language $Language -ExtraSettings $Extra -NoAccounts:$NoAccounts
    $script:Main = Find-RamWindow "^Roblox Account Manager$"
}

function Shot([string] $Name, [IntPtr] $Window = $script:Main) { Save-RamScreenshot $Window (Join-Path $OutDir "$Name.png") }

# layout of the main window (client coordinates, 100% scale)
$P = @{
    AddArrow = @(172, 33); OpenBrowserArrow = @(478, 33); Discord = @(781, 33); Language = @(843, 33); Theme = @(905, 33); Settings = @(947, 33)
    Search = @(150, 86); PlaceID = @(700, 157); JobID = @(840, 157); SaveToAccount = @(933, 157); Shuffle = @(901, 134); History = @(751, 134)
    UserID = @(730, 270); Alias = @(740, 333); Description = @(790, 400)
}

function Get-AccountList { Get-RamChildren $script:Main "SysListView32" | Select-Object -First 1 }
function Get-AccountRow([string] $Name) { Get-RamListItemPoint $script:Main (Get-AccountList) $Name }
function Select-Account([string] $Name) { $Point = Get-AccountRow $Name; Invoke-RamMouse $script:Main $Point.X $Point.Y }
function Open-AccountMenu([string] $Name = "NightFalcon_07") {
    $Point = Get-AccountRow $Name
    Invoke-RamMouse $script:Main $Point.X $Point.Y
    Invoke-RamMouse $script:Main $Point.X $Point.Y -Button Right
    Wait-Until -Message "context menu" { Test-RamMenuOpen }
}
function Open-DropMenu([int[]] $At, [switch] $Button) {
    # the arrow of a menu button opens its menu on mouse down, a plain button with a menu on click
    for ($Try = 0; $Try -lt 3 -and -not (Test-RamMenuOpen); $Try++) {
        if ($Button) { Invoke-RamButton $script:Main -At $At } else { Invoke-RamMouse $script:Main $At[0] $At[1] }
        try { Wait-Until -Timeout 2 { Test-RamMenuOpen } } catch { }
    }
    Assert (Test-RamMenuOpen) "menu at $At didn't open"
}
function Get-ClipText { (Get-Clipboard -Raw) -replace "`r", "" }
function Account([string] $Name) { Get-RamAccounts | Where-Object Username -eq $Name }
function Button([string] $Text, [IntPtr] $Window = $script:Main) { $Control = Get-RamControl $Window $Text; Assert ([bool]$Control) "button '$Text' not found"; $Control }

#endregion

Write-Host "Roblox Account Manager UI tests`nBuild: $BuildDir`n"

#region Main window

Start-App
Test "Startup" "main window opens in Portuguese with 6 accounts" {
    Assert ([bool]$script:Main) "no main window"
    Assert ([bool](Get-RamControl $script:Main "6 contas")) "account counter"
    Assert ([bool](Get-RamControl $script:Main "Adicionar conta")) "translated buttons"
    Shot "main_pt"
}

Test "Main window" "account buttons are disabled until an account is selected" {
    foreach ($Text in "Remover", "Abrir navegador", "Entrar no servidor", "Seguir", "Definir alias", "Guardar descrição", "Gerir conta") {
        Assert (-not (Test-RamEnabled (Button $Text))) "'$Text' is enabled without a selection"
    }
    Select-Account "NightFalcon_07"
    foreach ($Text in "Remover", "Abrir navegador", "Entrar no servidor", "Seguir", "Definir alias", "Guardar descrição", "Gerir conta") {
        Assert (Test-RamEnabled (Button $Text)) "'$Text' is disabled with an account selected"
    }
    AssertEq "Main" (Get-RamText (Get-RamControlAt $script:Main $P.Alias[0] $P.Alias[1])) "alias box shows the selected account"
}

Test "Main window" "search filters the list and updates the counter" {
    $Box = Set-RamText $script:Main $P.Search[0] $P.Search[1] "storage"
    Assert ([bool](Get-RamControl $script:Main "2 de 6 contas")) "counter while searching"
    Set-RamText -Window $script:Main -Control $Box -Text "nothing-matches" | Out-Null
    Assert ([bool](Get-RamControl $script:Main "0 de 6 contas")) "counter with no results"
    Set-RamText -Window $script:Main -Control $Box -Text "" | Out-Null
    Assert ([bool](Get-RamControl $script:Main "6 contas")) "counter after clearing"
}

Test "Main window" "Set Alias saves the alias" {
    Select-Account "GrindBot_A"
    Set-RamText $script:Main $P.Alias[0] $P.Alias[1] "Alt de teste" | Out-Null
    Invoke-RamButton $script:Main "Definir alias"
    Wait-Until -Message "alias saved" { (Account "GrindBot_A").Alias -eq "Alt de teste" }
}

Test "Main window" "Set Description saves the description" {
    Select-Account "GrindBot_A"
    Set-RamText $script:Main $P.Description[0] $P.Description[1] "Conta para farmar" | Out-Null
    Invoke-RamButton $script:Main "Guardar descrição"
    Wait-Until -Message "description saved" { (Account "GrindBot_A").Description -eq "Conta para farmar" }
}

Test "Main window" "save Place ID + Job ID to the account" {
    Select-Account "GrindBot_B"
    Set-RamText $script:Main $P.PlaceID[0] $P.PlaceID[1] "1818" | Out-Null
    Set-RamText $script:Main $P.JobID[0] $P.JobID[1] "test-job-id" | Out-Null
    Invoke-RamButton $script:Main -At $P.SaveToAccount
    Wait-Until -Message "fields saved" { $F = (Account "GrindBot_B").Fields; $F.SavedPlaceId -eq "1818" -and $F.SavedJobId -eq "test-job-id" }
    Set-RamText $script:Main $P.JobID[0] $P.JobID[1] "" | Out-Null
    Set-RamText $script:Main $P.PlaceID[0] $P.PlaceID[1] "5315046213" | Out-Null
}

Test "Main window" "shuffle job id toggles" {
    Invoke-RamMouse $script:Main $P.Shuffle[0] $P.Shuffle[1]
    Wait-Until -Message "shuffle on" { (Get-RamSetting "ShuffleJobId") -eq "true" }
    Invoke-RamMouse $script:Main $P.Shuffle[0] $P.Shuffle[1]
    Wait-Until -Message "shuffle off" { (Get-RamSetting "ShuffleJobId") -eq "false" }
}

Test "Main window" "hide usernames toggles" {
    Invoke-RamButton $script:Main "Ocultar nomes"
    Select-Account "NightFalcon_07"
    Shot "hidden_usernames"
    Invoke-RamButton $script:Main "Ocultar nomes"
}

Test "Main window" "Join Server with an expired account shows a translated error" {
    Select-Account "NightFalcon_07"
    Invoke-RamButton $script:Main "Entrar no servidor"
    $Dialog = Close-RamDialog "^OK$" -Timeout 20
    AssertMatch $Dialog.Text "ERRO" "error text"
}

if (-not $SkipNetwork) {
    Test "Main window" "Follow asks before following someone who isn't in a game" {
        Select-Account "NightFalcon_07"
        Set-RamText $script:Main $P.UserID[0] $P.UserID[1] "Roblox" | Out-Null
        Invoke-RamButton $script:Main "Seguir"
        Wait-RamDialog -Timeout 20 | Out-Null
        Close-RamDialog "^(Não|No|OK)$" | Out-Null
        Close-AllRamDialogs
    }
}
else { Skip "Main window" "Follow" "network tests skipped" }

Test "Main window" "Remove asks first, No keeps the account and Yes removes it" {
    Select-Account "StorageAlt02"
    Invoke-RamButton $script:Main "Remover"
    $Dialog = Close-RamDialog "^Não$"
    AssertMatch $Dialog.Text "StorageAlt02" "confirmation names the account"
    Assert ([bool](Account "StorageAlt02")) "removed after No"
    Invoke-RamButton $script:Main "Remover"
    Close-RamDialog "^Sim$" | Out-Null
    Wait-Until -Message "account removed" { -not (Account "StorageAlt02") }
    Assert ([bool](Get-RamControl $script:Main "5 contas")) "counter updated"
}

Test "Main window" "the Delete key removes the selected account (after asking)" {
    Select-Account "StorageAlt01"
    Send-RamKeys "Delete" -Target (Get-AccountList)
    $Dialog = Close-RamDialog "^Não$"
    AssertMatch $Dialog.Text "StorageAlt01" "confirmation"
    Assert ([bool](Account "StorageAlt01")) "removed after No"
}

if (-not $SkipBrowser) {
    Test "Browser" "Open Browser opens Chrome on roblox.com" {
        Select-Account "NightFalcon_07"
        Invoke-RamButton $script:Main "Abrir navegador"
        Wait-Until -Timeout 60 -Message "Chrome window" { Get-RamChrome | Where-Object MainWindowTitle -match "Roblox" }
        Close-AllRamDialogs
        Get-RamChrome | Stop-Process -Force
    }

    Test "Browser" "Add Account opens the Roblox login page" {
        Invoke-RamButton $script:Main "Adicionar conta"
        Wait-Until -Timeout 60 -Message "login page" { Get-RamChrome | Where-Object MainWindowTitle -match "Roblox" }
        Get-RamChrome | Stop-Process -Force
        Start-Sleep 1
    }
}
else { Skip "Browser" "Open Browser / Add Account" "browser tests skipped" }

#endregion

#region Menus

Test "Account menu" "Copy > Copy Username / Password / User:Pass / Profile / UserId" {
    $Expected = @("NightFalcon_07", "pw-one", "NightFalcon_07:pw-one", "https://www.roblox.com/users/1/profile", "1")
    for ($i = 0; $i -lt $Expected.Count; $i++) {
        Set-Clipboard "-"
        Open-AccountMenu
        Send-RamKeys ("Down,Down,Down,Right" + (",Down" * $i) + ",Return")
        Wait-Until -Message "clipboard $($Expected[$i])" { (Get-ClipText) -eq $Expected[$i] }
    }
}

Test "Account menu" "Sort Alphabetically asks first" {
    Open-AccountMenu
    Send-RamKeys "Down,Down,Down,Down,Return"
    $Dialog = Close-RamDialog "^Sim$"
    AssertMatch $Dialog.Text "alfabeticamente" "translated question"
}

Test "Account menu" "Quick Log In warns first" {
    Open-AccountMenu
    Send-RamKeys "Down,Down,Down,Down,Down,Return"
    Wait-RamDialog | Out-Null
    Close-RamDialog "^Não$" | Out-Null
}

Test "Account menu" "Groups > Toggle shows/hides the groups" {
    Open-AccountMenu
    Send-RamKeys "Down,Down,Down,Down,Down,Down,Right,Return"
    Shot "groups_off"
    Open-AccountMenu
    Send-RamKeys "Down,Down,Down,Down,Down,Down,Right,Return"
}

Test "Account menu" "Groups > Move Account To moves it to a new group" {
    Open-AccountMenu "PixelBuilderBR"
    Send-RamKeys "Down,Down,Down,Down,Down,Down,Right,Down,Return"
    $Prompt = Wait-RamWindow "Mover conta para grupo"
    $Box = Get-RamChildren $Prompt "EDIT" | Select-Object -First 1
    Set-RamText -Window $Prompt -Control $Box -Text "9Testes" | Out-Null
    Invoke-RamButton $Prompt "OK"
    Wait-Until -Message "group saved" { (Account "PixelBuilderBR").Group -eq "9Testes" }
}

Test "Account menu" "Groups > Copy Group" {
    Set-Clipboard "-"
    Open-AccountMenu "PixelBuilderBR"
    Send-RamKeys "Down,Down,Down,Down,Down,Down,Right,Down,Down,Return"
    Wait-Until -Message "group copied" { (Get-ClipText) -eq "9Testes" }
}

Test "Account menu" "Help > Groups and Help > Info show translated help" {
    Open-AccountMenu
    Send-RamKeys "Down,Down,Down,Down,Down,Down,Down,Right,Return"
    AssertMatch (Close-RamDialog).Text "grupos" "groups help"
    Open-AccountMenu
    Send-RamKeys "Down,Down,Down,Down,Down,Down,Down,Right,Down,Return"
    AssertMatch (Close-RamDialog).Text "GPLv3" "about"
}

Test "Account menu" "Dump Details writes a file for the account" {
    $Before = @(Get-Process notepad -ErrorAction SilentlyContinue | ForEach-Object Id)
    Open-AccountMenu "GrindBot_B"
    Send-RamKeys "Down,Down,Down,Down,Down,Down,Down,Down,Return"
    Wait-Until -Timeout 30 -Message "dump file" { Test-Path (Join-Path (Get-RamWorkDir) "AccountDumps\GrindBot_B.txt") }
    Start-Sleep 2
    Get-Process notepad -ErrorAction SilentlyContinue | Where-Object { $Before -notcontains $_.Id } | Stop-Process -Force
}

Test "Account menu" "Add Account / Remove Account from the menu" {
    Open-AccountMenu "GrindBot_B"
    Send-RamKeys "Down,Down,Return"
    AssertMatch (Close-RamDialog "^Não$").Text "GrindBot_B" "remove from the menu asks first"
}

Test "Add account menu" "Cookie(s) opens the import window, a bad cookie is reported" {
    Open-DropMenu $P.AddArrow
    Send-RamKeys "Down,Down,Down,Return"
    $Import = Wait-RamWindow "^Importar contas$"
    $Box = Get-RamChildren $Import "RICHEDIT" | Select-Object -First 1
    Set-RamText -Window $Import -Control $Box -Text "not-a-real-cookie" | Out-Null
    Invoke-RamButton $Import "Importar"
    Wait-Until -Timeout 20 -Message "import result" { (Get-RamText $Box) -match "Não foi possível adicionar not-a-real-cookie" }
    Shot "import" $Import
}

Test "Add account menu" "User:Pass asks for the accounts" {
    Open-DropMenu $P.AddArrow
    Send-RamKeys "Down,Down,Return"
    $Prompt = Wait-RamWindow "Importar por utilizador"
    Invoke-RamButton $Prompt "OK"
}

if (-not $SkipBrowser) {
    Test "Add account menu" "Custom (URL + JS) opens a browser" {
        Open-DropMenu $P.AddArrow
        Send-RamKeys "Down,Down,Down,Down,Return"
        $Prompt = Wait-RamWindow "Abrir navegador"
        Invoke-RamButton $Prompt "OK" # keeps https://roblox.com/
        $Prompt = Wait-RamWindow "Abrir navegador"
        Invoke-RamButton $Prompt "OK" # no script
        Wait-Until -Timeout 60 -Message "Chrome" { Get-RamChrome | Where-Object MainWindowTitle }
        Get-RamChrome | Stop-Process -Force
    }

    Test "Open browser menu" "URL opens a browser on that page" {
        Select-Account "NightFalcon_07"
        Open-DropMenu $P.OpenBrowserArrow
        Send-RamKeys "Down,Return"
        $Prompt = Wait-RamWindow "Abrir navegador"
        $Box = Get-RamChildren $Prompt "EDIT" | Select-Object -First 1
        Set-RamText -Window $Prompt -Control $Box -Text "https://www.roblox.com/charts" | Out-Null
        Invoke-RamButton $Prompt "OK"
        Wait-Until -Timeout 60 -Message "Chrome" { Get-RamChrome | Where-Object MainWindowTitle }
        Get-RamChrome | Stop-Process -Force
    }
}
else { Skip "Browser" "menu browsers" "browser tests skipped" }

Test "Open browser menu" "URL + Javascript warns first" {
    Select-Account "NightFalcon_07"
    Open-DropMenu $P.OpenBrowserArrow
    Send-RamKeys "Down,Down,Return"
    Wait-RamDialog | Out-Null
    Close-RamDialog "^Não$" | Out-Null
}

Test "Open browser menu" "Join Group asks for the group link" {
    Select-Account "NightFalcon_07"
    Open-DropMenu $P.OpenBrowserArrow
    Send-RamKeys "Down,Down,Down,Return"
    $Prompt = Wait-RamWindow "Abrir navegador"
    $Box = Get-RamChildren $Prompt "EDIT" | Select-Object -First 1
    Set-RamText -Window $Prompt -Control $Box -Text "not a link" | Out-Null
    Invoke-RamButton $Prompt "OK" # not a valid link, nothing opens
    Start-Sleep 1
    Assert (-not (Get-RamChrome)) "a browser opened for an invalid link"
}

#endregion

#region Language

Test "Language" "the flag menu switches the language right away and remembers it" {
    Open-DropMenu $P.Language -Button
    Send-RamKeys "Home,Return" # English
    Wait-Until -Message "English UI" { [bool](Get-RamControl $script:Main "Add Account") }
    AssertEq "en" (Get-RamSetting "Language") "saved language"
    Shot "main_en"
    Open-DropMenu $P.Language -Button
    Send-RamKeys "Home,Down,Return" # Português (Portugal)
    Wait-Until -Message "Portuguese UI" { [bool](Get-RamControl $script:Main "Adicionar conta") }
}

#endregion

#region Theme editor

Test "Theme editor" "every color and option is applied and saved right away" {
    Invoke-RamButton $script:Main -At $P.Theme
    $Theme = Wait-RamWindow "^Editor de tema$"
    $List = Get-RamChildren $Theme "LISTBOX" | Select-Object -First 1

    for ($i = 0; $i -lt 6; $i++) {
        Select-RamList $List $i
        Invoke-RamButton $Theme "Cor de fundo"
        Close-RamDialog "^(Cancelar|Cancel)$" | Out-Null   # the color picker
    }

    Select-RamList $List 1 # buttons
    $Style = Get-RamSetting "ButtonStyle" "RAMTheme.ini"
    Invoke-RamButton $Theme "Mudar estilo"
    Wait-Until -Message "style saved" { (Get-RamSetting "ButtonStyle" "RAMTheme.ini") -ne $Style }

    Select-RamList $List 0 # accounts
    $Headers = Get-RamSetting "ShowHeaders" "RAMTheme.ini"
    Invoke-RamButton $Theme "Ocultar cabeçalhos"
    Wait-Until -Message "headers saved" { (Get-RamSetting "ShowHeaders" "RAMTheme.ini") -ne $Headers }
    Invoke-RamButton $Theme "Ocultar cabeçalhos"

    Select-RamList $List 2 # forms
    $Dark = Get-RamSetting "DarkTopBar" "RAMTheme.ini"
    Invoke-RamButton $Theme "Barra de título escura"
    Wait-Until -Message "dark title bar saved" { (Get-RamSetting "DarkTopBar" "RAMTheme.ini") -ne $Dark }
    Assert (-not (Get-RamDialog)) "asked for a restart"
    Invoke-RamButton $Theme "Barra de título escura"

    Select-RamList $List 4 # labels
    $Transparent = Get-RamSetting "LabelsTransparent" "RAMTheme.ini"
    Invoke-RamButton $Theme "Fundo transparente"
    Wait-Until -Message "label background saved" { (Get-RamSetting "LabelsTransparent" "RAMTheme.ini") -ne $Transparent }
    Invoke-RamButton $Theme "Fundo transparente"
    Shot "theme" $Theme
}

#endregion

#region Settings

function Open-Settings { Invoke-RamButton $script:Main -At $P.Settings; Wait-RamWindow "^Definições$" }

function Test-Toggle([IntPtr] $Window, [string] $Text, [string] $Key, [scriptblock] $Check = $null) {
    $Before = Get-RamSetting $Key
    Invoke-RamButton $Window $Text
    Close-AllRamDialogs
    Wait-Until -Message "'$Text' saved" { (Get-RamSetting $Key) -ne $Before }
    if ($Check) { & $Check }
    Invoke-RamButton $Window $Text
    Close-AllRamDialogs
    Wait-Until -Message "'$Text' restored" { (Get-RamSetting $Key) -eq $Before -or ((Get-RamSetting $Key) -eq "false" -and -not $Before) }
}

Test "Settings" "General options are saved right away" {
    $Settings = Open-Settings
    Shot "settings" $Settings
    Test-Toggle $Settings "Abrir em sequência" "AsyncJoin"
    Test-Toggle $Settings "Desativar aviso de inatividade" "DisableAgingAlert"
    Test-Toggle $Settings "Ocultar aviso do Multi Roblox" "HideRbxAlert"
    Test-Toggle $Settings "Aleatório escolhe o servidor mais vazio" "ShuffleChoosesLowestServer"
    Test-Toggle $Settings "Renovar cookies automaticamente" "AutoCookieRefresh"
    Test-Toggle $Settings "Guardar palavras-passe" "SavePasswords"
}

Test "Settings" "Multi Roblox can be turned on and off without restarting" {
    $Settings = Open-Settings
    Test-Toggle $Settings "Multi Roblox" "EnableMultiRbx"
}

$RunKey = Get-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "Roblox Account Manager" -ErrorAction SilentlyContinue
if (-not $RunKey) {
    Test "Settings" "Run on Windows Startup adds and removes the startup entry" {
        $Settings = Open-Settings
        Invoke-RamButton $Settings "Iniciar com o Windows"
        Wait-Until -Message "startup entry" { (Get-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "Roblox Account Manager" -ErrorAction SilentlyContinue)."Roblox Account Manager" -like "$(Get-RamWorkDir)*" }
        Invoke-RamButton $Settings "Iniciar com o Windows"
        Wait-Until -Message "startup entry removed" { -not (Get-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "Roblox Account Manager" -ErrorAction SilentlyContinue) }
    }
}
else { Skip "Settings" "Run on Windows Startup" "a startup entry named 'Roblox Account Manager' already exists, it's left alone" }

Test "Settings" "language selector switches the language right away" {
    $Settings = Open-Settings
    $Combo = Get-RamChildren $Settings "COMBOBOX" | Select-Object -First 1
    Select-RamCombo $Combo 2 # Español
    Wait-Until -Message "Spanish UI" { [bool](Get-RamControl $script:Main "Añadir cuenta") }
    AssertEq "es" (Get-RamSetting "Language") "saved"
    $Settings = Wait-RamWindow "^Ajustes$"
    Select-RamCombo (Get-RamChildren $Settings "COMBOBOX" | Select-Object -First 1) 1 # back to Português (Portugal)
    Wait-Until -Message "Portuguese UI" { [bool](Get-RamControl $script:Main "Adicionar conta") }
}

Test "Settings" "text and number options are saved" {
    $Settings = Open-Settings
    $Region = Get-RamChildren $Settings "EDIT" | Where-Object { (Get-RamText $_) -match "city" } | Select-Object -First 1
    Set-RamText -Window $Settings -Control $Region -Text "<city> (<country>)" | Out-Null
    Wait-Until -Message "region format" { (Get-RamSetting "ServerRegionFormat") -eq "<city> (<country>)" }
    $Delay = Get-RamChildren $Settings "EDIT" | Where-Object { (Get-RamText $_) -eq "8" } | Select-Object -First 1
    Send-RamKeys "Up" -Target $Delay
    Wait-Until -Message "launch delay" { (Get-RamSetting "AccountJoinDelay") -eq "9" }
}

Test "Settings" "Reset Encryption Method shows the encryption choice" {
    $Settings = Open-Settings
    Invoke-RamButton $Settings "Mudar método de encriptação"
    Close-RamDialog "^Sim$" | Out-Null
    Wait-Until -Message "encryption choice" { [bool](Get-RamControl $script:Main "Encriptação padrão") }
    Invoke-RamButton $script:Main "Encriptação padrão"
    Wait-Until -Message "back to the accounts" { Test-RamEnabled (Button "Adicionar conta") }
}

Test "Settings" "Developer mode shows the developer menu items right away" {
    $Settings = Open-Settings
    Select-RamTab $Settings 1
    Invoke-RamButton $Settings "Ativar modo de programador"
    Wait-Until -Message "dev mode saved" { (Get-RamSetting "DevMode") -eq "true" }
    Set-Clipboard "-"
    Open-AccountMenu "NightFalcon_07"
    Send-RamKeys "Up,Up,Up,Return" # Copy Security Token (3rd from the bottom), only visible in developer mode
    Wait-Until -Message "token copied" { (Get-ClipText) -eq "fake-token-1" }
    Open-AccountMenu "NightFalcon_07"
    Send-RamKeys "Down,Down,Down,Down,Down,Down,Down,Down,Return" # View Fields
    $Fields = Wait-RamWindow "^Campos da conta$"
    Shot "fields" $Fields
    $Settings = Wait-RamWindow "^Definições$"
    Invoke-RamButton $Settings "Ativar modo de programador"
    Wait-Until -Message "dev mode off" { (Get-RamSetting "DevMode") -eq "false" }
}

Test "Settings" "web server starts, changes port and stops without restarting" {
    $Settings = Open-Settings
    Select-RamTab $Settings 1
    Invoke-RamButton $Settings "Ativar servidor web"
    Assert (-not (Get-RamDialog)) "asked for a restart"
    Wait-Until -Message "listening on 7963" { [bool](Get-NetTCPConnection -LocalPort 7963 -State Listen -ErrorAction SilentlyContinue) }
    AssertEq "true" (Invoke-WebRequest "http://localhost:7963/Running" -UseBasicParsing).Content "/Running"

    $Port = Get-RamChildren $Settings "EDIT" | Where-Object { (Get-RamText $_) -eq "7963" } | Select-Object -First 1
    Send-RamKeys "Up" -Target $Port
    Wait-Until -Message "listening on 7964" { [bool](Get-NetTCPConnection -LocalPort 7964 -State Listen -ErrorAction SilentlyContinue) }
    Assert (-not (Get-NetTCPConnection -LocalPort 7963 -State Listen -ErrorAction SilentlyContinue)) "old port still open"
    Send-RamKeys "Down" -Target $Port
    Wait-Until -Message "back on 7963" { [bool](Get-NetTCPConnection -LocalPort 7963 -State Listen -ErrorAction SilentlyContinue) }

    Invoke-RamButton $Settings "Ativar servidor web"
    Wait-Until -Message "stopped" { -not (Get-NetTCPConnection -LocalPort 7963 -State Listen -ErrorAction SilentlyContinue) }
}

Test "Settings" "web server password works right away and protects FollowUser/LaunchAccount" {
    $Settings = Open-Settings
    Select-RamTab $Settings 1
    Invoke-RamButton $Settings "Ativar servidor web"
    Wait-Until -Message "listening" { [bool](Get-NetTCPConnection -LocalPort 7963 -State Listen -ErrorAction SilentlyContinue) }

    function Status([string] $Path) { try { (Invoke-WebRequest "http://localhost:7963/$Path" -UseBasicParsing).StatusCode } catch { [int]$_.Exception.Response.StatusCode } }

    AssertEq 401 (Status "FollowUser?Account=NightFalcon_07&Username=Roblox") "no password configured"
    AssertEq 401 (Status "LaunchAccount?Account=NightFalcon_07&PlaceId=1") "LaunchAccount without password"

    $Box = Get-RamChildren $Settings "EDIT" | Where-Object { (Get-RamText $_) -eq "" } | Select-Object -Last 1
    Set-RamText -Window $Settings -Control $Box -Text "abcdef" | Out-Null
    Wait-Until -Message "password saved" { (Get-RamSetting "Password") -eq "abcdef" }

    AssertEq 401 (Status "FollowUser?Account=NightFalcon_07&Username=Roblox") "missing password"
    AssertEq 401 (Status "FollowUser?Account=NightFalcon_07&Username=Roblox&Password=wrong1") "wrong password"
    Assert ((Status "FollowUser?Account=NightFalcon_07&Username=Roblox&Password=abcdef") -ne 401) "right password refused"

    Set-RamText -Window $Settings -Control $Box -Text "" | Out-Null
    Invoke-RamButton $Settings "Ativar servidor web"
}

Test "Settings" "developer web server options are saved" {
    $Settings = Open-Settings
    Select-RamTab $Settings 1
    Test-Toggle $Settings "Exigir palavra-passe em todos os pedidos" "EveryRequestRequiresPassword"
    Test-Toggle $Settings "Permitir método GetCookie" "AllowGetCookie"
    Test-Toggle $Settings "Permitir método GetAccounts" "AllowGetAccounts"
    Test-Toggle $Settings "Permitir método LaunchAccount" "AllowLaunchAccount"
    Test-Toggle $Settings "Permitir métodos que alteram contas" "AllowAccountEditing"
    Test-Toggle $Settings "Não carregar imagens [menos RAM]" "DisableImages"
}

Test "Settings" "external connections fall back to this PC when permission isn't given" {
    $Settings = Open-Settings
    Select-RamTab $Settings 1
    Invoke-RamButton $Settings "Ativar servidor web"
    Wait-Until -Message "listening" { [bool](Get-NetTCPConnection -LocalPort 7963 -State Listen -ErrorAction SilentlyContinue) }
    Invoke-RamButton $Settings "Permitir ligações externas"
    $Asked = $false
    try { Wait-RamDialog -Timeout 5 | Out-Null; $Asked = $true } catch { }
    if ($Asked) { Close-RamDialog "^Não$" | Out-Null; Close-AllRamDialogs }   # no administrator prompt during tests
    Wait-Until -Message "still listening locally" { [bool](Get-NetTCPConnection -LocalPort 7963 -State Listen -ErrorAction SilentlyContinue) }
    Invoke-RamButton $Settings "Permitir ligações externas"
    Close-AllRamDialogs
    Invoke-RamButton $Settings "Ativar servidor web"
}

Test "Settings" "Miscellaneous options are saved right away" {
    $Settings = Open-Settings
    Select-RamTab $Settings 2
    Test-Toggle $Settings "Mostrar estado das contas" "ShowPresence"
    Test-Toggle $Settings "Desbloquear FPS" "UnlockFPS"
    $Rate = Get-RamChildren $Settings "EDIT" | Where-Object { (Get-RamText $_) -eq "5" } | Select-Object -First 1
    Send-RamKeys "Up" -Target $Rate
    Wait-Until -Message "presence rate" { (Get-RamSetting "PresenceUpdateRate") -eq "6" }
    Invoke-RamButton $Settings "Usar ClientAppSettings personalizado"
    Close-RamDialog "^(Cancelar|Cancel)$" | Out-Null # file picker
}

#endregion

#region Utilities window

function Open-Utilities { Invoke-RamButton $script:Main "Utilitários"; Wait-RamWindow "^Utilitários$" }

if (-not $SkipNetwork) {
    Test "Utilities" "Servers: Refresh loads the server list" {
        $Util = Open-Utilities
        Invoke-RamButton $Util "Atualizar"
        $List = Get-RamChildren $Util "SysListView32" | Select-Object -First 1
        Wait-Until -Timeout 30 -Message "servers" { (Get-RamListCount $List) -gt 0 }
        Shot "util_servers" $Util
    }

    Test "Utilities" "Servers: right click > Copy JobId" {
        $Util = Open-Utilities
        Select-RamTab $Util 0
        $List = Get-RamChildren $Util "SysListView32" | Select-Object -First 1
        $Point = Get-RamListItemPoint $Util $List
        Invoke-RamMouse $Util $Point.X $Point.Y
        Invoke-RamMouse $Util $Point.X $Point.Y -Button Right
        Set-Clipboard "-"
        Send-RamKeys "Down,Down,Return"
        Wait-Until -Message "job id copied" { (Get-ClipText) -match "^[0-9a-f-]{36}$" }
    }

    Test "Utilities" "Games: search fills the list" {
        $Util = Open-Utilities
        Select-RamTab $Util 1
        $Term = Get-RamChildren $Util "EDIT" | Select-Object -First 1
        Set-RamText -Window $Util -Control $Term -Text "obby" | Out-Null
        Invoke-RamButton $Util "Pesquisar"
        Start-Sleep 5
        Invoke-RamButton $Util "Vista em lista"
        Shot "util_games" $Util
        Invoke-RamButton $Util "Vista em lista"
    }

    Test "Utilities" "Favorites: favorite the current game" {
        $Util = Open-Utilities
        Select-RamTab $Util 2
        Invoke-RamButton $Util "Guardar jogo atual nos favoritos"
        Close-AllRamDialogs
        Invoke-RamButton $Util "Vista em lista"
        Shot "util_favorites" $Util
        Invoke-RamButton $Util "Vista em lista"
    }

    Test "Utilities" "Universe: view the games of a universe" {
        $Util = Open-Utilities
        Select-RamTab $Util 3
        $Boxes = @(Get-RamChildren $Util "EDIT")
        Set-RamText -Window $Util -Control $Boxes[-1] -Text "1686885941" | Out-Null
        Invoke-RamButton $Util "Ver jogos"
        Start-Sleep 4
        Invoke-RamButton $Util "Obter ID do universo"
        Shot "util_universe" $Util
    }

    Test "Utilities" "Outfits: view a player's outfits, wear custom asks for JSON" {
        $Util = Open-Utilities
        Select-RamTab $Util 4
        $Box = Get-RamChildren $Util "EDIT" | Select-Object -First 1
        Set-RamText -Window $Util -Control $Box -Text "Roblox" | Out-Null
        Invoke-RamButton $Util "Ver roupas"
        Start-Sleep 4
        Shot "util_outfits" $Util
        Select-Account "NightFalcon_07"
        $Util = Wait-RamWindow "^Utilitários$"
        Invoke-RamButton $Util "Vestir JSON"
        $Prompt = Wait-RamWindow "Vestir avatar"
        Invoke-RamButton $Prompt "OK"
    }
}
else { Skip "Utilities" "online tabs" "network tests skipped" }

Test "Utilities" "Watcher options are saved" {
    $Util = Open-Utilities
    Select-RamTab $Util 5
    foreach ($Option in @(
            @("Fechar se aparecer o menu inicial beta", "ExitOnBeta"),
            @("Fechar se ficar sem ligação ao servidor durante", "ExitIfNoConnection"),
            @("Guardar posições das janelas", "SaveWindowPositions"),
            @("Verificar sinal de saída", "VerifyDataModel"),
            @("Ignorar processos já abertos ao iniciar", "IgnoreExistingProcesses"),
            @("Fechar o Roblox se a memória for inferior a", "CloseRbxMemory"),
            @("Fechar o Roblox se o título da janela não for", "CloseRbxWindowTitle"))) {
        $Before = Get-RamSetting $Option[1]
        Invoke-RamButton $Util $Option[0]
        Wait-Until -Message $Option[1] { (Get-RamSetting $Option[1]) -ne $Before }
        Invoke-RamButton $Util $Option[0]
        Wait-Until -Message "$($Option[1]) restored" { (Get-RamSetting $Option[1]) -eq $Before -or (-not $Before -and (Get-RamSetting $Option[1]) -eq "false") }
    }
    Shot "util_watcher" $Util
}

$EulaAccepted = (Get-ItemProperty "HKCU:\SOFTWARE\Sysinternals\Handle" -Name EulaAccepted -ErrorAction SilentlyContinue).EulaAccepted -eq 1
if (-not $EulaAccepted) {
    Test "Utilities" "Enable Roblox Watcher asks for the Sysinternals EULA first" {
        $Util = Open-Utilities
        Select-RamTab $Util 5
        Invoke-RamButton $Util "Ativar monitor do Roblox"
        Close-RamDialog "^Não$" | Out-Null
        Wait-Until -Message "stays off" { (Get-RamSetting "Enabled") -ne "true" }
    }
}
else { Skip "Utilities" "Enable Roblox Watcher" "the Handle EULA is already accepted here, turning the watcher on could close a running Roblox" }

#endregion

#region Account utilities

Test "Account utilities" "every action reports the expired session instead of crashing" {
    Select-Account "NightFalcon_07"
    Invoke-RamButton $script:Main "Gerir conta"
    $Utils = Wait-RamWindow "^Gerir conta$"
    Shot "account_utils" $Utils

    foreach ($Action in "Mudar palavra-passe", "Mudar email", "Desbloquear") {
        Invoke-RamButton $Utils $Action
        Wait-RamDialog -Timeout 20 | Out-Null
        Close-AllRamDialogs
    }

    Invoke-RamButton $Utils "Terminar as outras sessões"
    Close-RamDialog "^Sim$" | Out-Null
    Close-AllRamDialogs

    foreach ($Action in "Bloquear", "Definir nome de exibição") {
        Invoke-RamButton $Utils $Action
        Wait-RamDialog -Timeout 20 | Out-Null
        Close-AllRamDialogs
    }

    Invoke-RamButton $Utils "Adicionar amigo" # fails quietly for an expired account
    Close-AllRamDialogs

    $Combo = Get-RamChildren $Utils "COMBOBOX" | Select-Object -First 1
    Select-RamCombo $Combo 0
    Close-AllRamDialogs
}

#endregion

#region Password protection & first run

Test "Encryption" "first run offers the encryption choice, password protection works" {
    Start-App -NoAccounts
    Assert ([bool](Get-RamControl $script:Main "Protegido por palavra-passe (recomendado)")) "encryption choice"
    Invoke-RamButton $script:Main "Protegido por palavra-passe (recomendado)"
    $Box = Set-RamText $script:Main 496 268 "teste1234"
    Start-Sleep 1
    Invoke-RamButton $script:Main "Continuar"
    Close-RamDialog | Out-Null # "confirm your password"
    Set-RamText -Window $script:Main -Control $Box -Text "teste1234" | Out-Null
    Start-Sleep 1
    Invoke-RamButton $script:Main "Continuar"
    Wait-Until -Message "encrypted file" { $B = [IO.File]::ReadAllBytes((Join-Path (Get-RamWorkDir) "AccountData.json")); [Text.Encoding]::ASCII.GetString($B, 0, 22) -eq "Roblox Account Manager" }

    # restart: wrong password is refused, the right one unlocks
    Stop-Process (Get-RamProcess) -Force; Start-Sleep 1
    $script:App = Start-Process -FilePath (Join-Path (Get-RamWorkDir) "RobloxAccountManager.exe") -WorkingDirectory (Get-RamWorkDir) -PassThru
    Wait-Until -Timeout 20 { [bool](Find-RamWindow "^Roblox Account Manager$") }
    $script:Main = Find-RamWindow "^Roblox Account Manager$"
    Start-Sleep 2
    Assert ([bool](Get-RamControl $script:Main "Acesso restrito")) "unlock screen"
    $Box = Set-RamText $script:Main 496 293 "errada99"
    Invoke-RamButton $script:Main "Continuar"
    AssertEq "Palavra-passe incorreta!" (Close-RamDialog -Timeout 20).Text "wrong password message"
    Set-RamText -Window $script:Main -Control $Box -Text "teste1234" | Out-Null
    Invoke-RamButton $script:Main "Continuar"
    Wait-Until -Timeout 20 -Message "unlocked" { Test-RamEnabled (Button "Adicionar conta") }
}

#endregion

#region Every language

foreach ($Language in "en", "pt-PT", "es", "fr", "de") {
    $Expected = @{ "en" = "Add Account"; "pt-PT" = "Adicionar conta"; "es" = "Añadir cuenta"; "fr" = "Ajouter un compte"; "de" = "Konto hinzufügen" }[$Language]
    Test "Languages" "the app starts in $Language" {
        Start-App -Language $Language
        Assert ([bool](Get-RamControl $script:Main $Expected)) "'$Expected' not shown"
        Shot "lang_$Language"
    }
}

Test "Shutdown" "closing the window exits the app and saves the place id" {
    $Process = Get-RamProcess
    [RamWin]::PostMessage($script:Main, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null # WM_CLOSE
    Wait-Until -Timeout 15 -Message "exit" { $Process.HasExited }
    Assert ([bool](Get-RamSetting "SavedPlaceId")) "place id saved"
}

#endregion

Stop-RamApp
if ($null -ne $SavedClipboard) { Set-Clipboard $SavedClipboard }

# a failed startup test may leave the entry pointing at the test copy behind
$Startup = (Get-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -ErrorAction SilentlyContinue)."Roblox Account Manager"
if ($Startup -and $Startup -like "$RunDir*") { Remove-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "Roblox Account Manager" }

$Passed = @($Results | Where-Object Result -eq "PASS").Count
$Failed = @($Results | Where-Object Result -eq "FAIL").Count
$Skipped = @($Results | Where-Object Result -eq "SKIP").Count

$Report = Join-Path $OutDir "report.md"
@("# UI test report", "", "$Passed passed, $Failed failed, $Skipped skipped ($(Get-Date -Format 'yyyy-MM-dd HH:mm'))", "", "| Group | Test | Result | Detail |", "|---|---|---|---|") +
($Results | ForEach-Object { "| $($_.Group) | $($_.Test) | $($_.Result) | $($_.Detail -replace '\|', '/') |" }) | Set-Content $Report -Encoding UTF8

Write-Host "`n$Passed passed, $Failed failed, $Skipped skipped. Report: $Report"
exit ([int]($Failed -gt 0))
