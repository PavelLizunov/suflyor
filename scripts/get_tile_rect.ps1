Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class W {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
    [DllImport("user32.dll")] public static extern IntPtr FindWindow(string c, string n);
    [DllImport("user32.dll", CharSet=CharSet.Auto)] public static extern bool EnumWindows(EnumWindowsProc p, IntPtr l);
    [DllImport("user32.dll", CharSet=CharSet.Auto)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int max);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    public delegate bool EnumWindowsProc(IntPtr h, IntPtr l);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
}
"@
$targetPid = (Get-Process overlay-host -ErrorAction SilentlyContinue).Id
foreach ($p in $targetPid) {
    Write-Output "[pid $p]"
}

$cb = [W+EnumWindowsProc]{
    param($h, $l)
    $pidOut = 0
    [W]::GetWindowThreadProcessId($h, [ref]$pidOut) | Out-Null
    if ($targetPid -contains $pidOut) {
        $sb = New-Object System.Text.StringBuilder 256
        [W]::GetWindowText($h, $sb, 256) | Out-Null
        $r = New-Object W+RECT
        if ([W]::GetWindowRect($h, [ref]$r)) {
            $w = $r.R - $r.L
            $hh = $r.B - $r.T
            Write-Output ("hwnd=0x{0:x} title='{1}' rect=({2},{3}) size=({4}x{5})" -f $h.ToInt64(), $sb.ToString(), $r.L, $r.T, $w, $hh)
        }
    }
    return $true
}
[W]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
