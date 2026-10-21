//! Wayland screen and region capture via `grim`, `slurp`, and `hyprctl`.
//!
//! Captures display monitors or user-selected regions at scale 1 into TOP-DOWN
//! BGRA frames, with strict PPM parsing and bounded resource limits.

use std::error::Error;
use std::io::Read;
use std::process::{Command, Output, Stdio};

use crate::capture::CapturedBgra;

const MAX_OUTPUT_BYTES: usize = 256 * 1024 * 1024; // 256 MiB output cap
const MAX_DIMENSION: u32 = 32768;
const MAX_PIXELS: u64 = 64_000_000;

/// Capture the monitor currently under the cursor (or focused) on Hyprland.
///
/// Returns the captured BGRA frame and the monitor's global origin `(x, y)`.
pub fn capture_display() -> Result<(CapturedBgra, i32, i32), Box<dyn Error>> {
    let mon_out = run_cmd("10s", "hyprctl", &["-j", "monitors"])?;
    if !mon_out.status.success() {
        if mon_out.status.code() == Some(124) {
            return Err("hyprctl monitors timed out after 10s".into());
        }
        return Err(format!("hyprctl monitors failed: exit {:?}", mon_out.status.code()).into());
    }

    let monitors = parse_monitors_json(&mon_out.stdout)?;
    if monitors.is_empty() {
        return Err("no monitors found".into());
    }

    let cursor = run_cmd("10s", "hyprctl", &["-j", "cursorpos"])
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| parse_cursorpos_output(&o.stdout))
        .or_else(|| {
            run_cmd("10s", "hyprctl", &["cursorpos"])
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| parse_cursorpos_output(&o.stdout))
        });

    let mon = select_monitor(&monitors, cursor)?;
    let grim_out = run_cmd("10s", "grim", &["-s", "1", "-o", &mon.name, "-t", "ppm", "-"])?;
    if !grim_out.status.success() {
        if grim_out.status.code() == Some(124) {
            return Err("grim timed out after 10s".into());
        }
        let err = String::from_utf8_lossy(&grim_out.stderr);
        return Err(format!("grim failed to capture monitor {}: {}", mon.name, err.trim()).into());
    }

    let bgra = parse_ppm(&grim_out.stdout)?;
    Ok((bgra, mon.x, mon.y))
}

/// Prompt the user to select a screen region via `slurp`, then capture it via `grim`.
///
/// Returns the captured BGRA frame and the region's global origin `(x, y)`.
pub fn capture_region() -> Result<(CapturedBgra, i32, i32), Box<dyn Error>> {
    let slurp_out = run_cmd("120s", "slurp", &[])?;
    if !slurp_out.status.success() {
        if slurp_out.status.code() == Some(124) {
            return Err("slurp timed out after 120s".into());
        }
        return Err("selection cancelled".into());
    }

    let slurp_str = std::str::from_utf8(&slurp_out.stdout)
        .map_err(|_| "invalid UTF-8 in slurp output")?;
    let (x, y, w, h) = parse_slurp_output(slurp_str)?;
    let geom = format!("{x},{y} {w}x{h}");

    let grim_out = run_cmd("10s", "grim", &["-s", "1", "-g", &geom, "-t", "ppm", "-"])?;
    if !grim_out.status.success() {
        if grim_out.status.code() == Some(124) {
            return Err("grim timed out after 10s".into());
        }
        let err = String::from_utf8_lossy(&grim_out.stderr);
        return Err(format!("grim failed to capture region: {}", err.trim()).into());
    }

    let bgra = parse_ppm(&grim_out.stdout)?;
    Ok((bgra, x, y))
}

fn run_cmd(timeout_sec: &str, program: &str, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let mut child = Command::new("timeout")
        .args(["--signal=KILL", timeout_sec, program]).args(args)
        .stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
    let mut bytes = Vec::new();
    let result = child.stdout.take().ok_or("capture pipe unavailable")?
        .take(MAX_OUTPUT_BYTES as u64 + 1).read_to_end(&mut bytes);
    if result.is_err() || bytes.len() > MAX_OUTPUT_BYTES {
        let _ = child.kill();
        let _ = child.wait();
        return Err("capture output exceeded its limit or could not be read".into());
    }
    Ok(Output { status: child.wait()?, stdout: bytes, stderr: Vec::new() })
}

struct MonitorInfo {
    name: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    focused: bool,
}

