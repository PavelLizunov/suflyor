# DPI-aware press-and-HOLD at absolute physical screen coords, then
# release after HoldMs. Verifies push-to-record (hold) buttons.
param([int]$X, [int]$Y, [int]$HoldMs = 2500)

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Hold {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr e);
  public const uint LEFTDOWN = 0x0002, LEFTUP = 0x0004;
}
"@
[void][Hold]::SetProcessDPIAware()
[Hold]::SetCursorPos($X, $Y)
Start-Sleep -Milliseconds 80
[Hold]::mouse_event([Hold]::LEFTDOWN, 0, 0, 0, [IntPtr]::Zero)
Write-Output "DOWN ($X,$Y) holding ${HoldMs}ms"
Start-Sleep -Milliseconds $HoldMs
[Hold]::mouse_event([Hold]::LEFTUP, 0, 0, 0, [IntPtr]::Zero)
Write-Output "UP released"
