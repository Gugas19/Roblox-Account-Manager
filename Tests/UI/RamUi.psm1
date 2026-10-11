# Helpers for driving Roblox Account Manager in UI tests.
# Everything is sent to the app's own windows (messages to window handles), never to screen coordinates,
# so other programs on the desktop (like a running Roblox) are never clicked.

Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public static class RamWin
{
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
    [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern IntPtr PostMessage(IntPtr h, int m, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, int m, IntPtr w, IntPtr l);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr h, int m, IntPtr w, string l);
    [DllImport("user32.dll")] public static extern IntPtr ChildWindowFromPointEx(IntPtr parent, POINT p, uint flags);
    [DllImport("user32.dll")] public static extern bool MapWindowPoints(IntPtr from, IntPtr to, ref POINT p, uint n);
    [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr h);
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }

    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern IntPtr SendMessageTimeout(IntPtr h, int m, IntPtr w, StringBuilder l, uint flags, uint timeout, out IntPtr result);

    // WM_GETTEXT, GetWindowText can't read the text of edit controls of another process
    public static string Text(IntPtr h) { var s = new StringBuilder(8192); IntPtr r; SendMessageTimeout(h, 0x000D, (IntPtr)8192, s, 2, 2000, out r); return s.ToString(); }
    public static string Class(IntPtr h) { var s = new StringBuilder(256); GetClassName(h, s, 256); return s.ToString(); }

    public static List<IntPtr> TopWindows(uint Pid)
    {
        var List = new List<IntPtr>();
        EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == Pid && IsWindowVisible(h)) List.Add(h); return true; }, IntPtr.Zero);
        return List;
    }

    public static List<IntPtr> Children(IntPtr Parent)
    {
        var List = new List<IntPtr>();
        EnumChildWindows(Parent, (h, l) => { List.Add(h); return true; }, IntPtr.Zero);
        return List;
    }

    public static IntPtr Deepest(IntPtr Parent, int X, int Y)
    {
        var Point = new POINT { X = X, Y = Y };
        while (true)
        {
            IntPtr Child = ChildWindowFromPointEx(Parent, Point, 1 | 2);
            if (Child == IntPtr.Zero || Child == Parent) return Parent;
            MapWindowPoints(Parent, Child, ref Point, 1);
            Parent = Child;
        }
    }

    public static POINT ToLocal(IntPtr From, IntPtr To, int X, int Y) { var P = new POINT { X = X, Y = Y }; MapWindowPoints(From, To, ref P, 1); return P; }

    // reading a list view of another process needs its structs in that process' memory
    [DllImport("kernel32.dll")] static extern IntPtr OpenProcess(uint access, bool inherit, uint pid);
    [DllImport("kernel32.dll")] static extern IntPtr VirtualAllocEx(IntPtr p, IntPtr a, int size, uint type, uint protect);
    [DllImport("kernel32.dll")] static extern bool VirtualFreeEx(IntPtr p, IntPtr a, int size, uint type);
    [DllImport("kernel32.dll")] static extern bool WriteProcessMemory(IntPtr p, IntPtr a, byte[] b, int size, out IntPtr written);
    [DllImport("kernel32.dll")] static extern bool ReadProcessMemory(IntPtr p, IntPtr a, byte[] b, int size, out IntPtr read);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr h);

    /// <summary>Center of the first list view row whose first column is Text, relative to Window (X = -1 when not found)</summary>
    public static POINT FindListItem(IntPtr List, IntPtr Window, string Text)
    {
        uint Pid; GetWindowThreadProcessId(List, out Pid);
        IntPtr Process = OpenProcess(0x0008 | 0x0010 | 0x0020, false, Pid);
        IntPtr Remote = VirtualAllocEx(Process, IntPtr.Zero, 4096, 0x3000, 0x04);
        IntPtr Written;
        try
        {
            int Count = (int)SendMessage(List, 0x1004, IntPtr.Zero, IntPtr.Zero); // LVM_GETITEMCOUNT
            for (int i = 0; i < Count; i++)
            {
                var Item = new byte[88]; // LVITEMW (x64): iSubItem 0, pszText at 24, cchTextMax at 32
                BitConverter.GetBytes((long)Remote + 512).CopyTo(Item, 24);
                BitConverter.GetBytes(256).CopyTo(Item, 32);
                WriteProcessMemory(Process, Remote, Item, Item.Length, out Written);
                int Length = (int)SendMessage(List, 0x1073, (IntPtr)i, Remote); // LVM_GETITEMTEXTW
                var Buffer = new byte[Length * 2];
                ReadProcessMemory(Process, Remote + 512, Buffer, Buffer.Length, out Written);
                if (Text != null && Encoding.Unicode.GetString(Buffer) != Text) continue; // null: the first row

                var Rect = new byte[16]; BitConverter.GetBytes(2).CopyTo(Rect, 0); // LVIR_LABEL
                WriteProcessMemory(Process, Remote, Rect, 16, out Written);
                SendMessage(List, 0x100E, (IntPtr)i, Remote); // LVM_GETITEMRECT
                ReadProcessMemory(Process, Remote, Rect, 16, out Written);
                int L = BitConverter.ToInt32(Rect, 0), T = BitConverter.ToInt32(Rect, 4), B = BitConverter.ToInt32(Rect, 12);
                return ToLocal(List, Window, L + 10, (T + B) / 2);
            }
            return new POINT { X = -1, Y = -1 };
        }
        finally { VirtualFreeEx(Process, Remote, 0, 0x8000); CloseHandle(Process); }
    }
}
"@

