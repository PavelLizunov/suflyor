# Robust window enumerator — uses a C# class with a static List so the
# EnumWindows callback reliably collects results (PS scriptblock
# delegates sometimes drop output).
Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public class WinEnum {
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc cb, IntPtr l);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll", CharSet=CharSet.Auto)] static extern int GetWindowText(IntPtr h, StringBuilder s, int m);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    delegate bool EnumProc(IntPtr h, IntPtr l);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
    public static List<string> Collect(uint[] pids) {
        var outp = new List<string>();
        EnumWindows((h, l) => {
            uint wp; GetWindowThreadProcessId(h, out wp);
            foreach (var p in pids) if (p == wp) {
                var sb = new StringBuilder(256); GetWindowText(h, sb, 256);
                RECT r; GetWindowRect(h, out r);
                bool vis = IsWindowVisible(h);
                if (vis && (r.R - r.L) > 50)
                    outp.Add(String.Format("0x{0:x}|{1}|{2},{3},{4},{5}", h.ToInt64(), sb.ToString(), r.L, r.T, r.R, r.B));
                break;
            }
            return true;
        }, IntPtr.Zero);
        return outp;
    }
}
"@
$pids = (Get-Process overlay-host -ErrorAction SilentlyContinue | Select-Object -Expand Id)
if (-not $pids) { Write-Output "no overlay-host"; exit }
[uint32[]]$pidArr = $pids
[WinEnum]::Collect($pidArr) | ForEach-Object { Write-Output $_ }
