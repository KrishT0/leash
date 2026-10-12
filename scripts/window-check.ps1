# Reports what the Leash process is doing from the outside: whether it runs, whether
# its window is really visible, and whether it holds a higher integrity level than the
# shell that runs this script.
#
# powershell -ExecutionPolicy Bypass -File scripts\window-check.ps1
#
# Get-Process reports the wrong window for a Tauri app, because the single-instance
# plugin owns a window of its own and that one is reported as the main window. This
# walks the real window list instead. A hidden window is still listed, with visibility
# False, which is what a logon start looks like.

Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public static class Windows {
    public delegate bool EnumProc(IntPtr handle, IntPtr param);

    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumProc callback, IntPtr param);

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr handle, out uint process);

    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr handle);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetWindowTextW(IntPtr handle, StringBuilder text, int count);

    public static List<string> WindowsOf(uint wanted) {
        var found = new List<string>();
        EnumWindows(delegate(IntPtr handle, IntPtr param) {
            uint process;
            GetWindowThreadProcessId(handle, out process);
            if (process != wanted) return true;
            var text = new StringBuilder(256);
            GetWindowTextW(handle, text, 256);
            found.Add(String.Format("{0} | visible {1}", text.ToString(), IsWindowVisible(handle)));
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
"@

$found = @(Get-Process -Name leash -ErrorAction SilentlyContinue)
if ($found.Count -eq 0) {
    Write-Host "Leash is not running."
    exit 0
}

foreach ($process in $found) {
    Write-Host ("Leash is running, pid {0}, started {1}" -f $process.Id, $process.StartTime)

    $path = ""
    try { $path = $process.Path } catch { }
    if ([string]::IsNullOrEmpty($path)) {
        Write-Host " its path cannot be read, so it holds a higher integrity level than this shell"
        Write-Host " that is what administrator rights look like from outside"
    } else {
        Write-Host (" its path reads back, so it runs at the same level as this shell")
    }

    $windows = [Windows]::WindowsOf([uint32]$process.Id)
    if ($windows.Count -eq 0) {
        Write-Host " it owns no windows at all"
    } else {
        Write-Host " its windows, as the window list sees them:"
        foreach ($window in $windows) { Write-Host ("   {0}" -f $window) }
    }

    $visible = $windows | Where-Object { $_ -like "Leash | visible True" }
    $hidden = $windows | Where-Object { $_ -like "Leash | visible False" }
    if ($visible) {
        Write-Host " the Leash window is VISIBLE, so this was not a quiet start"
    } elseif ($hidden) {
        Write-Host " the Leash window is hidden, which is what a logon start with start_minimized looks like"
    } else {
        Write-Host " there is no window titled Leash"
    }
}