$script:App = $null
$script:WorkDir = $null

#region App lifetime

function Start-RamApp {
    <# Copies the build into a fresh test folder with fake accounts, starts it and waits for the main window #>
    param(
        [Parameter(Mandatory)] [string] $BuildDir,
        [Parameter(Mandatory)] [string] $WorkDir,
        [string] $Language = "pt-PT",
        [string[]] $ExtraSettings = @(),
        [switch] $NoAccounts,
        [switch] $Encrypted
    )

    $WorkDir = [IO.Path]::GetFullPath($WorkDir)
    $script:WorkDir = $WorkDir
    Stop-RamApp # also a copy left running by an earlier run
    if (Test-Path $WorkDir) { Remove-Item -Recurse -Force $WorkDir }
    Copy-Item -Recurse $BuildDir $WorkDir
    # never carry over real data/settings that might be in the build folder
    Get-ChildItem $WorkDir -File | Where-Object { $_.Name -match '^(AccountData|RAMSettings|RAMTheme|RecentGames|FavoriteGames|log\.txt|RBX Alt Manager\.exe)' } | Remove-Item -Force

    if (-not $NoAccounts) {
        New-Item -ItemType File "$WorkDir\NoEncryption.IUnderstandTheRisks.iautamor" | Out-Null
        $Now = (Get-Date).ToString("o"); $Old = (Get-Date).AddDays(-27).ToString("o")
        @(
            @{ Username = "NightFalcon_07"; UserID = 1; Group = "1Main"; Alias = "Main"; LastUse = $Now; SecurityToken = "fake-token-1"; Password = "pw-one" },
            @{ Username = "PixelBuilderBR"; UserID = 2; Group = "1Main"; Alias = "Builder"; LastUse = $Now; SecurityToken = "fake-token-2" },
            @{ Username = "StorageAlt01"; UserID = 3; Group = "2Storage"; Alias = "Bank"; LastUse = $Old; SecurityToken = "fake-token-3" },
            @{ Username = "StorageAlt02"; UserID = 4; Group = "2Storage"; Alias = ""; LastUse = $Now; SecurityToken = "fake-token-4" },
            @{ Username = "GrindBot_A"; UserID = 5; Group = "3Farm"; Alias = "Farm A"; LastUse = $Now; SecurityToken = "fake-token-5" },
            @{ Username = "GrindBot_B"; UserID = 6; Group = "3Farm"; Alias = "Farm B"; LastUse = $Now; SecurityToken = "fake-token-6" }
        ) | ConvertTo-Json | Set-Content -Encoding UTF8 "$WorkDir\AccountData.json"
    }

    @("[General]", "HideRbxAlert=true", "EnableMultiRbx=false", "Language=$Language") + $ExtraSettings | Set-Content "$WorkDir\RAMSettings.ini"

    $script:WorkDir = $WorkDir
    $script:App = Start-Process -FilePath "$WorkDir\RobloxAccountManager.exe" -WorkingDirectory $WorkDir -PassThru

    # the app restarts itself once when its config file doesn't match, follow the new process
    Wait-Until -Timeout 30 -Message "main window" { Get-RamProcess | Out-Null; [bool](Find-RamWindow "^Roblox Account Manager$") }
    Start-Sleep -Milliseconds 1500
}

function Get-RamProcess {
    $Running = Get-Process RobloxAccountManager -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "$script:WorkDir*" } | Select-Object -First 1
    if ($Running) { $script:App = $Running }
    return $script:App
}

function Stop-RamApp {
    Get-Process RobloxAccountManager -ErrorAction SilentlyContinue | Where-Object { $script:WorkDir -and $_.Path -like "$script:WorkDir*" } | Stop-Process -Force -ErrorAction SilentlyContinue
    Get-RamChrome | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500
}