fn parse_monitors_json(json_bytes: &[u8]) -> Result<Vec<MonitorInfo>, Box<dyn Error>> {
    let val: serde_json::Value = serde_json::from_slice(json_bytes)?;
    let arr = val.as_array().ok_or("expected json array from hyprctl monitors")?;
    let mut monitors = Vec::with_capacity(arr.len());
    for item in arr {
        let name = item.get("name").and_then(|v| v.as_str()).ok_or("missing monitor name")?.to_string();
        let x = item.get("x").and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))).ok_or("missing monitor x")? as i32;
        let y = item.get("y").and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))).ok_or("missing monitor y")? as i32;
        let width = item.get("width").and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))).ok_or("missing monitor width")? as i32;
        let height = item.get("height").and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))).ok_or("missing monitor height")? as i32;
        let scale = item.get("scale").and_then(|v| v.as_f64()).filter(|s| *s > 0.0).ok_or("invalid display scale")?;
        let (width, height) = if item.get("transform").and_then(|v| v.as_u64()).unwrap_or(0) % 2 == 1 { (height, width) } else { (width, height) };
        let width = (f64::from(width) / scale).round() as i32;
        let height = (f64::from(height) / scale).round() as i32;
        let focused = item.get("focused").and_then(|v| v.as_bool()).unwrap_or(false);
        monitors.push(MonitorInfo { name, x, y, width, height, focused });
    }
    Ok(monitors)
}

fn parse_cursorpos_output(output: &[u8]) -> Option<(i32, i32)> {
    if let Ok(val) = serde_json::from_slice::<serde_json::Value>(output) {
        if let (Some(x), Some(y)) = (
            val.get("x").and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))),
            val.get("y").and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))),
        ) {
            return Some((x as i32, y as i32));
        }
    }
    let text = std::str::from_utf8(output).ok()?;
    let parts: Vec<&str> = text.trim().split(',').collect();
    if parts.len() == 2 {
        if let (Ok(x), Ok(y)) = (parts[0].trim().parse::<i32>(), parts[1].trim().parse::<i32>()) {
            return Some((x, y));
        }
    }
    None
}

fn select_monitor<'a>(
    monitors: &'a [MonitorInfo],
    cursor: Option<(i32, i32)>,
) -> Result<&'a MonitorInfo, Box<dyn Error>> {
    if let Some((cx, cy)) = cursor {
        if let Some(m) = monitors.iter().find(|m| {
            cx >= m.x && cx < m.x + m.width && cy >= m.y && cy < m.y + m.height
        }) {
            return Ok(m);
        }
    }
    if let Some(m) = monitors.iter().find(|m| m.focused) {
        return Ok(m);
    }
    monitors.first().ok_or_else(|| "no monitor available".into())
}

fn parse_slurp_output(s: &str) -> Result<(i32, i32, u32, u32), Box<dyn Error>> {
    let trimmed = s.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(format!("invalid slurp output format: {trimmed:?}").into());
    }

    let coords: Vec<&str> = parts[0].split(',').collect();
    if coords.len() != 2 {
        return Err(format!("invalid coordinates in slurp output: {:?}", parts[0]).into());
    }
    let x: i32 = coords[0].parse().map_err(|_| format!("invalid x coordinate: {:?}", coords[0]))?;
    let y: i32 = coords[1].parse().map_err(|_| format!("invalid y coordinate: {:?}", coords[1]))?;

    let dims: Vec<&str> = parts[1].split('x').collect();
    if dims.len() != 2 {
        return Err(format!("invalid dimensions in slurp output: {:?}", parts[1]).into());
    }
    let w: u32 = dims[0].parse().map_err(|_| format!("invalid width: {:?}", dims[0]))?;
    let h: u32 = dims[1].parse().map_err(|_| format!("invalid height: {:?}", dims[1]))?;

    if w == 0 || h == 0 {
        return Err("selection dimensions must be positive non-zero".into());
    }
    if w > MAX_DIMENSION || h > MAX_DIMENSION {
        return Err(format!("selection dimension exceeds maximum of {MAX_DIMENSION}").into());
    }
    let pixels = (w as u64).checked_mul(h as u64).ok_or("pixel count overflow")?;
    if pixels > MAX_PIXELS {
        return Err(format!("selection exceeds maximum of {MAX_PIXELS} pixels").into());
    }

    Ok((x, y, w, h))
}

