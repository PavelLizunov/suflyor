//! Local OCR via a bundled **Tesseract child process**.
//!
//! This replaces the small vision-LLM on the read-aloud OCR path (Ctrl+F8 /
//! Shift+Alt+2). The 4B VLM loops and hallucinates on dense text; Tesseract is
//! deterministic and never invents words. It runs as a CHILD PROCESS — NOT an
//! FFI crate, and NOT onnxruntime/RapidOCR: a 2nd static onnxruntime in this
//! process would crash against the app's `ort`/GigaAM STT (the same reason the
//! neural TTS lives in its own `suflyor-tts.exe` sidecar — see [`crate::tts`]).
//!
//! Layout (mirrors the `tts/` voices dir):
//! `<root>\tesseract\tesseract.exe` + its sibling DLLs + `tessdata\{rus,eng}.traineddata`,
//! where `<root>` is the directory next to the running exe OR `%APPDATA%\suflyor`.
//! Windows resolves tesseract's sibling DLLs from the EXE's own directory
//! automatically, so spawning it by full path is enough — no PATH/cwd juggling.
//!
//! The captured screen region is fed to tesseract over **stdin** and the text
//! read back over **stdout**, so the user's screen pixels never touch disk
//! (privacy — the OCR/read-aloud region is the user's choice and stays local).
//!
//! TRUST BOUNDARY: `%APPDATA%\suflyor\tesseract` is user-writable, and a child
//! exe resolves its non-system DLL imports from its OWN directory first — so
//! whoever can write that directory can run code in this process's context
//! (which holds config.json's live keys + screen pixels). The integrity of the
//! engine is therefore established at INSTALL time: the download-on-first-use
//! step MUST fetch over HTTPS and verify the archive + extracted exe / DLLs /
//! `*.traineddata` against SHA-256 hashes pinned in the binary BEFORE the first
//! spawn (the same verify-before-execute bar as `update.rs` / the local-AI
//! installer — tasks #137/#178), then the populated directory is trusted for
//! repeated spawns (mirrors how the llama.cpp engine is handled). The
//! `<exe_dir>\tesseract` candidate (admin-protected under Program Files) is
//! preferred over the `%APPDATA%` one for exactly this reason.

use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// Default recognition languages. `rus+eng` so Latin terms / code embedded in
/// Russian text (the user is a developer) are recognised too. Paired with the
/// LSTM engine (`--oem 1`) and a single uniform text block (`--psm 6`), which
/// suits a user-selected region of text.
pub const DEFAULT_OCR_LANG: &str = "rus+eng";

/// Resolve the directory holding `tesseract.exe`. Checks, in order:
/// 1. `<exe_dir>\tesseract\` — release: bundled / downloaded next to overlay-host;
/// 2. `%APPDATA%\suflyor\tesseract\` — dev + download-on-first-use target.
///
/// Returns the first directory that actually contains `tesseract.exe`.
#[must_use]
pub fn tesseract_root() -> Option<PathBuf> {
    // 1. Next to the running executable.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let cand = dir.join("tesseract");
            if cand.join(EXE_NAME).is_file() {
                return Some(cand);
            }
        }
    }
    // 2. The app data dir.
    if let Some(d) = crate::paths::data_root() {
        let cand = d.join("tesseract");
        if cand.join(EXE_NAME).is_file() {
            return Some(cand);
        }
    }
    None
}

#[cfg(windows)]
const EXE_NAME: &str = "tesseract.exe";
#[cfg(not(windows))]
const EXE_NAME: &str = "tesseract";

/// Full path to the tesseract executable, if installed.
#[must_use]
pub fn tesseract_exe() -> Option<PathBuf> {
    tesseract_root().map(|d| d.join(EXE_NAME))
}

/// Whether the local OCR engine is installed. Drives the caller's decision to
/// use Tesseract vs. fall back to the vision-LLM.
#[must_use]
pub fn is_available() -> bool {
    tesseract_exe().is_some()
}

