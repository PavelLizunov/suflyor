//! v0.17.2 — сквозной (E2E) backend-тест цикла архива, явно запрошенный
//! тестером (P0.1: «звонок → запись → транскрибация → сводка → архив»).
//!
//! Покрывает РЕАЛЬНУЮ цепочку на диске: рекордер пишет настоящие WAV из
//! синтетических аудио-чанков → WAV читаются/валидируются тем же кодом, что
//! и офлайн ре-STT → транскрипт собирается из канальных текстов (сам
//! STT-движок подменён готовым текстом — единственное звено, требующее
//! модели) → журнал сессии проецируется индексером в SQLite-каталог → архив
//! отвечает теми же запросами, которыми его рисует F7-окно.
//!
//! Плюс регресс на корневую причину «0 и 0»: сессия, завершённая в ТЕКУЩЕМ
//! запуске, пропускалась launch-реиндексом (skip_active) и не появлялась в
//! каталоге до перезапуска; точечный `index_journal_file` на stop_session
//! обязан делать её видимой и заменять строку wholesale.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use overlay_backend::audio::{AudioChunk, AudioSource};
use overlay_backend::persistence::{index_all, index_journal_file, Store};
use overlay_backend::re_transcribe::{assemble_lines, load_wav_pcm};
use overlay_backend::recorder::SessionRecorder;
use std::io::Write;
use std::path::{Path, PathBuf};

fn write_journal(dir: &Path, id: &str, lines: &[&str]) -> PathBuf {
    let p = dir.join(format!("{id}.jsonl"));
    let mut f = std::fs::File::create(&p).unwrap();
    for l in lines {
        writeln!(f, "{l}").unwrap();
    }
    p
}

#[test]
fn full_cycle_record_transcribe_journal_index_archive() {
    let tmp = tempfile::tempdir().unwrap();
    let id = "2026-06-11_10-00-00_e2e1";

    // ── 1) ЗАПИСЬ: рекордер пишет настоящие mic.wav / system.wav ──
    let rec_dir = tmp.path().join("recordings").join(id);
    {
        let rec = SessionRecorder::start_in(rec_dir.clone()).unwrap();
        let tone: Vec<i16> = (0..16_000).map(|i| ((i % 100) * 300) as i16).collect();
        rec.feed(&AudioChunk {
            source: AudioSource::Mic,
            pcm_i16: tone.clone(),
            timestamp_ms: 0,
        });
        rec.feed(&AudioChunk {
            source: AudioSource::System,
            pcm_i16: tone,
            timestamp_ms: 10,
        });
        assert_eq!(rec.dropped_chunks(), 0);
    } // drop → WAV-заголовки финализированы

    // ── 2) РЕ-ТРАНСКРИБАЦИЯ: тот же загрузчик/валидатор, что у офлайн ре-STT ──
    let mic_pcm = load_wav_pcm(&rec_dir.join("mic.wav")).unwrap();
    assert_eq!(
        mic_pcm.len(),
        16_000,
        "записанные сэмплы читаются без потерь"
    );
    assert!(rec_dir.join("system.wav").is_file());
    // STT-движок подменён готовым текстом; сборка транскрипта — боевая.
    let lines = assemble_lines("привет, это сквозной тест", "ответ собеседника");
    assert_eq!(lines.len(), 2, "оба канала попали в транскрипт");

    // ── 3) ЖУРНАЛ → 4) ИНДЕКС: проекция в каталог (как на stop_session) ──
    let sess_dir = tmp.path().join("sessions");
    std::fs::create_dir_all(&sess_dir).unwrap();
    let jpath = write_journal(
        &sess_dir,
        id,
        &[
            r#"{"kind":"session_start","unix_ms":1779580800000,"ai_model":"gemma"}"#,
            r#"{"kind":"transcript_line","unix_ms":1779580801000,"source":"mic","text":"привет, это сквозной тест"}"#,
            r#"{"kind":"transcript_line","unix_ms":1779580802000,"source":"system","text":"ответ собеседника"}"#,
            r#"{"kind":"ai_request","unix_ms":1779580803000,"purpose":"ask","model":"gemma","user_prompt":"о чём разговор?"}"#,
            r#"{"kind":"ai_response","unix_ms":1779580804000,"text":"о сквозном тесте","cost_microcents":0}"#,
            r#"{"kind":"session_stop","unix_ms":1779580805000}"#,
        ],
    );
    let mut store = Store::open_in_memory().unwrap();
    let sess = index_journal_file(&mut store, &jpath).unwrap();
    assert_eq!(sess.status, "completed");
    assert_eq!(sess.transcript_lines, 2, "счётчик строк НЕ «0»");
    assert_eq!(sess.ai_turns_count, 1, "счётчик AI НЕ «0»");
    assert_eq!(sess.started_at_ms, Some(1_779_580_800_000));

    // ── 5) АРХИВ: ровно те запросы, которыми F7-окно рисует список/тайл ──
    let listed = store.list_sessions().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, id);
    let utts = store.session_utterances(id).unwrap();
    assert_eq!(utts.len(), 2);
    assert!(utts.iter().any(|u| u.text.contains("привет")));
    let turns = store.session_ai_turns(id).unwrap();
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].answer, "о сквозном тесте");
}

