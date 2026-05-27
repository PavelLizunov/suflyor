$results = New-Object System.Collections.ArrayList
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class W {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
    [DllImport("user32.dll", CharSet=CharSet.Auto)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int max);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint p);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
}
"@

$pids = (Get-Process overlay-host -ErrorAction SilentlyContinue).Id
$pids | ForEach-Object { $script:targetPids = @() } -Begin {} -End {}
$script:targetPids = $pids

$script:found = New-Object System.Collections.ArrayList

$delegate = [W+EnumProc] {
    param([IntPtr]$h, [IntPtr]$l)
    $pidOut = [uint32]0
    [void][W]::GetWindowThreadProcessId($h, [ref]$pidOut)
    if ($script:targetPids -contains $pidOut) {
        $sb = New-Object System.Text.StringBuilder 256
        [void][W]::GetWindowText($h, $sb, 256)
        $r = New-Object W+RECT
        [void][W]::GetWindowRect($h, [ref]$r)
        $vis = [W]::IsWindowVisible($h)
        $obj = [PSCustomObject]@{
            Hwnd = [string]::Format("0x{0:x}", $h.ToInt64())
            Title = $sb.ToString()
            X = $r.L
            Y = $r.T
            W = $r.R - $r.L
            H = $r.B - $r.T
            Visible = $vis
        }
        [void]$script:found.Add($obj)
    }
    return $true
}

[void][W]::EnumWindows($delegate, [IntPtr]::Zero)
$script:found | Format-Table -AutoSize | Out-String | Write-Output
