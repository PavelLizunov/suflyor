param([string]$Hex = "0x318004a")
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class W {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr h, int idx);
    [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X,Y; }
}
"@
$h = [IntPtr][System.Convert]::ToInt64($Hex, 16)
$ok = [W]::IsWindow($h)
$vis = [W]::IsWindowVisible($h)
$r = New-Object W+RECT
$gotRect = [W]::GetWindowRect($h, [ref]$r)
$exStyle = [W]::GetWindowLong($h, -20)
Write-Output ("hwnd=$Hex isWindow=$ok visible=$vis rect=({0},{1},{2},{3}) size=({4}x{5}) exStyle=0x{6:x}" -f $r.L, $r.T, $r.R, $r.B, ($r.R-$r.L), ($r.B-$r.T), $exStyle)

# Test WindowFromPoint at the rightmost area where pin button should be
$testX = $r.R - 80
$testY = $r.T + 18
$pt = New-Object W+POINT
$pt.X = $testX; $pt.Y = $testY
$wfp = [W]::WindowFromPoint($pt)
Write-Output ("WindowFromPoint($testX,$testY) -> 0x{0:x}" -f $wfp.ToInt64())

# Same for the spacer drag zone
$pt.X = $r.L + 180; $pt.Y = $r.T + 18
$wfp2 = [W]::WindowFromPoint($pt)
Write-Output ("WindowFromPoint($($pt.X),$($pt.Y)) [drag-zone] -> 0x{0:x}" -f $wfp2.ToInt64())
