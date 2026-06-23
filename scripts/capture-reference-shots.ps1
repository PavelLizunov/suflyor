# =============================================================================
# capture-reference-shots.ps1  —  Этап 0 baseline harness (Track 3 redesign)
# =============================================================================
#
# WHAT THIS IS
#   Best-effort v1 baseline capture for the "zero-visual-change" redesign work
#   (docs/slint-design-system-and-safe-redesign-plan.md §7 Этап 0). It launches
#   the release overlay-host.exe and saves a PNG of every top-level window that
#   belongs to that process, so a later diff can confirm a token refactor (Этап
#   1+) did not move a single pixel. PURE TOOLING — it changes nothing in the
#   app; it only drives + photographs it.
#
# WHY CopyFromScreen (NOT computer-use / MCP screenshot)
#   This project's hard rule: computer-use screenshots MIS-RENDER the
#   transparent overlay's COLOURS (they show the bar dark when the live scheme
#   is light). Ground truth = System.Drawing.Graphics.CopyFromScreen at the
#   window's real HWND rect (Win32 EnumWindows + GetWindowRect, filtered by the
#   launched PID). See CLAUDE.md "visual verification" + memory
#   [[overlay-host-visual-verification]]. So we read each window's exact rect
#   off Win32 and blit THAT region from the live framebuffer.
#
# -----------------------------------------------------------------------------
# AUTOMATED COVERAGE (what this script captures on its own)
#   - The overlay BAR (always up at startup; title "suflyor (Slint)").
#   - Every other top-level overlay-host window currently visible, by PID.
#   - Surfaces openable via the registered GLOBAL HOTKEYS (read from
#     overlay_host.rs): F4 = KB palette, F1 = Help. (F8 = capture overlay is
#     intentionally SKIPPED by default — it freezes the whole desktop and grabs
#     the foreground; pass -IncludeCapture to attempt it.)
#   - ONE theme + ONE DPI + ONE monitor layout: whatever the machine is set to
#     RIGHT NOW (the script does not change scheme/scale/monitors).
#
# MANUAL COVERAGE (the operator MUST still do these by hand — out of scope for
# a v1 harness; see §7 Этап 0 + the §8 verification matrix)
#   - All 4 THEMES: Glacier, Graphite, Obsidian, Light Frost (Settings ▸
#     Interface ▸ scheme, or config color_scheme 0..3) — re-run per theme.
#   - All 3 DPI scales: Windows 100% / 125% / 150% — re-run per scale (log off /
#     on between changes so Slint picks up the new scale-factor).
#   - 1 vs 2 MONITORS (incl. the portrait secondary at negative-x) — the bar
#     pins to the primary; tiles use win32::pick_monitor.
#   - DATA STATES that need a live session / backend: empty, loading, error,
#     filled, STREAMING (start a session, ask F9, force an AI/STT error).
#   - A TILE in its states — and especially a tile with LONG MARKDOWN, a code
#     block, a table, and a long URL (§7: "тайл с длинным markdown"). Tiles are
#     spawned on demand (F9 / F6 / auto-detector), so they won't exist at a cold
#     boot — open one, then re-run with -SkipLaunch to photograph it.
#   - Settings / wizard / text-ask / recover-offer windows: opened from chips or
#     app state, not hotkeys — open the one you need, then -SkipLaunch.
#   - STEALTH on/off — by design stealth windows are EXCLUDED from capture
#     (WDA_EXCLUDEFROMCAPTURE), which also blocks this blit; capture with
#     stealth OFF, and verify stealth ON separately via a real screen-share.
#
# USAGE
#   pwsh -File scripts/capture-reference-shots.ps1
#   pwsh -File scripts/capture-reference-shots.ps1 -SkipLaunch        # photograph an already-running instance (e.g. after you opened a tile)
#   pwsh -File scripts/capture-reference-shots.ps1 -IncludeCapture    # also try the F8 capture overlay (freezes desktop briefly)
#   pwsh -File scripts/capture-reference-shots.ps1 -OutDir docs/reference-shots/glacier-150
#
# OUTPUT
#   One PNG per window at <OutDir>/<NN>-<sanitised-title>.png plus a
#   manifest.txt logging PID, HWND, title, rect and the run's theme/DPI context.
#   Robust: a missing/zero-size/off-screen window is logged and skipped, never
#   throws; the harness always tears down the instance it launched.
# =============================================================================

