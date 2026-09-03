use super::retention::{prune_old_sessions_with_size_cap, KEEP_LAST_SESSIONS, MAX_TOTAL_BYTES};
use super::time::{chrono_like_stamp, now_unix_ms};
use super::types::{JournalEvent, SessionCounters};
use anyhow::{Context, Result};
use parking_lot::Mutex;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;

pub fn sessions_dir() -> Result<PathBuf> {
    let root = crate::paths::data_root().context("no config dir")?;
    Ok(root.join("sessions"))
}

#[derive(Clone, Default)]
pub struct Journal {
    pub(crate) path: Option<Arc<PathBuf>>,
    pub(crate) counters: Option<Arc<Mutex<SessionCounters>>>,
    pub(crate) writer: Option<Arc<Mutex<WriterState>>>,
}

pub(crate) enum WriterCmd {
    Line(String),
    Shutdown(std::sync::mpsc::Sender<Result<(), String>>),
}

pub(crate) struct WriterState {
    pub(crate) tx: Option<mpsc::UnboundedSender<WriterCmd>>,
    pub(crate) join: Option<std::thread::JoinHandle<()>>,
    pub(crate) shutdown: Option<ShutdownState>,
}

#[derive(Clone)]
pub(crate) enum ShutdownState {
    InProgress,
    Done(Result<(), String>),
}

