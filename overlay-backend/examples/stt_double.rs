//! Reproduce the app's crash trigger: load GigaAM (ort/DirectML), DROP it, load
//! it AGAIN — while sherpa-onnx is LINKED into this binary (overlay-backend pulls
//! it in) but never called. The app does exactly this (warm load + session load)
//! and dies natively on the second. The single-load spike survives.
//!
//!   CARGO_TARGET_DIR=slint-experiment/target \
//!     cargo run --manifest-path overlay-backend/Cargo.toml --example stt_double

use std::path::Path;

const GIGAAM_DIR: &str = r"C:\Users\x3d_mutant\suflyor-local-ai\gigaam-v3";

fn load_once(tag: &str) {
    overlay_backend::stt::configure_gigaam_accelerator(true); // DirectML, like the app
    match transcribe_rs::onnx::gigaam::GigaAMModel::load(
        Path::new(GIGAAM_DIR),
        &transcribe_rs::onnx::Quantization::Int8,
    ) {
        Ok(_m) => eprintln!("[double] {tag}: GigaAM loaded OK"),
        Err(e) => eprintln!("[double] {tag}: load Err (not a crash): {e}"),
    }
    // _m dropped here → onnxruntime session torn down
}

fn main() {
    eprintln!("[double] === load #1 (warm) ===");
    load_once("load#1");
    eprintln!("[double] === load #2 (session) — the app's crash point ===");
    load_once("load#2");
    eprintln!("[double] SURVIVED both loads (crash NOT reproduced here)");
}