/// OCR a top-down BGRA frame (as produced by the screen-capture crop) and
/// return the recognised UTF-8 text.
///
/// All failures (engine missing, spawn/IO error, non-zero exit) come back as an
/// `Err` for the caller to surface as a GENERIC tile message — never the raw
/// chain (it could embed local paths). An empty recognition is `Ok("")`.
pub fn run_ocr(bgra: &[u8], width: u32, height: u32, lang: &str) -> Result<String> {
    let root = tesseract_root().ok_or_else(|| anyhow!("OCR engine not installed"))?;
    let exe = root.join(EXE_NAME);
    let tessdata = root.join("tessdata");

    // Guard the buffer so the BMP encoder can never read out of bounds on a
    // short / mismatched slice.
    let need = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| anyhow!("image dimensions overflow"))?;
    if width == 0 || height == 0 || bgra.len() < need {
        bail!("empty or undersized image buffer");
    }

    let lang = if lang.trim().is_empty() {
        DEFAULT_OCR_LANG
    } else {
        lang
    };
    let bmp = bgra_to_bmp(bgra, width, height);

    let mut child = spawn_tesseract(&exe, &tessdata, lang).context("spawn tesseract")?;
    // Feed the image on a writer thread so a full stdout pipe can drain
    // concurrently — the classic child-pipe deadlock avoidance.
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("tesseract stdin unavailable"))?;
    let writer = std::thread::spawn(move || {
        use std::io::Write;
        // A broken pipe (tesseract died early) is not fatal here — the exit
        // status below is the source of truth.
        let _ = stdin.write_all(&bmp);
        // Dropping `stdin` closes the pipe so tesseract sees EOF.
    });
    let out = child.wait_with_output().context("tesseract wait")?;
    let _ = writer.join();

    if !out.status.success() {
        bail!("tesseract exited with status {}", out.status);
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(normalize_ocr_text(&text))
}

/// Spawn tesseract reading the image from stdin and writing text to stdout.
/// `--oem 1` = LSTM engine (matches the `tessdata_fast` models), `--psm 6` =
/// assume a single uniform block of text (a selected region).
#[cfg(windows)]
fn spawn_tesseract(exe: &Path, tessdata: &Path, lang: &str) -> std::io::Result<Child> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    Command::new(exe)
        .args(["stdin", "stdout", "-l", lang, "--oem", "1", "--psm", "6"])
        .env("TESSDATA_PREFIX", tessdata)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
}

#[cfg(not(windows))]
fn spawn_tesseract(exe: &Path, tessdata: &Path, lang: &str) -> std::io::Result<Child> {
    Command::new(exe)
        .args(["stdin", "stdout", "-l", lang, "--oem", "1", "--psm", "6"])
        .env("TESSDATA_PREFIX", tessdata)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
}

