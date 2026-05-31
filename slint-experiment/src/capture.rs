//! Screen capture for the vision feature (V2).
//!
//! Grabs a monitor (later: a region) via the Win32 BitBlt helpers in
//! [`crate::win32`], converts the top-down BGRA buffer to a downscaled JPEG,
//! and base64-encodes it into a `data:image/jpeg;base64,…` URI ready for the
//! vision endpoint. The only Win32 lives in `win32`; this module is the
//! image-processing + monitor-pick orchestration.

use base64::Engine;

/// Longest-edge cap before encoding. Matches Claude's per-image tile budget —
/// bigger wastes tokens without adding readable detail.
const MAX_EDGE: u32 = 1568;
/// JPEG quality (0-100). 80 is visually clean for screenshots at a fraction of
/// a PNG's size.
const JPEG_QUALITY: u8 = 80;

/// A raw captured frame: TOP-DOWN BGRA, 4 bytes/pixel (`bgra.len() == w*h*4`).
pub struct CapturedBgra {
    pub bgra: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Capture the full monitor currently under the mouse cursor. The caller is
/// responsible for hiding our own windows first
/// ([`crate::win32::hide_own_windows`]) so they don't appear in the shot.
pub fn capture_monitor_under_cursor() -> Result<CapturedBgra, Box<dyn std::error::Error>> {
    let monitors = crate::win32::enum_monitors();
    if monitors.is_empty() {
        return Err("no monitors found".into());
    }
    let (cx, cy) = crate::win32::cursor_pos();
    let mon = monitors
        .iter()
        .find(|m| cx >= m.left && cx < m.right && cy >= m.top && cy < m.bottom)
        .or_else(|| monitors.iter().find(|m| m.is_primary))
        .or_else(|| monitors.first())
        .ok_or("no monitor under cursor")?;
    let (w, h) = (mon.width(), mon.height());
    let bgra = crate::win32::capture_rect_bgra(mon.left, mon.top, w, h)?;
    Ok(CapturedBgra {
        bgra,
        width: w as u32,
        height: h as u32,
    })
}

/// Convert a TOP-DOWN BGRA buffer to a downscaled JPEG `data:` URI. CPU-bound
/// (per-pixel swizzle + resize + encode) — run it off the UI thread.
pub fn bgra_to_jpeg_data_url(
    bgra: &[u8],
    width: u32,
    height: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or("image dimensions overflow")?;
    if bgra.len() != expected || expected == 0 {
        return Err(format!("bgra len {} != expected {expected}", bgra.len()).into());
    }
    // BGRA → RGB (drop alpha, swap B/R).
    let mut rgb: Vec<u8> = Vec::with_capacity((width as usize) * (height as usize) * 3);
    for px in bgra.chunks_exact(4) {
        rgb.push(px[2]);
        rgb.push(px[1]);
        rgb.push(px[0]);
    }
    let img: image::RgbImage =
        image::RgbImage::from_raw(width, height, rgb).ok_or("rgb buffer size mismatch")?;
    // Downscale the longest edge to MAX_EDGE.
    let longest = width.max(height);
    let img = if longest > MAX_EDGE {
        let scale = f64::from(MAX_EDGE) / f64::from(longest);
        let nw = ((f64::from(width) * scale).round() as u32).max(1);
        let nh = ((f64::from(height) * scale).round() as u32).max(1);
        image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    // Encode JPEG.
    let mut jpeg: Vec<u8> = Vec::new();
    {
        use image::ImageEncoder;
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, JPEG_QUALITY).write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )?;
    }
    let b64 = base64::engine::general_purpose::STANDARD.encode(&jpeg);
    Ok(format!("data:image/jpeg;base64,{b64}"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn bgra_to_jpeg_produces_data_uri() {
        // 2x2 solid red BGRA (B=0, G=0, R=255, A=255).
        let one = [0u8, 0, 255, 255];
        let bgra: Vec<u8> = one.iter().cycle().take(2 * 2 * 4).copied().collect();
        let url = bgra_to_jpeg_data_url(&bgra, 2, 2).unwrap();
        assert!(url.starts_with("data:image/jpeg;base64,"));
        assert!(url.len() > "data:image/jpeg;base64,".len() + 8);
    }

    #[test]
    fn bgra_wrong_size_errors() {
        assert!(bgra_to_jpeg_data_url(&[0, 0, 0, 0], 2, 2).is_err());
        assert!(bgra_to_jpeg_data_url(&[], 0, 0).is_err());
    }
}