function Get-RamChrome {
    <# Chrome processes started by the app (its own Chrome for Testing), never the user's browser #>
    Get-Process chrome -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "*\RobloxAccountManager\Chrome\*" }
}

function Get-RamWorkDir { $script:WorkDir }

#endregion

#region Windows & controls

function Wait-Until {
    param([Parameter(Mandatory, Position = 0)] [scriptblock] $Condition, [double] $Timeout = 10, [string] $Message = "condition")
    $End = (Get-Date).AddSeconds($Timeout)
    while ((Get-Date) -lt $End) {
        try { if (& $Condition) { return } } catch { }
        Start-Sleep -Milliseconds 250
    }
    throw "Timed out after $Timeout s waiting for $Message"
}

function Get-RamWindows {
    $P = Get-RamProcess
    if (-not $P) { return @() }
    [RamWin]::TopWindows($P.Id) | ForEach-Object { [pscustomobject]@{ Handle = $_; Title = [RamWin]::Text($_); Class = [RamWin]::Class($_) } }
}

function Find-RamWindow {
    param([Parameter(Position = 0)] [string] $Title)
    Get-RamWindows | Where-Object { $_.Title -match $Title -and $_.Class -notmatch '#32770' } | Select-Object -First 1 -ExpandProperty Handle
}

function Wait-RamWindow {
    param([Parameter(Position = 0)] [string] $Title, [double] $Timeout = 10)
    Wait-Until -Timeout $Timeout -Message "window '$Title'" { [bool](Find-RamWindow $Title) }
    Find-RamWindow $Title
}

function Get-RamControl {
    <# A child control of a window, by its exact text #>
    param([Parameter(Mandatory)] [IntPtr] $Window, [Parameter(Mandatory)] [string] $Text, [string] $ClassMatch = ".")
    [RamWin]::Children($Window) | Where-Object { [RamWin]::IsWindowVisible($_) -and ([RamWin]::Text($_) -replace '&', '') -eq $Text -and [RamWin]::Class($_) -match $ClassMatch } | Select-Object -First 1
}

function Get-RamControlAt {
    param([Parameter(Mandatory)] [IntPtr] $Window, [int] $X, [int] $Y)
    $Hit = [RamWin]::Deepest($Window, $X, $Y)
    if ($Hit -eq $Window) { throw "No control at $X,$Y" }
    $Hit
}

function Test-RamEnabled { param([IntPtr] $Control) [RamWin]::IsWindowEnabled($Control) }

