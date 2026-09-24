# drive.ps1 — live click-through driver for the running debug build.
#
# Drives ONLY the window of target\debug\il2_mission_utility.exe:
#   shot        -Out <png>              capture the client area (PrintWindow)
#   click       -X -Y                   left click at client coords
#   move        -X -Y                   move the pointer (hover)
#   drag        -X -Y -X2 -Y2           left drag
#   key         -Keys 1,2,Esc,F1,Enter  plain key presses
#   ctrl        -Keys G,Z,1             Ctrl+key
#   shiftclick  -X -Y                   Shift+left click
# Coordinates are client pixels = pixels of the `shot` image (100% scaling).
#
# Uses REAL input (SetCursorPos / mouse_event / keybd_event): winit ignores
# posted mouse messages. Every action first brings the app to the front and
# checks that the target pixel belongs to its window; otherwise it throws and
# sends nothing. It moves the user's real cursor, so run it only when the
# user says the screen is free. The computer-use screenshot hung in the
# 2026-09-24 session; PrintWindow capture works.
#
# Usage (PowerShell): & .\tools\ui-live\drive.ps1 -Action click -X 187 -Y 230
param(
    [string]$Action,
    [int]$X = 0, [int]$Y = 0, [int]$X2 = 0, [int]$Y2 = 0,
    [string]$Keys = '',
    [string]$Out = ''
)
Add-Type -AssemblyName System.Drawing
if (-not ('UiLiveDrv' -as [type])) {
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class UiLiveDrv {
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint f);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref PT p);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(PT p);
  [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h, uint f);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct PT { public int X, Y; }
}
"@
}
$proc = Get-Process il2_mission_utility -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -like '*\target\debug\il2_mission_utility.exe' -and $_.MainWindowTitle -eq 'IL-2 Group Generator' } |
    Select-Object -First 1
# The title check skips windowless `--probe-*` CLI runs of the same exe.
if (-not $proc) { throw 'the app window is not open (cargo build, then start target\debug\il2_mission_utility.exe)' }
$h = $proc.MainWindowHandle

function Set-Front {
    [void][UiLiveDrv]::SetForegroundWindow($h); Start-Sleep -Milliseconds 150
    if ([UiLiveDrv]::GetForegroundWindow() -ne $h) { throw 'the app is not the foreground window; nothing sent' }
}
function Get-VK([string]$k) {
    switch -regex ($k) {
        '^F(\d+)$' { return 0x6F + [int]$Matches[1] }
        '^Esc$' { return 0x1B }
        '^Enter$' { return 0x0D }
        '^Left$' { return 0x25 }
        '^Right$' { return 0x27 }
        '^.$' { return [int][char]$k.ToUpper() }
    }
    throw "unknown key $k"
}
# Point the real cursor at client (x, y): the app must be in front and own that pixel.
function Send-At([int]$x, [int]$y) {
    Set-Front
    $pt = New-Object UiLiveDrv+PT; $pt.X = $x; $pt.Y = $y
    [void][UiLiveDrv]::ClientToScreen($h, [ref]$pt)
    if ([UiLiveDrv]::GetAncestor([UiLiveDrv]::WindowFromPoint($pt), 2) -ne $h) { throw "point $x,$y is not over the app window" }
    [void][UiLiveDrv]::SetCursorPos($pt.X, $pt.Y); Start-Sleep -Milliseconds 60
}
function Send-Down { [UiLiveDrv]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 60 }
function Send-Up { [UiLiveDrv]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 300 }
function Send-Key([byte]$vk, [bool]$up) { [UiLiveDrv]::keybd_event($vk, 0, $(if ($up) { 2 } else { 0 }), [UIntPtr]::Zero); Start-Sleep -Milliseconds 50 }

switch ($Action) {
    'shot' {
        $r = New-Object UiLiveDrv+RECT; [void][UiLiveDrv]::GetClientRect($h, [ref]$r)
        $bmp = New-Object System.Drawing.Bitmap ($r.R), ($r.B)
        $g = [System.Drawing.Graphics]::FromImage($bmp); $dc = $g.GetHdc()
        [void][UiLiveDrv]::PrintWindow($h, $dc, 3); $g.ReleaseHdc($dc); $g.Dispose()
        $bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png); "$($r.R)x$($r.B)"
    }
    'click' { Send-At $X $Y; Send-Down; Send-Up }
    'move' { Send-At $X $Y; Start-Sleep -Milliseconds 300 }
    'drag' {
        Send-At $X $Y; Send-Down
        for ($k = 1; $k -le 12; $k++) { Send-At ([int]($X + ($X2 - $X) * $k / 12)) ([int]($Y + ($Y2 - $Y) * $k / 12)); Start-Sleep -Milliseconds 30 }
        Send-Up
    }
    'key' {
        Set-Front
        foreach ($k in $Keys.Split(',')) { $vk = [byte](Get-VK $k); Send-Key $vk $false; Send-Key $vk $true; Start-Sleep -Milliseconds 150 }
    }
    'ctrl' {
        Set-Front
        foreach ($k in $Keys.Split(',')) {
            $vk = [byte](Get-VK $k)
            Send-Key 0x11 $false; Send-Key $vk $false; Send-Key $vk $true; Send-Key 0x11 $true; Start-Sleep -Milliseconds 200
        }
    }
    'shiftclick' {
        Set-Front
        Send-Key 0x10 $false; Start-Sleep -Milliseconds 30
        Send-At $X $Y; Send-Down; Send-Up
        Send-Key 0x10 $true; Start-Sleep -Milliseconds 200
    }
    default { throw "unknown action '$Action' (shot, click, move, drag, key, ctrl, shiftclick)" }
}
