# Send a single virtual-key press (down+up) via keybd_event.
# global-hotkey (RegisterHotKey) intercepts F-keys system-wide, so this
# fires the overlay's F3/F4/F9 handlers regardless of foreground window.
# Usage: send_key.ps1 -Vk 0x73   (0x73 = VK_F4)
param([int]$Vk = 0x73)

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Key {
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, IntPtr extra);
  public const uint KEYUP = 0x0002;
}
"@
[Key]::keybd_event([byte]$Vk, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 50
[Key]::keybd_event([byte]$Vk, 0, [Key]::KEYUP, [IntPtr]::Zero)
Write-Output "sent vk=0x$($Vk.ToString('X2'))"