impl Journal {
    #[cfg(test)]
    pub(crate) fn counting_for_test() -> Self {
        Self {
            path: None,
            counters: Some(Arc::new(Mutex::new(SessionCounters::default()))),
            writer: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn capturing_for_test() -> (Self, mpsc::UnboundedReceiver<WriterCmd>) {
        let (tx, rx) = mpsc::unbounded_channel::<WriterCmd>();
        let journal = Self {
            path: None,
            counters: Some(Arc::new(Mutex::new(SessionCounters::default()))),
            writer: Some(Arc::new(Mutex::new(WriterState {
                tx: Some(tx),
                join: None,
                shutdown: None,
            }))),
        };
        (journal, rx)
    }

    pub fn open_new_session() -> Result<Self> {
        Self::open_new_session_with_limits(KEEP_LAST_SESSIONS, MAX_TOTAL_BYTES)
    }

    pub fn open_new_session_with_limits(keep_sessions: usize, max_bytes: u64) -> Result<Self> {
        let dir = sessions_dir()?;
        std::fs::create_dir_all(&dir).context("create sessions dir")?;
        let stamp = chrono_like_stamp();
        let rand: u32 = (now_unix_ms() & 0xFFFFFF) as u32;
        let path = dir.join(format!("{stamp}_{rand:06x}.jsonl"));

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .context("open journal file")?;
        log::info!("journal opened: {}", path.display());

        let keep = if keep_sessions == 0 {
            usize::MAX
        } else {
            keep_sessions
        };
        match prune_old_sessions_with_size_cap(&dir, keep, max_bytes) {
            Ok(n) if n > 0 => log::info!("journal pruned {n} old session(s)"),
            Ok(_) => {}
            Err(e) => log::warn!("journal prune failed (non-fatal): {e:#}"),
        }

        let (tx, rx) = mpsc::unbounded_channel::<WriterCmd>();
        let join = spawn_writer(rx, file).context("spawn journal writer thread")?;

        let counters = Arc::new(Mutex::new(SessionCounters {
            start_unix_ms: now_unix_ms(),
            ..Default::default()
        }));

        Ok(Self {
            path: Some(Arc::new(path)),
            counters: Some(counters),
            writer: Some(Arc::new(Mutex::new(WriterState {
                tx: Some(tx),
                join: Some(join),
                shutdown: None,
            }))),
        })
    }

    pub fn write(&self, event: &JournalEvent<'_>) {
        if let Some(c) = &self.counters {
            bump_counters(&mut c.lock(), event);
        }
        let Some(writer) = &self.writer else { return };
        let line = match serde_json::to_string(event) {
            Ok(line) => line,
            Err(e) => {
                log::warn!("journal serialize failed: {e}");
                return;
            }
        };
        let state = writer.lock();
        if let Some(tx) = &state.tx {
            let _ = tx.send(WriterCmd::Line(line));
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref().map(PathBuf::as_path)
    }

    pub fn session_id(&self) -> Option<String> {
        self.path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .map(str::to_string)
    }

    pub fn close(&self) {
        let _ = self.shutdown_blocking(std::time::Duration::from_secs(2));
    }

    pub fn shutdown_blocking(&self, timeout: std::time::Duration) -> Result<(), String> {
        let Some(writer) = &self.writer else {
            return Ok(());
        };
        let (ack_tx, ack_rx) = std::sync::mpsc::channel();
        let (tx, join) = {
            let mut state = writer.lock();
            if let Some(ShutdownState::Done(result)) = &state.shutdown {
                return result.clone();
            }
            if state.shutdown.is_some() {
                return Err("journal shutdown already in progress".into());
            }
            state.shutdown = Some(ShutdownState::InProgress);
            let tx = state.tx.take();
            let join = state.join.take();
            (tx, join)
        };
        let Some(tx) = tx else {
            return Ok(());
        };
        if tx.send(WriterCmd::Shutdown(ack_tx)).is_err() {
            let mut state = writer.lock();
            state.shutdown = Some(ShutdownState::Done(Ok(())));
            return Ok(());
        }
        let outcome = ack_rx
            .recv_timeout(timeout)
            .map_err(|e| format!("journal shutdown wait failed: {e}"))
            .and_then(|result| result);
        if let Some(join) = join {
            let _ = join.join();
        }
        let mut state = writer.lock();
        state.shutdown = Some(ShutdownState::Done(outcome.clone()));
        outcome
    }

    pub fn emit_summary_and_stop(&self) {
        if let Some(c) = &self.counters {
            let c = c.lock().clone();
            let now = now_unix_ms();
            let duration_ms = if c.start_unix_ms > 0 && now >= c.start_unix_ms {
                now - c.start_unix_ms
            } else {
                0
            };
            self.write(&JournalEvent::SessionSummary {
                unix_ms: now,
                duration_ms,
                transcript_lines: c
                    .transcript_mic
                    .saturating_add(c.transcript_system),
                transcript_mic: c.transcript_mic,
                transcript_system: c.transcript_system,
                detector_triggered: c.detector_triggered,
                detector_skipped: c.detector_skipped,
                ai_requests_total: c.ai_requests_total,
                ai_responses_ok: c.ai_responses_ok,
                ai_errors: c.ai_errors,
                tiles_spawned: c.tiles_spawned,
                rate_limited: c.rate_limited,
                total_cost_microcents: c.total_cost_microcents,
            });
        }
        self.write(&JournalEvent::SessionStop {
            unix_ms: now_unix_ms(),
        });
        self.close();
    }
}

pub(crate) fn note_write_error(first_error: &mut Option<String>, e: std::io::Error) {
    log::warn!("journal write failed (continuing): {e}");
    if first_error.is_none() {
        *first_error = Some(e.to_string());
    }
}

pub(crate) fn finish_writer(
    file: &mut BufWriter<std::fs::File>,
    first_error: Option<String>,
    ack: std::sync::mpsc::Sender<Result<(), String>>,
) {
    let flush_result = file
        .flush()
        .map_err(|e| format!("journal final flush failed: {e}"));
    let outcome = match first_error {
        Some(e) => Err(format!("earlier journal write failed: {e}")),
        None => flush_result,
    };
    let _ = ack.send(outcome);
    log::debug!("journal writer thread exit (shutdown)");
}

pub(crate) fn spawn_writer(
    mut rx: mpsc::UnboundedReceiver<WriterCmd>,
    file: std::fs::File,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("journal-writer".into())
        .spawn(move || {
            let mut file = BufWriter::new(file);
            let mut first_error: Option<String> = None;

            loop {
                match rx.blocking_recv() {
                    Some(WriterCmd::Line(line)) => {
                        if let Err(e) = writeln!(file, "{line}") {
                            note_write_error(&mut first_error, e);
                        }
                        loop {
                            match rx.try_recv() {
                                Ok(WriterCmd::Line(line)) => {
                                    if let Err(e) = writeln!(file, "{line}") {
                                        note_write_error(&mut first_error, e);
                                    }
                                }
                                Ok(WriterCmd::Shutdown(ack)) => {
                                    finish_writer(&mut file, first_error, ack);
                                    return;
                                }
                                Err(_) => break,
                            }
                        }
                        if let Err(e) = file.flush() {
                            note_write_error(&mut first_error, e);
                        }
                    }
                    Some(WriterCmd::Shutdown(ack)) => {
                        finish_writer(&mut file, first_error, ack);
                        return;
                    }
                    None => {
                        let _ = file.flush();
                        log::debug!("journal writer thread exit (channel closed)");
                        return;
                    }
                }
            }
        })
}

pub(crate) fn bump_counters(c: &mut SessionCounters, event: &JournalEvent<'_>) {
    match event {
        JournalEvent::TranscriptLine { source, .. } => {
            if *source == "mic" {
                c.transcript_mic = c.transcript_mic.saturating_add(1);
            } else if *source == "system" {
                c.transcript_system = c.transcript_system.saturating_add(1);
            }
        }
        JournalEvent::DetectorDecision { triggered, .. } => {
            if *triggered {
                c.detector_triggered = c.detector_triggered.saturating_add(1);
            } else {
                c.detector_skipped = c.detector_skipped.saturating_add(1);
            }
        }
        JournalEvent::AiRequest { .. } => {
            c.ai_requests_total = c.ai_requests_total.saturating_add(1);
        }
        JournalEvent::AiResponse {
            cost_microcents, ..
        } => {
            c.ai_responses_ok = c.ai_responses_ok.saturating_add(1);
            c.total_cost_microcents = c.total_cost_microcents.saturating_add(*cost_microcents);
        }
        JournalEvent::TileSpawn { .. } => {
            c.tiles_spawned = c.tiles_spawned.saturating_add(1);
        }
        JournalEvent::RateLimited { .. } => {
            c.rate_limited = c.rate_limited.saturating_add(1);
        }
        JournalEvent::Error { .. } => {
            c.ai_errors = c.ai_errors.saturating_add(1);
        }
        JournalEvent::SessionStart { .. }
        | JournalEvent::SessionStop { .. }
        | JournalEvent::SessionSummary { .. } => {}
    }
}
