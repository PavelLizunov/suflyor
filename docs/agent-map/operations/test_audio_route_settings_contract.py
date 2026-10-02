"""Audio route/settings/watchdog source assertions, no device/Rust/COM execution."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[3]


def source(path):return (ROOT/path).read_text()


class AudioRouteSourceFixtures(unittest.TestCase):
    def test_settings_clone_save_commit_and_invalid_missing_selection_guard(self):
        s=source('slint-experiment/src/bin/overlay_host/settings_audio.rs')
        body=s[s.index('fn persist_selection'):s.index('fn model(')]
        self.assertIn('let mut candidate = current.clone()',body)
        self.assertLess(body.index('persist(&candidate)?'),body.index('*current = candidate'))
        self.assertIn('(missing && index == names.len() - 1)',s)
        self.assertIn('win.get_audio_devices_loading() || win.get_audio_devices_failed()',s)
        self.assertIn('current = cfg.read()',s)

    def test_names_duplicate_warning_and_route_default_console_null_loss(self):
        a=source('overlay-backend/src/audio.rs');r=source('overlay-backend/src/audio_route.rs')
        self.assertIn('duplicate {dir:?} endpoint name',a)
        self.assertIn('if fname == name',a)
        self.assertIn('RouteRole::from_windows(role)',r)
        self.assertIn('if endpoint_id.is_null()',r)
        self.assertIn('DeviceSelection::FollowDefault',r)
        self.assertIn('DeviceSelection::Pinned(_)',r)

    def test_windows_capture_drop_not_join_and_recovery_unbounded_fixed_sleep(self):
        s=source('overlay-backend/src/audio.rs')
        drop=s[s.index('impl Drop for CaptureHandle'):s.index('pub fn start_capture')]
        self.assertIn('self.stop.store(true',drop);self.assertNotIn('.join()',drop)
        recover=s[s.index('fn capture_with_recovery'):s.index('enum CaptureExit')]
        self.assertIn('while !stop.load(Ordering::Acquire)',recover)
        self.assertIn('recovery_attempt.saturating_add(1)',recover)
        self.assertIn('thread::sleep(Duration::from_secs(1))',recover)
        self.assertNotIn('MAX_ATTEMPTS',recover)

    def test_mac_watchdog_never_flowed_not_stop_flowed_stall_and_one_shot(self):
        s=source('slint-experiment/src/bin/overlay_host/capture_watchdog.rs')
        self.assertIn('STAGNANT_TICKS: u32 = 5',s)
        self.assertIn('if !self.expected',s)
        self.assertIn('if !session_intended',s)
        self.assertIn('if self.stop_requested',s)
        self.assertIn('self.stop_requested = true',s)
        self.assertIn('fn never_flowed_streams_do_not_stop()',s)

    def test_mac_counter_only_successful_enqueue_full_queue_not_emission(self):
        s=source('overlay-backend/src/audio_macos.rs')
        for name in ('Mic','System'):
            self.assertIn(f'metrics.record_emitted_chunk(Stream::{name}, timestamp_ms)',s)
            self.assertIn(f'metrics.record_queue_drop(Stream::{name})',s)
        metrics=source('overlay-backend/src/audio_metrics.rs')
        self.assertIn('self.with_state(',metrics)
        self.assertIn('s.emitted_chunks.saturating_add(1)',metrics)
        self.assertIn('max_pending_samples',metrics)

    def test_watchdog_host_stop_generation_lifecycle_guard_not_restart(self):
        s=source('slint-experiment/src/bin/overlay_host_windows.rs')
        body=s[s.index('let _capture_watchdog_timer ='):s.index('// ===== Spawn-tile poll Timer')]
        self.assertIn('metrics.mic.emitted_chunks, metrics.system.emitted_chunks',body)
        self.assertLess(body.index('lifecycle.lock().await'),body.index('generation.load(Ordering::Acquire) == stop_intent'))
        self.assertLess(body.index('generation.load(Ordering::Acquire) == stop_intent'),body.index('stop_session_and_maybe_debrief('))
        self.assertNotIn('slint_session::start_session',body)
        self.assertIn('TileKind::Error',body)


if __name__=='__main__':unittest.main()