[CmdletBinding()]
param(
    # Where to drop the PNGs + manifest. Default groups tonight's baseline under
    # docs/reference-shots/. Use a per-theme/per-DPI subdir for the manual matrix.
    [string]$OutDir = "docs/reference-shots",
    # Photograph an already-running overlay-host instead of launching one. Use
    # this after manually opening a tile / Settings / wizard you want captured.
    [switch]$SkipLaunch,
    # Also attempt the F8 capture-region overlay (off by default: it freezes the
    # whole virtual desktop and steals the foreground).
    [switch]$IncludeCapture,
    # Seconds to wait after launch for the bar to appear + pin itself.
    [int]$BootWaitSec = 6,
    # Seconds to wait after sending a hotkey for the surface to open + paint.
    [double]$OpenWaitSec = 1.2
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

# ---- Resolve paths relative to the repo root (this script lives in scripts/) --
$RepoRoot = Split-Path -Parent $PSScriptRoot
$Exe = Join-Path $RepoRoot 'slint-experiment/target/release/overlay-host.exe'
if (-not [System.IO.Path]::IsPathRooted($OutDir)) { $OutDir = Join-Path $RepoRoot $OutDir }
if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Path $OutDir -Force | Out-Null }

# ---- Win32 window enumerator (static-list pattern: PS scriptblock delegates
#      drop callback output, so collect inside C# — same approach as
#      scripts/probe_bar.ps1). Filters to the target PIDs, visible, > 50px wide.
Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public class RefShotEnum {
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc cb, IntPtr l);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll", CharSet=CharSet.Auto)] static extern int GetWindowText(IntPtr h, StringBuilder s, int m);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    delegate bool EnumProc(IntPtr h, IntPtr l);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
    // Returns "hwnd|pid|title|L,T,R,B" for each matching top-level window.
    public static List<string> Collect(uint[] pids) {
        var outp = new List<string>();
        EnumWindows((h, l) => {
            uint wp; GetWindowThreadProcessId(h, out wp);
            foreach (var p in pids) {
                if (p == wp) {
                    if (IsWindowVisible(h)) {
                        var sb = new StringBuilder(256); GetWindowText(h, sb, 256);
                        RECT r; GetWindowRect(h, out r);
                        if ((r.R - r.L) > 50 && (r.B - r.T) > 10)
                            outp.Add(String.Format("0x{0:x}|{1}|{2}|{3},{4},{5},{6}",
                                h.ToInt64(), wp, sb.ToString(), r.L, r.T, r.R, r.B));
                    }
                    break;
                }
            }
            return true;
        }, IntPtr.Zero);
        return outp;
    }
}
"@

# ---- Helpers ----------------------------------------------------------------

function Get-OverlayWindows {
    param([uint32[]]$Pids)
    if (-not $Pids -or $Pids.Count -eq 0) { return @() }
    $rows = [RefShotEnum]::Collect($Pids)
    $list = New-Object System.Collections.ArrayList
    foreach ($row in $rows) {
        $parts = $row -split '\|', 4
        $coords = $parts[3] -split ','
        [void]$list.Add([PSCustomObject]@{
            Hwnd  = $parts[0]
            Pid   = [int]$parts[1]
            Title = $parts[2]
            X     = [int]$coords[0]
            Y     = [int]$coords[1]
            W     = [int]$coords[2] - [int]$coords[0]
            H     = [int]$coords[3] - [int]$coords[1]
        })
    }
    return $list
}

function Get-SafeName {
    param([string]$Title)
    if ([string]::IsNullOrWhiteSpace($Title)) { return 'untitled' }
    # Strip chars illegal in filenames + collapse runs to single dashes.
    $s = $Title -replace '[\\/:*?"<>|]', '-' -replace '\s+', '-'
    $s = $s.Trim('-').ToLowerInvariant()
    if ($s.Length -gt 48) { $s = $s.Substring(0, 48).Trim('-') }
    if ([string]::IsNullOrWhiteSpace($s)) { return 'untitled' }
    return $s
}

