//! Windows-only isolated NeMo-Speech.cpp V3 diarization runner.
//! No native runtime is linked into the ONNX-bearing Suflyor processes.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use crate::persistence::DiarSegment;

const MODEL_ID: &str = "nemotron-3-diarization/q8_0@f667ed73aee57d40cc39428eb768b4fd87a0a29e";
const MAX_RTTM_BYTES: u64 = 8 * 1024 * 1024;
const MAX_RUN_TIME: Duration = Duration::from_secs(3 * 60 * 60);

/// Execute the packaged native CLI with a verified local model and convert RTTM
/// into the existing millisecond segment contract. Never replace stored data on failure.
///
/// # Errors
/// Returns an error on missing assets, failure, timeout or malformed model output.
pub fn diarize(
    wav: &Path,
    duration_ms: i64,
    cancel: &AtomicBool,
) -> Result<(Vec<DiarSegment>, i64, String)> {
    let model = crate::diar_install::nemotron_model_path()
        .filter(|_| crate::diar_install::nemotron_installed())
        .context("Nemotron model unavailable")?;
    if !crate::diar_install::nemotron_model_digest_ok(&model)? {
        bail!("Nemotron model digest mismatch; reinstall the model");
    }
    let exe = std::env::current_exe()
        .context("resolve application executable")?
        .with_file_name("nemo-speech.exe");
    if !exe.is_file() {
        bail!("Nemotron runtime unavailable");
    }
    let work = tempfile::tempdir().context("create private diarization workspace")?;
    let rttm = work.path().join("result.rttm");
    let mut cmd = Command::new(exe);
    cmd.args(["diarize"])
        .arg(wav)
        .arg("--model")
        .arg(model)
        .args([
            "--device",
            "cpu",
            "--preset",
            "v3-offline",
            "--format",
            "rttm",
            "--recording-id",
            "suflyor",
            "--output",
        ])
        .arg(&rttm)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = crate::download::no_window(&mut cmd)
        .spawn()
        .context("start Nemotron")?;
    crate::local_ai::assign_to_lifetime_job(&child);
    // Ownership is scoped to this method: a dropped worker cannot leave the CLI
    // behind, and failure never writes to the session's persisted row.
    struct ChildGuard(std::process::Child);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut guard = ChildGuard(child);
    let started = Instant::now();
    loop {
        if cancel.load(Ordering::Acquire) {
            bail!("Nemotron canceled");
        }
        if let Some(status) = guard.0.try_wait().context("wait for Nemotron")? {
            if !status.success() {
                bail!("Nemotron exited unsuccessfully");
            }
            break;
        }
        if started.elapsed() > MAX_RUN_TIME {
            bail!("Nemotron timed out");
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let file = std::fs::File::open(&rttm).context("open Nemotron RTTM")?;
    if file.metadata().context("read RTTM size")?.len() > MAX_RTTM_BYTES {
        bail!("RTTM too large");
    }
    let parsed = parse_rttm(BufReader::new(file), duration_ms)?;
    Ok((parsed.0, parsed.1, MODEL_ID.to_string()))
}

fn parse_rttm(reader: impl BufRead, duration_ms: i64) -> Result<(Vec<DiarSegment>, i64)> {
    if duration_ms <= 0 {
        bail!("invalid audio length");
    }
    let mut raw: Vec<(i64, i64, String)> = Vec::new();
    for line in reader.lines() {
        let line = line.context("read RTTM")?;
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() != 10 || fields[0] != "SPEAKER" || fields[1] != "suflyor" {
            bail!("invalid RTTM record");
        }
        let start: f64 = fields[3].parse().context("RTTM start")?;
        let length: f64 = fields[4].parse().context("RTTM duration")?;
        if !start.is_finite()
            || !length.is_finite()
            || start < 0.0
            || length <= 0.0
            || (start + length) * 1000.0 > duration_ms as f64 + 20.0
            || fields[7].is_empty()
        {
            bail!("invalid RTTM segment");
        }
        let s = (start * 1000.0).round() as i64;
        let e = ((start + length) * 1000.0).round() as i64;
        if s >= e {
            bail!("RTTM segment too short");
        }
        raw.push((s, e, fields[7].to_string()));
    }
    if raw.is_empty() {
        bail!("no speakers detected");
    }
    raw.sort_by_key(|(start, end, _)| (*start, *end));
    let ids: BTreeSet<&str> = raw.iter().map(|(_, _, id)| id.as_str()).collect();
    if ids.len() > 8 {
        bail!("too many Nemotron speakers");
    }
    let mut map: BTreeMap<String, i32> = BTreeMap::new();
    let segments = raw
        .into_iter()
        .map(|(start_ms, end_ms, name)| {
            let next = map.len() as i32;
            let speaker = *map.entry(name).or_insert(next);
            DiarSegment {
                start_ms,
                end_ms,
                speaker,
            }
        })
        .collect();
    Ok((segments, map.len() as i64))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    #[test]
    fn keeps_overlap_and_first_arrival_ids() {
        let r = b"SPEAKER suflyor 1 1.500 1.000 <NA> <NA> speaker_8 <NA> <NA>\nSPEAKER suflyor 1 2.000 1.500 <NA> <NA> speaker_3 <NA> <NA>\n";
        let (segs, count) = parse_rttm(&r[..], 5000).unwrap();
        assert_eq!(count, 2);
        assert_eq!(
            segs[0],
            DiarSegment {
                start_ms: 1500,
                end_ms: 2500,
                speaker: 0
            }
        );
        assert_eq!(
            segs[1],
            DiarSegment {
                start_ms: 2000,
                end_ms: 3500,
                speaker: 1
            }
        );
    }
    #[test]
    fn rejects_nan_bounds_and_unexpected_file() {
        for row in [
            "SPEAKER suflyor 1 NaN 1 <NA> <NA> s1 <NA> <NA>",
            "SPEAKER elsewhere 1 1 1 <NA> <NA> s1 <NA> <NA>",
            "SPEAKER suflyor 1 1 99 <NA> <NA> s1 <NA> <NA>",
        ] {
            assert!(parse_rttm(row.as_bytes(), 5000).is_err());
        }
    }
}
