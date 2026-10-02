"""WSOLA/playback source contracts only, no Rust DSP/listening/allocation benchmark."""
import unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[3]


def source(path):return (ROOT/path).read_text()


class WsolaPlaybackSourceFixtures(unittest.TestCase):
    def test_stream_inverse_speed_sample_rate_time_and_tail_once(self):
        s=source('suflyor-wsola/src/stream.rs')
        self.assertIn('let ratio = 1.0 / f64::from(speed)',s)
        self.assertIn('sample_rate',s)
        self.assertIn('if fresh.is_empty()',s)
        self.assertIn('std::mem::take(&mut self.held_output_tail)',s)
        self.assertIn('self.wsola.process(&input)?',s)
        self.assertIn('input.resize(self.wsola.segment_size(), 0.0)',s)

    def test_no_grow_output_flag_does_not_bound_fft_workspace(self):
        s=source('suflyor-wsola/src/wsola.rs')
        no_grow=s[s.index('pub fn process_into_no_grow'):s.index('fn process_into_internal')]
        self.assertIn('false, false',no_grow)
        correlation=s[s.index('fn find_best_position'):s.index('fn overlap_for_ratio')]
        self.assertIn('ensure_fft_plan(fft_size)',correlation)
        self.assertIn('corr_values_buf.resize(',correlation)
        self.assertIn('fft_fwd_scratch.resize(',correlation)
        self.assertIn('FFT_CANDIDATE_THRESHOLD',correlation)
        self.assertNotIn('allow_internal_growth',correlation)

    def test_all_transports_rewind_reset_stretcher_error_fallback(self):
        for path in ('suflyor-tts/src/playback.rs','suflyor-tts/src/playback_macos.rs','suflyor-teratts/src/playback.rs','suflyor-teratts/src/playback_macos.rs'):
            with self.subTest(path=path):
                s=source(path)
                self.assertIn('timeline.rewind_to(checkpoint)',s)
                self.assertIn('*stretcher = None',s)
                self.assertIn('active.finish()',s)
                self.assertIn('Err(_) => output.extend(fresh)',s)
                self.assertIn('if available < STRETCH_INPUT_CHUNK && !eos',s)
                self.assertIn('prune_played_history(sample_rate)',s)

    def test_transcript_separate_adapter_error_empty_and_fixed_tail(self):
        s=source('slint-experiment/src/bin/overlay_host/transcript_player.rs')
        self.assertIn('const XFADE_OUT: usize = 256',s)
        self.assertIn('self.wsola.process(&inbuf).unwrap_or_default()',s)
        self.assertIn('if (self.speed - 1.0).abs() < f32::EPSILON',s)
        self.assertIn('let cur = self.position_sample()',s)
        self.assertIn('self.load_from(cur)',s)
        self.assertNotIn('StreamingWsola::',s)

    def test_rust_test_identity_is_repeat_determinism_not_input_bit_identity(self):
        s=source('suflyor-wsola/tests/speech_contract.rs')
        self.assertIn('assert_eq!(output.len(), input.len())',s)
        self.assertIn('assert_eq!(output, second.process(&input).unwrap())',s)
        self.assertNotIn('assert_eq!(output, input)',s)
        self.assertIn('SAMPLE_RATE as f64 / 25.0',s)
        self.assertIn('sample.is_finite()',s)

    def test_mac_callback_uses_mutex_and_prunes_fed_not_audible_cursor(self):
        s=source('suflyor-teratts/src/playback_macos.rs')
        self.assertIn('pcm_queue_callback',s)
        self.assertIn('.lock()',s)
        self.assertIn('guard.pop_front().unwrap_or(0.0)',s)
        self.assertIn('timeline.prune_played_history(sample_rate)',s)
        t=source('suflyor-tts/src/playback.rs')
        self.assertIn('std::sync::mpsc::channel::<Vec<f32>>()',t)
        self.assertIn('self.cursor.saturating_sub(self.start).saturating_sub(keep)',t)
        self.assertIn('buf.push(s);',t)


if __name__=='__main__':unittest.main()
