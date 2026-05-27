//! Screenshot capture via xcap. Returns base64 data URL ready
//! to plug into AI MessageContent::Parts(ImageUrl).

use anyhow::{Context, Result};
use base64::Engine;
use std::io::Cursor;

/// Capture primary monitor, return data URL ("data:image/jpeg;base64,...").
pub fn capture_primary_jpeg() -> Result<String> {
    let monitors = xcap::Monitor::all().context("enumerate monitors")?;
    let monitor = monitors
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .or_else(|| xcap::Monitor::all().ok().and_then(|v| v.into_iter().next()))
        .context("no monitor")?;

    let image = monitor.capture_image().context("capture image")?;

    // Encode as JPEG q=70 (good compression for screenshots of text/code).
    let mut buf = Vec::with_capacity(256 * 1024);
    let mut writer = Cursor::new(&mut buf);
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, 70);
    image.write_with_encoder(encoder).context("encode jpeg")?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
    Ok(format!("data:image/jpeg;base64,{b64}"))
}