# Blit a window's rect from the live framebuffer. Returns $true on success.
# Never throws — clamps to the virtual screen and logs anything odd.
function Save-WindowShot {
    param(
        [Parameter(Mandatory)] $Win,
        [Parameter(Mandatory)] [string]$Path
    )
    try {
        if ($Win.W -le 0 -or $Win.H -le 0) {
            Write-Warning ("  skip '{0}' - non-positive size ({1}x{2})" -f $Win.Title, $Win.W, $Win.H)
            return $false
        }
        # Clamp the capture rect to the virtual desktop so an off-screen / pre-
        # create window (some surfaces park at huge negative coords before show)
        # can't blow up the bitmap.
        $vs = [System.Windows.Forms.SystemInformation]::VirtualScreen
        $x = [Math]::Max($Win.X, $vs.Left)
        $y = [Math]::Max($Win.Y, $vs.Top)
        $right  = [Math]::Min($Win.X + $Win.W, $vs.Right)
        $bottom = [Math]::Min($Win.Y + $Win.H, $vs.Bottom)
        $w = $right - $x
        $h = $bottom - $y
        if ($w -le 0 -or $h -le 0) {
            Write-Warning ("  skip '{0}' - off virtual screen (rect {1},{2} {3}x{4})" -f $Win.Title, $Win.X, $Win.Y, $Win.W, $Win.H)
            return $false
        }
        $bmp = New-Object System.Drawing.Bitmap $w, $h
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        try {
            $g.CopyFromScreen($x, $y, 0, 0, (New-Object System.Drawing.Size $w, $h))
            $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
        } finally {
            $g.Dispose(); $bmp.Dispose()
        }
        Write-Host "  saved $([System.IO.Path]::GetFileName($Path))  (${w}x${h} @ $x,$y)"
        return $true
    } catch {
        Write-Warning "  FAILED '$($Win.Title)': $($_.Exception.Message)"
        return $false
    }
}

# Send a virtual key to the foreground via SendKeys (global hotkeys are
# process-wide, so focus doesn't have to be on the overlay). F-keys map to
# SendKeys tokens "{F1}".."{F12}".
function Send-Hotkey {
    param([string]$Keys)
    try { [System.Windows.Forms.SendKeys]::SendWait($Keys) } catch {
        Write-Warning "  hotkey '$Keys' send failed: $($_.Exception.Message)"
    }
}

# ---- Launch (unless attaching to a running instance) ------------------------

$proc = $null
$launchedHere = $false
if (-not $SkipLaunch) {
    if (-not (Test-Path $Exe)) {
        throw "release binary not found: $Exe`n  build it first: cargo build --release --bin overlay-host (from slint-experiment/)"
    }
    Write-Host "launching $Exe ..."
    $proc = Start-Process -FilePath $Exe -PassThru
    $launchedHere = $true
    Write-Host "  pid $($proc.Id); waiting ${BootWaitSec}s for the bar to pin..."
    # Poll for the bar instead of a flat sleep — bail early once a window shows.
    $deadline = (Get-Date).AddSeconds($BootWaitSec)
    do {
        Start-Sleep -Milliseconds 400
        $wins = Get-OverlayWindows -Pids @([uint32]$proc.Id)
    } while ($wins.Count -eq 0 -and (Get-Date) -lt $deadline)
}

