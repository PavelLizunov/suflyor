//! Windows screenshot acquisition for the vision pipeline.

/// Capture a screen rectangle (physical virtual-screen coords) as TOP-DOWN
/// BGRA bytes (4 bytes/pixel; `len == w*h*4`) via a GDI BitBlt of the desktop
/// DC. NOTE: GDI capture IGNORES WDA_EXCLUDEFROMCAPTURE, so any of our own
/// windows inside the rect WILL appear — hide them first (`hide_own_windows`).
pub fn capture_rect_bgra(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::ffi::c_void;
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        HGDIOBJ, SRCCOPY,
    };
    if w <= 0 || h <= 0 {
        return Err(format!("invalid capture size {w}x{h}").into());
    }
    let buf_len = (w as usize)
        .checked_mul(h as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or("capture size overflow")?;
    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return Err("GetDC(screen) failed".into());
        }
        let mem = CreateCompatibleDC(Some(screen));
        if mem.is_invalid() {
            let _ = ReleaseDC(None, screen);
            return Err("CreateCompatibleDC failed".into());
        }
        let bmp = CreateCompatibleBitmap(screen, w, h);
        if bmp.is_invalid() {
            let _ = DeleteDC(mem);
            let _ = ReleaseDC(None, screen);
            return Err("CreateCompatibleBitmap failed".into());
        }
        let old = SelectObject(mem, HGDIOBJ(bmp.0));
        let blt = BitBlt(mem, 0, 0, w, h, Some(screen), x, y, SRCCOPY);
        // Deselect the bitmap from the DC BEFORE reading its bits: GetDIBits
        // requires the bitmap NOT be selected into any DC (documented contract).
        SelectObject(mem, old);
        // Skip the large (full-virtual-desktop) GetDIBits copy + buffer alloc
        // entirely when BitBlt failed — but still run the GDI cleanup below on
        // every path.
        let result: Result<Vec<u8>, Box<dyn std::error::Error>> = if blt.is_err() {
            Err("BitBlt failed".into())
        } else {
            let mut buf = vec![0u8; buf_len];
            let mut bi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h, // negative => top-down rows
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let lines = GetDIBits(
                mem,
                bmp,
                0,
                h as u32,
                Some(buf.as_mut_ptr().cast::<c_void>()),
                &mut bi,
                DIB_RGB_COLORS,
            );
            if lines == 0 {
                Err("GetDIBits returned 0 scanlines".into())
            } else {
                Ok(buf)
            }
        };
        // Free the remaining GDI objects on all paths.
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        let _ = DeleteDC(mem);
        let _ = ReleaseDC(None, screen);
        result
    }
}