function Invoke-RamButton {
    <# Clicks a button/checkbox by its text, or the control at a point of the window #>
    param([Parameter(Mandatory)] [IntPtr] $Window, [string] $Text, [int[]] $At)
    $Control = if ($Text) { Get-RamControl $Window $Text } else { Get-RamControlAt $Window $At[0] $At[1] }
    if (-not $Control) { throw "Control '$Text' not found" }
    [RamWin]::PostMessage($Control, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null # BM_CLICK
    Start-Sleep -Milliseconds 400
}

function Invoke-RamMouse {
    <# Left or right click inside the app window at a point relative to it #>
    param([Parameter(Mandatory)] [IntPtr] $Window, [int] $X, [int] $Y, [ValidateSet("Left", "Right")] [string] $Button = "Left")
    $Control = Get-RamControlAt $Window $X $Y
    $Local = [RamWin]::ToLocal($Window, $Control, $X, $Y)
    $Lp = [IntPtr](($Local.Y -shl 16) -bor ($Local.X -band 0xFFFF))
    [RamWin]::PostMessage($Control, 0x0200, [IntPtr]::Zero, $Lp) | Out-Null # WM_MOUSEMOVE
    if ($Button -eq "Left") { [RamWin]::PostMessage($Control, 0x0201, [IntPtr]1, $Lp) | Out-Null; [RamWin]::PostMessage($Control, 0x0202, [IntPtr]0, $Lp) | Out-Null }
    else { [RamWin]::PostMessage($Control, 0x0204, [IntPtr]2, $Lp) | Out-Null; [RamWin]::PostMessage($Control, 0x0205, [IntPtr]0, $Lp) | Out-Null }
    Start-Sleep -Milliseconds 500
}

function Send-RamKeys {
    <# Key presses, to the open menu (default) or to a control. Example: Send-RamKeys "Down,Down,Return" #>
    param([Parameter(Mandatory, Position = 0)] [string] $Keys, [IntPtr] $Target = [IntPtr]::Zero)
    $Codes = @{ Down = 0x28; Up = 0x26; Right = 0x27; Left = 0x25; Return = 0x0D; Escape = 0x1B; Delete = 0x2E; Tab = 0x09; Home = 0x24 }
    foreach ($Key in $Keys.Split(',')) {
        $To = $Target
        if ($To -eq [IntPtr]::Zero) {
            $To = Get-RamWindows | Where-Object { $_.Title -eq "" -and $_.Class -match "WindowsForms10.Window.808" } | Select-Object -First 1 -ExpandProperty Handle
            if (-not $To) { throw "No menu is open" }
        }
        [RamWin]::PostMessage($To, 0x0100, [IntPtr]$Codes[$Key], [IntPtr]1) | Out-Null
        [RamWin]::PostMessage($To, 0x0101, [IntPtr]$Codes[$Key], [IntPtr]0xC0000001) | Out-Null
        Start-Sleep -Milliseconds 300
    }
}

function Test-RamMenuOpen { [bool](Get-RamWindows | Where-Object { $_.Title -eq "" -and $_.Class -match "WindowsForms10.Window.808" }) }

function Set-RamText {
    <# Sets the text of an edit control (fires TextChanged) #>
    param([Parameter(Mandatory)] [IntPtr] $Window, [int] $X, [int] $Y, [string] $Text, [IntPtr] $Control = [IntPtr]::Zero)
    if ($Control -eq [IntPtr]::Zero) { $Control = Get-RamControlAt $Window $X $Y }
    [RamWin]::SendMessage($Control, 0x000C, [IntPtr]::Zero, $Text) | Out-Null # WM_SETTEXT
    Start-Sleep -Milliseconds 300
    $Control
}

function Get-RamText { param([IntPtr] $Control) [RamWin]::Text($Control) }

function Get-RamChildren {
    <# Visible child controls of a window whose class matches #>
    param([Parameter(Mandatory)] [IntPtr] $Window, [string] $ClassMatch = ".")
    [RamWin]::Children($Window) | Where-Object { [RamWin]::IsWindowVisible($_) -and [RamWin]::Class($_) -match $ClassMatch }
}

function Select-RamCombo {
    <# Selects an item of a combo box like a user would (CB_SETCURSEL + CBN_SELCHANGE) #>
    param([Parameter(Mandatory)] [IntPtr] $Combo, [int] $Index)
    [RamWin]::SendMessage($Combo, 0x014E, [IntPtr]$Index, [IntPtr]::Zero) | Out-Null
    $Id = [RamWin]::GetDlgCtrlID($Combo)
    [RamWin]::SendMessage([RamWin]::GetParent($Combo), 0x0111, [IntPtr](($Id -band 0xFFFF) -bor (1 -shl 16)), $Combo) | Out-Null
    Start-Sleep -Milliseconds 500
}

function Select-RamList {
    <# Selects an item of a list box like a user would (LB_SETCURSEL + LBN_SELCHANGE) #>
    param([Parameter(Mandatory)] [IntPtr] $List, [int] $Index)
    [RamWin]::SendMessage($List, 0x0186, [IntPtr]$Index, [IntPtr]::Zero) | Out-Null
    $Id = [RamWin]::GetDlgCtrlID($List)
    [RamWin]::SendMessage([RamWin]::GetParent($List), 0x0111, [IntPtr](($Id -band 0xFFFF) -bor (1 -shl 16)), $List) | Out-Null
    Start-Sleep -Milliseconds 300
}

function Select-RamTab {
    param([Parameter(Mandatory)] [IntPtr] $Window, [int] $Index)
    $Tabs = Get-RamChildren $Window "SysTabControl32" | Select-Object -First 1
    [RamWin]::SendMessage($Tabs, 0x1330, [IntPtr]$Index, [IntPtr]::Zero) | Out-Null # TCM_SETCURFOCUS
    Start-Sleep -Milliseconds 500
}

function Get-RamListItemPoint {
    <# Where a list view row is (by its first column text), relative to the window #>
    param([Parameter(Mandatory)] [IntPtr] $Window, [Parameter(Mandatory)] [IntPtr] $List, [string] $Text) # no text: the first row
    $Point = [RamWin]::FindListItem($List, $Window, $(if ($Text) { $Text } else { [NullString]::Value }))
    if ($Point.X -lt 0) { throw "Row '$Text' not found" }
    $Point
}

function Get-RamListCount {
    param([Parameter(Mandatory)] [IntPtr] $ListView)
    [int][RamWin]::SendMessage($ListView, 0x1004, [IntPtr]::Zero, [IntPtr]::Zero) # LVM_GETITEMCOUNT
}

#endregion

#region Dialogs

function Get-RamDialog {
    <# The open message box / task dialog: its title, texts and button handles #>
    $Dialog = Get-RamWindows | Where-Object { $_.Class -eq "#32770" } | Select-Object -First 1
    if (-not $Dialog) { return $null }
    $Children = [RamWin]::Children($Dialog.Handle)
    $Texts = $Children | Where-Object { [RamWin]::IsWindowVisible($_) -and [RamWin]::Class($_) -notmatch "Button" } | ForEach-Object { [RamWin]::Text($_) } | Where-Object { $_ }
    # task dialogs draw their text themselves, their accessible text is on the window
    [pscustomobject]@{ Handle = $Dialog.Handle; Title = $Dialog.Title; Text = ($Texts -join "`n"); Buttons = @($Children | Where-Object { [RamWin]::Class($_) -match "Button" }) }
}

function Wait-RamDialog {
    param([double] $Timeout = 10)
    Wait-Until -Timeout $Timeout -Message "a dialog" { [bool](Get-RamDialog) }
    Get-RamDialog
}

function Close-RamDialog {
    <# Presses a dialog button by its text (regex), returns the dialog that was open #>
    param([Parameter(Position = 0)] [string] $Button = "^(OK|Aceptar)$", [double] $Timeout = 10)
    $Dialog = Wait-RamDialog -Timeout $Timeout
    $Target = $Dialog.Buttons | Where-Object { ([RamWin]::Text($_) -replace '&', '') -match $Button } | Select-Object -First 1
    if ($Target) { [RamWin]::PostMessage($Target, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null }
    else {
        # task dialogs: their buttons are drawn by the dialog, close with Enter (Yes) or Escape (No/Cancel)
        $Key = if ($Button -match "Sim|Yes|OK|Sí|Oui|Ja") { 0x0D } else { 0x1B }
        [RamWin]::PostMessage($Dialog.Handle, 0x0100, [IntPtr]$Key, [IntPtr]1) | Out-Null
        [RamWin]::PostMessage($Dialog.Handle, 0x0101, [IntPtr]$Key, [IntPtr]0xC0000001) | Out-Null
    }
    Wait-Until -Timeout 5 -Message "dialog to close" { -not (Get-RamWindows | Where-Object { $_.Handle -eq $Dialog.Handle }) }
    Start-Sleep -Milliseconds 300
    $Dialog
}

function Close-AllRamDialogs {
    for ($i = 0; $i -lt 10 -and (Get-RamDialog); $i++) { try { Close-RamDialog "^(OK|Não|No|Cancelar|Cancel)$" -Timeout 2 | Out-Null } catch { } }
}

#endregion

#region Data

function Get-RamAccounts {
    <# The accounts file of the test folder (plain JSON thanks to NoEncryption) #>
    Get-Content "$script:WorkDir\AccountData.json" -Raw -Encoding UTF8 | ConvertFrom-Json
}

function Get-RamSetting {
    param([Parameter(Mandatory)] [string] $Key, [string] $File = "RAMSettings.ini")
    $Line = Get-Content "$script:WorkDir\$File" -ErrorAction SilentlyContinue | Where-Object { $_ -match "^$([regex]::Escape($Key))=" } | Select-Object -First 1
    if ($Line) { $Line.Substring($Key.Length + 1) } else { $null }
}

function Save-RamScreenshot {
    <# Screenshot of a window of the app (PrintWindow, works even when covered) #>
    param([Parameter(Mandatory)] [IntPtr] $Window, [Parameter(Mandatory)] [string] $Path)
    $Script = @"
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class Shot { [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint f);
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int a, out RECT r, int s);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; } }
'@ -ReferencedAssemblies System.Drawing
`$h = [IntPtr]$([int64]$Window); `$r = New-Object Shot+RECT; [Shot]::DwmGetWindowAttribute(`$h, 9, [ref]`$r, 16) | Out-Null
`$b = New-Object Drawing.Bitmap ([Math]::Max(1, `$r.R-`$r.L)), ([Math]::Max(1, `$r.B-`$r.T)); `$g = [Drawing.Graphics]::FromImage(`$b); `$dc = `$g.GetHdc()
[Shot]::PrintWindow(`$h, `$dc, 2) | Out-Null; `$g.ReleaseHdc(`$dc); `$g.Dispose(); `$b.Save('$Path', [Drawing.Imaging.ImageFormat]::Png)
"@
    powershell.exe -NoProfile -ExecutionPolicy Bypass -Command $Script | Out-Null
}

#endregion

Export-ModuleMember -Function *