# Collect the live PID set (covers the -SkipLaunch case where we didn't spawn).
$pids = (Get-Process overlay-host -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
if (-not $pids) {
    if ($launchedHere -and $proc) { try { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue } catch {} }
    throw "no overlay-host process found to capture."
}
[uint32[]]$pidArr = @($pids)
Write-Host "target pids: $($pidArr -join ', ')"

# ---- Run context (for the manifest) -----------------------------------------

# Best-effort DPI of the primary screen via the device caps the .NET Graphics
# exposes; the live colour scheme isn't readable from here (it's in
# %APPDATA%\suflyor\config.json color_scheme) so we record where to look.
$dpi = try {
    $gfx = [System.Drawing.Graphics]::FromHwnd([IntPtr]::Zero)
    $d = [int]$gfx.DpiX; $gfx.Dispose(); $d
} catch { 'unknown' }
$cfgPath = Join-Path $env:APPDATA 'suflyor/config.json'

$manifest = New-Object System.Collections.ArrayList
[void]$manifest.Add("# Этап 0 reference-shot manifest")
[void]$manifest.Add("# captured: $(Get-Date -Format o)")
[void]$manifest.Add("# host primary DPI: $dpi (96 = 100%, 120 = 125%, 144 = 150%)")
[void]$manifest.Add("# live colour scheme: see color_scheme in $cfgPath (0 Glacier / 1 Graphite / 2 Obsidian / 3 Light Frost)")
[void]$manifest.Add("# NOTE: this run = ONE theme + ONE DPI + ONE monitor layout. The 4 themes,")
[void]$manifest.Add("#       3 DPI scales, 1-vs-2 monitors and the data/markdown states are MANUAL.")
[void]$manifest.Add("")
[void]$manifest.Add("idx`tpid`thwnd`trect(x,y,w,h)`tfile`ttitle")

$index = 0
$captured = 0

function Capture-Pass {
    param([string]$Label)
    $wins = Get-OverlayWindows -Pids $pidArr
    if ($wins.Count -eq 0) { Write-Warning "[$Label] no windows visible right now"; return }
    foreach ($w in $wins) {
        $script:index++
        $name = '{0:d2}-{1}' -f $script:index, (Get-SafeName $w.Title)
        $png = Join-Path $OutDir "$name.png"
        $ok = Save-WindowShot -Win $w -Path $png
        $file = if ($ok) { "$name.png" } else { '(skipped)' }
        if ($ok) { $script:captured++ }
        [void]$script:manifest.Add(("{0}`t{1}`t{2}`t{3},{4},{5},{6}`t{7}`t{8}" -f `
            $script:index, $w.Pid, $w.Hwnd, $w.X, $w.Y, $w.W, $w.H, $file, $w.Title))
    }
}

# ---- Pass 1: whatever is already on screen (the bar + any open window) -------
Write-Host "`n[pass 1] cold surfaces (bar + anything already open)"
Capture-Pass -Label 'cold'

# ---- Pass 2: hotkey-openable surfaces ---------------------------------------
# Map from overlay_host.rs registration: F4 = KB palette (toggle), F1 = Help
# (toggle). Both are stealth-aware but NOT WDA-excluded unless stealth is on, so
# they blit fine with stealth off. We toggle each open, capture, then toggle
# shut so the next one starts clean.
$hotkeySurfaces = @(
    @{ Key = '{F4}'; Name = 'KB palette' },
    @{ Key = '{F1}'; Name = 'Help' }
)
if ($IncludeCapture) {
    # F8 freezes the virtual desktop + grabs foreground; only when asked.
    $hotkeySurfaces += @{ Key = '{F8}'; Name = 'capture overlay'; Cancel = '{ESC}' }
}

foreach ($s in $hotkeySurfaces) {
    Write-Host "`n[pass 2] $($s.Name) via $($s.Key)"
    Send-Hotkey $s.Key
    Start-Sleep -Seconds $OpenWaitSec
    Capture-Pass -Label $s.Name
    # Close it again: explicit Cancel key if given (Esc for capture), else
    # re-send the toggle key.
    if ($s.ContainsKey('Cancel')) { Send-Hotkey $s.Cancel } else { Send-Hotkey $s.Key }
    Start-Sleep -Milliseconds 500
}

# ---- Write manifest + tear down ---------------------------------------------
$manifestPath = Join-Path $OutDir 'manifest.txt'
$manifest | Out-File -FilePath $manifestPath -Encoding utf8
Write-Host "`nmanifest -> $manifestPath"
Write-Host "captured $captured window shot(s) into $OutDir"

if ($launchedHere -and $proc) {
    Write-Host "stopping the instance we launched (pid $($proc.Id))..."
    try { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue } catch {}
    # Sweep any sibling overlay-host the launch may have left (rare).
    Get-Process overlay-host -ErrorAction SilentlyContinue |
        Where-Object { $_.StartTime -ge $proc.StartTime } |
        ForEach-Object { try { Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue } catch {} }
} else {
    Write-Host "left the (attached) instance running — you started it, you stop it."
}

Write-Host "`nDONE. Automated: bar + hotkey surfaces, current theme/DPI/monitor."
Write-Host "MANUAL still required: 4 themes, 3 DPI, 1-and-2 monitors, data states (empty/loading/error/filled/streaming), long-markdown tile. See the header + plan section 7 / section 8."
