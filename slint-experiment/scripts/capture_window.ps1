# DPI-aware capture of a specific top-level window by title substring.
# Captures the on-screen pixels at the window's rect (+margin) so layered
# / transparent Slint windows are captured as composited (PrintWindow
# fails on skia-rendered layered windows).
param(
  [string]$TitleLike = "overlay-mvp (Slint)",
  [string]$Out = "C:\Users\x3d_mutant\Natively\overlay-mvp\slint-experiment\verify.png",
  [int]$Margin = 8
)

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class Cap {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr l);
  public delegate bool EnumWindowsProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  public struct RECT { public int Left, Top, Right, Bottom; }
  public static IntPtr Found = IntPtr.Zero;
  public static string Needle = "";
  public static RECT Rect;
  public static void Find() {
    Found = IntPtr.Zero;
    EnumWindows((h,l)=>{ if(IsWindowVisible(h)){ var sb=new StringBuilder(256); GetWindowText(h,sb,256);
      if(sb.ToString().Contains(Needle)){ RECT r; GetWindowRect(h,out r); Found=h; Rect=r; return false; } }
      return true; }, IntPtr.Zero);
  }
}
"@
[void][Cap]::SetProcessDPIAware()
[Cap]::Needle = $TitleLike
[Cap]::Find()
if ([Cap]::Found -eq [IntPtr]::Zero) { Write-Output "window not found: $TitleLike"; exit 1 }
$r = [Cap]::Rect
$x = $r.Left - $Margin; $y = $r.Top - $Margin
$w = ($r.Right - $r.Left) + 2*$Margin; $h = ($r.Bottom - $r.Top) + 2*$Margin
if ($x -lt 0) { $x = 0 }; if ($y -lt 0) { $y = 0 }

Add-Type -AssemblyName System.Drawing
$bmp = New-Object System.Drawing.Bitmap($w, $h)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($x, $y, 0, 0, $bmp.Size)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Output "saved $Out  window-rect=($($r.Left),$($r.Top),$($r.Right),$($r.Bottom))  crop=($x,$y,$w,$h)"
