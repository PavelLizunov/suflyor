# Taskbar exclusion audit — 2026-08-03

The defect was a one-frame Windows taskbar button created before the existing
post-creation Win32 cleanup ran. Slint's winit window attributes now request
taskbar exclusion before any application window is shown. The existing Win32
cleanup remains as a fallback for Settings and tile handles.

| Before | After |
|---|---|
| ![Transient Suflyor taskbar button reported by the tester](taskbar-before-user-crop.png) | ![Taskbar with the audited Suflyor process running and no Suflyor button](taskbar-after.png) |

The after state was captured from the Windows taskbar window while the exact
MCP-enabled audit binary was running. The tester also repeatedly opened tiles
and confirmed that the taskbar row no longer shifts or flashes.

The same process exposed two Slint windows through the embedded MCP server: the
hidden 1×1 capture surface and the visible 1200×64 overlay bar. The bar capture
below confirms the expected English UI and visible local STT/AI model identity.

![MCP-audited overlay bar](mcp-overlay-bar-after.png)

No API keys, transcripts, session names, URLs, or local paths are present in
these captures.
