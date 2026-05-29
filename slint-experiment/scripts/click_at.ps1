# DPI-aware synthetic left-click at absolute physical screen coords.
# Used to drive the Slint overlay for verification (computer-use can't
# grant a dev binary). Coords must match the DPI-aware capture script.
param([int]$X, [int]$Y, [int]$Clicks = 1)

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Click {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr e);
  public const uint LEFTDOWN = 0x0002, LEFTUP = 0x0004;
}
"@
[void][Click]::SetProcessDPIAware()
for ($i=0; $i -lt $Clicks; $i++) {
  [Click]::SetCursorPos($X, $Y)
  Start-Sleep -Milliseconds 60
  [Click]::mouse_event([Click]::LEFTDOWN, 0, 0, 0, [IntPtr]::Zero)
  Start-Sleep -Milliseconds 40
  [Click]::mouse_event([Click]::LEFTUP, 0, 0, 0, [IntPtr]::Zero)
  Start-Sleep -Milliseconds 120
}
Write-Output "clicked ($X,$Y) x$Clicks"
