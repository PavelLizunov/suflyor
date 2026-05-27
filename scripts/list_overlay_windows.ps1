Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class W {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll", CharSet=CharSet.Auto)] public static extern bool EnumWindows(EnumProc p, IntPtr l);
    [DllImport("user32.dll", CharSet=CharSet.Auto)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int max);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint p);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
}
"@
$targetPid = (Get-Process overlay-host -ErrorAction SilentlyContinue).Id
if (-not $targetPid) { Write-Output "no overlay-host running"; exit 0 }
Write-Output "target pids: $targetPid"
[W]::EnumWindows({
    param([IntPtr]$h, [IntPtr]$l)
    $pidOut = [uint32]0
    [void][W]::GetWindowThreadProcessId($h, [ref]$pidOut)
    if ($targetPid -contains $pidOut) {
        $sb = New-Object System.Text.StringBuilder 256
        [void][W]::GetWindowText($h, $sb, 256)
        $r = New-Object W+RECT
        [void][W]::GetWindowRect($h, [ref]$r)
        $vis = [W]::IsWindowVisible($h)
        Write-Output ("hwnd=0x{0:x} title='{1}' rect=({2},{3},{4},{5}) visible={6}" -f $h.ToInt64(), $sb.ToString(), $r.L, $r.T, $r.R, $r.B, $vis)
    }
    return $true
}, [IntPtr]::Zero) | Out-Null
