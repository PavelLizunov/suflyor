param(
    [int]$X = 1150,
    [int]$Y = 95,
    [switch]$Drag,
    [int]$EndX = 1350,
    [int]$EndY = 250
)
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class MouseSim {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, uint dx, uint dy, uint dwData, IntPtr dwExtraInfo);
    public const uint LEFTDOWN = 0x0002;
    public const uint LEFTUP = 0x0004;
    public const uint MOVE = 0x0001;
}
"@
[MouseSim]::SetCursorPos($X, $Y)
Start-Sleep -Milliseconds 80
[MouseSim]::mouse_event([MouseSim]::LEFTDOWN, 0, 0, 0, [IntPtr]::Zero)
if ($Drag) {
    # Move in small steps to simulate a real drag
    $steps = 10
    for ($i = 1; $i -le $steps; $i++) {
        $cx = [int]($X + ($EndX - $X) * $i / $steps)
        $cy = [int]($Y + ($EndY - $Y) * $i / $steps)
        [MouseSim]::SetCursorPos($cx, $cy)
        Start-Sleep -Milliseconds 30
    }
}
Start-Sleep -Milliseconds 60
[MouseSim]::mouse_event([MouseSim]::LEFTUP, 0, 0, 0, [IntPtr]::Zero)
Write-Output "click_sent at ($X,$Y) drag=$Drag"