/// Tidy tesseract output for text-to-speech: drop the per-page form-feed it
/// appends, strip trailing whitespace per line, and trim leading/trailing blank
/// lines. Line order + interior blank lines are preserved.
fn normalize_ocr_text(s: &str) -> String {
    s.replace('\u{000C}', "") // page form-feed (\f)
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Encode a top-down BGRA buffer as a 24-bit BGR Windows BMP (negative height =
/// top-down). Lossless — no JPEG artifacts to confuse the recogniser — and
/// needs no image-codec dependency (BMP from BGRA is essentially a copy).
fn bgra_to_bmp(bgra: &[u8], width: u32, height: u32) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    // Each output row is `w*3` bytes padded up to a 4-byte boundary.
    let row_bytes = (w * 3 + 3) & !3;
    let pad = row_bytes - w * 3;
    let img_size = row_bytes * h;
    let file_size = 54 + img_size;

    let mut out = Vec::with_capacity(file_size);
    // BITMAPFILEHEADER (14 bytes)
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(file_size as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // reserved
    out.extend_from_slice(&54u32.to_le_bytes()); // pixel data offset
                                                 // BITMAPINFOHEADER (40 bytes)
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(width as i32).to_le_bytes());
    out.extend_from_slice(&(-(height as i32)).to_le_bytes()); // negative = top-down
    out.extend_from_slice(&1u16.to_le_bytes()); // planes
    out.extend_from_slice(&24u16.to_le_bytes()); // bits per pixel
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB (no compression)
    out.extend_from_slice(&(img_size as u32).to_le_bytes());
    out.extend_from_slice(&2835i32.to_le_bytes()); // 72 DPI x (px/m)
    out.extend_from_slice(&2835i32.to_le_bytes()); // 72 DPI y
    out.extend_from_slice(&0u32.to_le_bytes()); // palette colors
    out.extend_from_slice(&0u32.to_le_bytes()); // important colors

    // Pixel rows, top-down, BGR (drop alpha) + row padding.
    for y in 0..h {
        let row = y * w * 4;
        for x in 0..w {
            let p = row + x * 4;
            // Bounds were guaranteed by the caller's length check, but index
            // safely anyway so a future caller can't trip UB.
            if let Some(px) = bgra.get(p..p + 3) {
                out.push(px[0]); // B
                out.push(px[1]); // G
                out.push(px[2]); // R
            } else {
                out.extend_from_slice(&[0, 0, 0]);
            }
        }
        out.resize(out.len() + pad, 0u8);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bmp_header_is_well_formed_and_top_down() {
        // 2x2 BGRA (top-down): the encoder must emit a 24-bit BI_RGB BMP with a
        // NEGATIVE height (top-down) and the right total size.
        let bgra = vec![
            10, 20, 30, 255, 11, 21, 31, 255, // row 0
            12, 22, 32, 255, 13, 23, 33, 255, // row 1
        ];
        let bmp = bgra_to_bmp(&bgra, 2, 2);
        assert_eq!(&bmp[0..2], b"BM", "magic");
        // row = 2*3=6 bytes → padded to 8; img = 8*2 = 16; file = 54+16 = 70.
        let file_size = u32::from_le_bytes([bmp[2], bmp[3], bmp[4], bmp[5]]);
        assert_eq!(file_size as usize, bmp.len());
        assert_eq!(file_size, 70);
        // biHeight (offset 22) must be negative (top-down).
        let h = i32::from_le_bytes([bmp[22], bmp[23], bmp[24], bmp[25]]);
        assert_eq!(h, -2);
        // bpp (offset 28) = 24.
        let bpp = u16::from_le_bytes([bmp[28], bmp[29]]);
        assert_eq!(bpp, 24);
        // First pixel bytes (offset 54) are the BGR of pixel (0,0) = 10,20,30.
        assert_eq!(&bmp[54..57], &[10, 20, 30]);
    }

    #[test]
    fn normalize_strips_formfeed_trailing_ws_and_blank_edges() {
        let raw = "\n\nПривет  \nмир\t\n\u{000C}\n";
        assert_eq!(normalize_ocr_text(raw), "Привет\nмир");
    }

    #[test]
    fn normalize_keeps_interior_blank_lines_and_order() {
        let raw = "один\n\nдва\n";
        assert_eq!(normalize_ocr_text(raw), "один\n\nдва");
    }

    #[test]
    fn run_ocr_rejects_empty_or_short_buffer() {
        // Zero dims and a too-short buffer must error BEFORE any spawn.
        assert!(run_ocr(&[], 0, 0, "rus").is_err());
        assert!(run_ocr(&[0, 0, 0, 0], 4, 4, "rus").is_err()); // needs 64 bytes
    }

    #[test]
    fn default_lang_is_rus_plus_eng() {
        assert_eq!(DEFAULT_OCR_LANG, "rus+eng");
    }
}