fn skip_header_ws_and_comments(data: &[u8], idx: &mut usize) {
    while *idx < data.len() {
        let b = data[*idx];
        if b == b'#' {
            *idx += 1;
            while *idx < data.len() && data[*idx] != b'\n' {
                *idx += 1;
            }
            if *idx < data.len() && data[*idx] == b'\n' {
                *idx += 1;
            }
        } else if b.is_ascii_whitespace() {
            *idx += 1;
        } else {
            break;
        }
    }
}

fn next_header_token<'a>(data: &'a [u8], idx: &mut usize) -> Result<&'a [u8], Box<dyn Error>> {
    skip_header_ws_and_comments(data, idx);
    if *idx >= data.len() {
        return Err("unexpected EOF in PPM header".into());
    }
    let start = *idx;
    while *idx < data.len() && !data[*idx].is_ascii_whitespace() && data[*idx] != b'#' {
        *idx += 1;
    }
    if start == *idx {
        return Err("empty token in PPM header".into());
    }
    Ok(&data[start..*idx])
}

fn parse_ppm(data: &[u8]) -> Result<CapturedBgra, Box<dyn Error>> {
    let mut idx = 0;
    let magic = next_header_token(data, &mut idx)?;
    if magic != b"P6" {
        return Err("invalid PPM magic, expected P6".into());
    }

    let w_tok = next_header_token(data, &mut idx)?;
    let w: u32 = std::str::from_utf8(w_tok)
        .map_err(|_| "invalid width token")?
        .parse()
        .map_err(|_| "invalid width value")?;

    let h_tok = next_header_token(data, &mut idx)?;
    let h: u32 = std::str::from_utf8(h_tok)
        .map_err(|_| "invalid height token")?
        .parse()
        .map_err(|_| "invalid height value")?;

    let maxval_tok = next_header_token(data, &mut idx)?;
    let maxval: u32 = std::str::from_utf8(maxval_tok)
        .map_err(|_| "invalid maxval token")?
        .parse()
        .map_err(|_| "invalid maxval value")?;

    if maxval != 255 {
        return Err(format!("unsupported maxval: {maxval}, expected 255").into());
    }

    // Netpbm requires exactly one whitespace character delimiter after maxval.
    // Do NOT skip leading raster whitespace bytes.
    if idx >= data.len() || !data[idx].is_ascii_whitespace() {
        return Err("missing whitespace delimiter after maxval".into());
    }
    idx += 1;

    if w == 0 || h == 0 {
        return Err("PPM dimensions must be positive non-zero".into());
    }
    if w > MAX_DIMENSION || h > MAX_DIMENSION {
        return Err(format!("PPM dimension exceeds maximum of {MAX_DIMENSION}").into());
    }
    let num_pixels = (w as u64).checked_mul(h as u64).ok_or("pixel count overflow")?;
    if num_pixels > MAX_PIXELS {
        return Err(format!("PPM pixel count {num_pixels} exceeds limit of {MAX_PIXELS}").into());
    }

    let expected_bytes = (num_pixels as usize)
        .checked_mul(3)
        .ok_or("byte count overflow")?;
    let raster = &data[idx..];
    if raster.len() != expected_bytes {
        return Err(format!(
            "PPM raster size mismatch: expected {expected_bytes} bytes, got {}",
            raster.len()
        ).into());
    }

    let mut bgra = Vec::with_capacity((num_pixels as usize) * 4);
    for px in raster.chunks_exact(3) {
        bgra.push(px[2]); // B
        bgra.push(px[1]); // G
        bgra.push(px[0]); // R
        bgra.push(255);   // A
    }

    Ok(CapturedBgra {
        bgra,
        width: w,
        height: h,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn test_malformed_ppm() {
        // Bad magic
        assert!(parse_ppm(b"P5 1 1 255\n\x00").is_err());
        assert!(parse_ppm(b"P3 1 1 255\n0 0 0").is_err());
        assert!(parse_ppm(b"XYZ 1 1 255\n\x00\x00\x00").is_err());

        // Incomplete / EOF headers
        assert!(parse_ppm(b"").is_err());
        assert!(parse_ppm(b"P6").is_err());
        assert!(parse_ppm(b"P6 10").is_err());
        assert!(parse_ppm(b"P6 10 10").is_err());
        assert!(parse_ppm(b"P6 10 10 255").is_err()); // No delimiter after maxval

        // Bad numeric tokens
        assert!(parse_ppm(b"P6 abc 10 255\n").is_err());
        assert!(parse_ppm(b"P6 10 def 255\n").is_err());
        assert!(parse_ppm(b"P6 10 10 xyz\n").is_err());

        // Zero dimensions
        assert!(parse_ppm(b"P6 0 10 255\n").is_err());
        assert!(parse_ppm(b"P6 10 0 255\n").is_err());

        // Maxval not 255
        assert!(parse_ppm(b"P6 1 1 256\n\x00\x00\x00").is_err());
        assert!(parse_ppm(b"P6 1 1 128\n\x00\x00\x00").is_err());
    }

    #[test]
    fn test_ppm_comments_and_whitespace() {
        let input = b"# First comment\nP6\n# Second comment\n 2 \t # inline comment\n 1 \n 255\n\x01\x02\x03\x04\x05\x06";
        let captured = parse_ppm(input).unwrap();
        assert_eq!(captured.width, 2);
        assert_eq!(captured.height, 1);
        // BGRA: pixel 1: B=3, G=2, R=1, A=255; pixel 2: B=6, G=5, R=4, A=255
        assert_eq!(captured.bgra, vec![3, 2, 1, 255, 6, 5, 4, 255]);
    }

    #[test]
    fn test_first_pixel_whitespace() {
        // The first raster bytes are ASCII whitespace: space (0x20), tab (0x09), newline (0x0A)
        let mut data = Vec::from(b"P6 1 1 255\n".as_slice());
        data.extend_from_slice(&[b' ', b'\t', b'\n']); // R=32, G=9, B=10
        let captured = parse_ppm(&data).unwrap();
        assert_eq!(captured.width, 1);
        assert_eq!(captured.height, 1);
        assert_eq!(captured.bgra, vec![10, 9, 32, 255]);
    }

    #[test]
    fn test_truncated_ppm() {
        // 2x2 requires 12 bytes, provide only 6
        let input = b"P6 2 2 255\n\x01\x02\x03\x04\x05\x06";
        assert!(parse_ppm(input).is_err());

        // 1x1 requires 3 bytes, provide 2
        let input = b"P6 1 1 255\n\x01\x02";
        assert!(parse_ppm(input).is_err());

        // Extra trailing bytes should also error (strictly w*h*3 bytes)
        let input = b"P6 1 1 255\n\x01\x02\x03\x04";
        assert!(parse_ppm(input).is_err());
    }

    #[test]
    fn test_overflow_ppm() {
        // Dimension > 32768
        assert!(parse_ppm(b"P6 32769 1 255\n").is_err());
        assert!(parse_ppm(b"P6 1 32769 255\n").is_err());

        // Pixels > 64,000,000 (30000 x 3000 = 90,000,000)
        assert!(parse_ppm(b"P6 30000 3000 255\n").is_err());
    }

    #[test]
    fn test_regionparse_negativepositions() {
        // Negative origin
        let (x, y, w, h) = parse_slurp_output("-1920,-1080 1920x1080\n").unwrap();
        assert_eq!((x, y, w, h), (-1920, -1080, 1920, 1080));

        let (x, y, w, h) = parse_slurp_output("-50,20 100x200").unwrap();
        assert_eq!((x, y, w, h), (-50, 20, 100, 200));

        let (x, y, w, h) = parse_slurp_output("100,-200 300x400").unwrap();
        assert_eq!((x, y, w, h), (100, -200, 300, 400));

        // Positive origin
        let (x, y, w, h) = parse_slurp_output("0,0 1920x1080").unwrap();
        assert_eq!((x, y, w, h), (0, 0, 1920, 1080));

        // Invalid cases
        assert!(parse_slurp_output("").is_err());
        assert!(parse_slurp_output("-1920,-1080").is_err());
        assert!(parse_slurp_output("-1920,-1080 1920x1080 extra").is_err());
        assert!(parse_slurp_output("0,0 0x100").is_err());
        assert!(parse_slurp_output("0,0 100x0").is_err());
        assert!(parse_slurp_output("0,0 -100x100").is_err());
        assert!(parse_slurp_output("0,0 32769x100").is_err());
        assert!(parse_slurp_output("0,0 30000x3000").is_err());
        assert!(parse_slurp_output("foo,bar 100x100").is_err());
    }

    #[test]
    #[ignore]
    fn test_live_compositor() {
        // Ignored live test against real compositor
        let res_disp = capture_display();
        let _ = res_disp;
    }
}
