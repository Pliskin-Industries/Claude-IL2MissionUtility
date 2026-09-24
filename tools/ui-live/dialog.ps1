# dialog.ps1 — answer the app's native file dialog (rfd) during a live run.
#
# Waits for a standard dialog (#32770) owned by the app's process, focuses the
# File name box (Alt+N), types the path and presses Enter. It sends REAL
# keyboard input, so run it only when the user says the screen is free.
# Pair with drive.ps1, which opens the dialog:
#   & .\tools\ui-live\drive.ps1 -Action ctrl -Keys O
#   & .\tools\ui-live\dialog.ps1 -Path C:\temp\Tanks.Group
# Works for open, multi-open (one path) and save dialogs.
param([string]$Path, [int]$TimeoutMs = 6000)
if (-not ('DlgDrv' -as [type])) {
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class DlgDrv {
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
}
"@
}
# Match by path: while a dialog is open the main window title changes.
$app = Get-Process il2_mission_utility -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -like '*\target\debug\il2_mission_utility.exe' -and $_.MainWindowHandle -ne 0 } |
    Select-Object -First 1
if (-not $app) { throw 'app not running' }
$deadline = (Get-Date).AddMilliseconds($TimeoutMs)
while ((Get-Date) -lt $deadline) {
    $h = [DlgDrv]::GetForegroundWindow()
    $sb = New-Object System.Text.StringBuilder 64
    [void][DlgDrv]::GetClassName($h, $sb, 64)
    $procId = 0; [void][DlgDrv]::GetWindowThreadProcessId($h, [ref]$procId)
    if ($sb.ToString() -eq '#32770' -and $procId -eq $app.Id) {
        Start-Sleep -Milliseconds 600
        $ws = New-Object -ComObject WScript.Shell
        # SendKeys treats + ^ % ~ ( ) { } [ ] specially; escape them.
        $esc = ($Path -replace '([+^%~(){}\[\]])', '{$1}')
        $ws.SendKeys('%n')
        Start-Sleep -Milliseconds 150
        $ws.SendKeys($esc)
        Start-Sleep -Milliseconds 200
        $ws.SendKeys('~')
        return 'answered'
    }
    Start-Sleep -Milliseconds 150
}
throw 'no file dialog appeared'