#[test]
fn session_finished_in_current_run_becomes_visible_after_stop_index() {
    // Корневая причина «0 и 0»: launch-реиндекс пропускает ЖИВУЮ сессию
    // (skip_active) — и до v0.17.2 её никто больше не индексировал.
    let tmp = tempfile::tempdir().unwrap();
    let id = "2026-06-11_11-00-00_e2e2";
    let jpath = write_journal(
        tmp.path(),
        id,
        &[
            r#"{"kind":"session_start","unix_ms":1000}"#,
            r#"{"kind":"transcript_line","unix_ms":2000,"source":"mic","text":"строка"}"#,
            r#"{"kind":"session_stop","unix_ms":3000}"#,
        ],
    );
    let mut store = Store::open_in_memory().unwrap();

    // Имитация launch-реиндекса, когда сессия ещё активна → пропуск.
    let stats = index_all(&mut store, tmp.path(), Some(id)).unwrap();
    assert_eq!(stats.indexed, 0);
    assert_eq!(stats.skipped, 1);
    assert!(
        store.list_sessions().unwrap().is_empty(),
        "до фикса архив пуст"
    );

    // Точечная индексация на stop_session → сессия видна БЕЗ перезапуска.
    index_journal_file(&mut store, &jpath).unwrap();
    let listed = store.list_sessions().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].status, "completed");
}

#[test]
fn stop_index_replaces_partial_row_wholesale() {
    // Архив, открытый ВО ВРЕМЯ звонка (или краш до stop), мог положить в
    // каталог НЕПОЛНУЮ строку. Повторный index_journal_file того же файла
    // обязан заменить её целиком (актуальные счётчики, статус), не дублируя.
    let tmp = tempfile::tempdir().unwrap();
    let id = "2026-06-11_12-00-00_e2e3";
    let jpath = write_journal(
        tmp.path(),
        id,
        &[
            r#"{"kind":"session_start","unix_ms":1000}"#,
            r#"{"kind":"transcript_line","unix_ms":2000,"source":"mic","text":"первая"}"#,
        ],
    );
    let mut store = Store::open_in_memory().unwrap();
    let partial = index_journal_file(&mut store, &jpath).unwrap();
    assert_eq!(partial.status, "crashed", "без session_stop строка «сырая»");
    assert_eq!(partial.transcript_lines, 1);

    // Сессия дописалась и закрылась — переиндексация той же самой.
    let jpath2 = write_journal(
        tmp.path(),
        id,
        &[
            r#"{"kind":"session_start","unix_ms":1000}"#,
            r#"{"kind":"transcript_line","unix_ms":2000,"source":"mic","text":"первая"}"#,
            r#"{"kind":"transcript_line","unix_ms":2500,"source":"system","text":"вторая"}"#,
            r#"{"kind":"session_stop","unix_ms":3000}"#,
        ],
    );
    let fixed = index_journal_file(&mut store, &jpath2).unwrap();
    assert_eq!(fixed.status, "completed");
    assert_eq!(fixed.transcript_lines, 2);
    let listed = store.list_sessions().unwrap();
    assert_eq!(listed.len(), 1, "замена wholesale — без дублей");
}

#[test]
fn missing_recordings_dir_is_a_graceful_error_for_re_stt() {
    // «Не удалось транскрибировать»: канал без записи должен давать ошибку
    // на этапе чтения WAV, а не панику. (Полный transcribe_session требует
    // STT-движок; здесь фиксируем контракт нижнего слоя.)
    let tmp = tempfile::tempdir().unwrap();
    let err = load_wav_pcm(&tmp.path().join("нет-такой-папки").join("mic.wav"));
    assert!(err.is_err(), "отсутствующий WAV → Err, не panic");
}
